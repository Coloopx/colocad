//! Lazy exact inspection regions propagated along the feature DAG.
//! They are constructed only for ancestors of an inspection_region request.
use crate::*;
use engineering::{in_frame, prism, profile_tool, OpenFace};
use operations::{Frame, Profile};
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RegionKind {
    Outer,
    Cavity,
    Opening,
}
pub(crate) struct Regions {
    outer: Brep,
    cavity: Brep,
    openings: Vec<Brep>,
    open_faces: Vec<usize>,
    wall: f64,
    exact: bool,
}
fn copy(k: &mut Kernel, b: &Brep) -> Result<Brep> {
    Ok(solidcore::topology::brep::p3::brep_copy(k, b)?)
}
fn union(k: &mut Kernel, a: &Brep, b: &Brep) -> Result<Brep> {
    let a = copy(k, a)?;
    let b = copy(k, b)?;
    Ok(boolean_union(k, a, b)?)
}
fn common(k: &mut Kernel, a: &Brep, b: &Brep) -> Result<Brep> {
    let a = copy(k, a)?;
    let b = copy(k, b)?;
    Ok(boolean_intersection(k, a, b)?)
}
fn difference(k: &mut Kernel, a: &Brep, b: &Brep) -> Result<Brep> {
    let a = copy(k, a)?;
    let b = copy(k, b)?;
    Ok(boolean_difference(k, a, b)?)
}
fn vol(k: &Kernel, b: &mut Brep) -> Result<f64> {
    if b.face_list().is_empty() {
        Ok(0.)
    } else {
        Ok(brep_volume_default(k, b, 1e-6)?)
    }
}
impl Regions {
    fn duplicate(&self, k: &mut Kernel) -> Result<Self> {
        Ok(Self {
            outer: copy(k, &self.outer)?,
            cavity: copy(k, &self.cavity)?,
            openings: self
                .openings
                .iter()
                .map(|b| copy(k, b))
                .collect::<Result<_>>()?,
            open_faces: self.open_faces.clone(),
            wall: self.wall,
            exact: self.exact,
        })
    }
    fn each(&mut self, mut f: impl FnMut(&mut Brep) -> Result<()>) -> Result<()> {
        f(&mut self.outer)?;
        f(&mut self.cavity)?;
        for b in &mut self.openings {
            f(b)?;
        }
        Ok(())
    }
    pub fn report(&self) -> Value {
        json!({"exact":self.exact,"wall_mm":self.wall,"openings":self.openings.len(),"open_faces":self.open_faces})
    }
}
pub(crate) fn required(m: &Model) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for f in &m.features {
        if let Feature::InspectionRegion { input, .. } = f {
            set.insert(input.clone());
        }
    }
    for f in m.features.iter().rev() {
        if set.contains(f.id()) {
            for input in f.inputs() {
                set.insert(input.into());
            }
        }
    }
    set
}
pub(crate) struct Before {
    regions: Option<Regions>,
    source: Option<Brep>,
    operand: Option<Brep>,
}
pub(crate) fn before(
    k: &mut Kernel,
    f: &Feature,
    regions: &BTreeMap<String, Regions>,
    bodies: &BTreeMap<String, Brep>,
) -> Result<Before> {
    let input = f.inputs().first().copied();
    let old = match input.and_then(|i| regions.get(i)) {
        Some(r) => Some(r.duplicate(k)?),
        None => None,
    };
    let source = if matches!(
        f,
        Feature::Shell { .. }
            | Feature::Hollow { .. }
            | Feature::FaceCutout { .. }
            | Feature::FaceHoles { .. }
            | Feature::FaceHoleGrid { .. }
    ) {
        input
            .and_then(|id| bodies.get(id))
            .map(|b| copy(k, b))
            .transpose()?
    } else {
        None
    };
    let operand = match f {
        Feature::Cut { right, .. }
        | Feature::Union { right, .. }
        | Feature::Intersect { right, .. } => Some(copy(k, &bodies[right])?),
        _ => None,
    };
    Ok(Before {
        regions: old,
        source,
        operand,
    })
}
fn shifted(k: &mut Kernel, mut b: Brep, z: f64) -> Result<Brep> {
    operations::transform(k, &mut b, Vec3::new(0., 0., z), Vec3::ZERO)?;
    Ok(b)
}
fn rebuild(k: &mut Kernel, r: &mut Regions, body: &Brep) -> Result<()> {
    let mut outer = union(k, body, &r.cavity)?;
    for opening in &r.openings {
        let piece = common(k, opening, &r.outer)?;
        if !piece.face_list().is_empty() {
            outer = boolean_union(k, outer, piece)?;
        }
    }
    r.outer = outer;
    Ok(())
}
fn through(k: &mut Kernel, r: &Regions, tool: &Brep) -> Result<bool> {
    let mut common = common(k, &r.cavity, tool)?;
    if vol(k, &mut common)? > 1e-8 {
        return Ok(true);
    }
    Ok(crate::distance::boundary_distance(k, &r.cavity, tool)? <= 1e-4)
}
fn pieces(k: &mut Kernel, body: &Brep) -> Result<Vec<Brep>> {
    if body.face_list().is_empty() {
        return Ok(vec![]);
    }
    if brep_material_census(body)?.0 <= 1 {
        return Ok(vec![copy(k, body)?]);
    }
    let neutral =
        exchange::to_model(body).map_err(|e| CadError::new("REGIONS_FAILED", e.to_string()))?;
    neutral
        .bodies
        .iter()
        .map(|body| exchange::build_body(k, body))
        .collect()
}
fn opening(k: &mut Kernel, r: &mut Regions, tool: Brep, column: Option<Brep>) -> Result<bool> {
    if !through(k, r, &tool)? {
        return Ok(false);
    }
    let mut region = tool;
    if let Some(column) = column {
        let inside = common(k, &column, &r.cavity)?;
        for piece in pieces(k, &inside)? {
            if crate::distance::boundary_distance(k, &piece, &region)? <= 1e-6 {
                region = boolean_union(k, region, piece)?;
            }
        }
    }
    r.openings.push(region);
    Ok(true)
}
fn positioned_frame(f: &Frame, xy: [f64; 2], p: &BTreeMap<String, f64>) -> Result<Frame> {
    let (o, x, y) = f.resolve(p)?;
    let o = o + x * xy[0] + y * xy[1];
    Ok(Frame {
        origin: [o.x, o.y, o.z].map(Scalar::Number),
        x_axis: f.x_axis.clone(),
        y_axis: f.y_axis.clone(),
    })
}
fn column(
    k: &mut Kernel,
    profile: &Profile,
    frame: &Frame,
    depth: f64,
    p: &BTreeMap<String, f64>,
) -> Result<Brep> {
    profile_tool(k, profile, frame, depth, p)
}
pub(crate) fn after(
    k: &mut Kernel,
    f: &Feature,
    m: &Model,
    before: Before,
    body: &Brep,
) -> Result<Option<Regions>> {
    let p = &m.parameters;
    if matches!(f, Feature::FaceHoleGrid { .. }) {
        return after(k, &engineering::expand_grid(f, p)?, m, before, body);
    }
    if let Feature::ShellBody {
        edge_radius,
        profile,
        height,
        wall,
        floor,
        open_face,
        frame,
        ..
    } = f
    {
        let (h, w, fl) = (height.resolve(p)?, wall.resolve(p)?, floor.resolve(p)?);
        let mut outer = prism(k, profile, 0., h + 2. * w, p)?;
        let mut cavity = prism(k, profile, w, h - fl + w, p)?;
        let mut opening = prism(k, profile, 0., 2. * w + 1., p)?;
        match open_face {
            OpenFace::Top => {
                cavity = shifted(k, cavity, fl)?;
                opening = shifted(k, opening, h)?;
            }
            OpenFace::Bottom => {
                outer = shifted(k, outer, -2. * w)?;
                cavity = shifted(k, cavity, -w)?;
                opening = shifted(k, opening, -2. * w - 1.)?;
            }
        }
        for b in [&mut outer, &mut cavity, &mut opening] {
            in_frame(k, b, frame, p)?;
        }
        let mut regions = Regions {
            outer,
            cavity,
            openings: vec![opening],
            open_faces: vec![0],
            wall: w,
            exact: true,
        };
        if edge_radius.resolve(p)? > 0. {
            rebuild(k, &mut regions, body)?;
        }
        return Ok(Some(regions));
    }
    if let Feature::Shell { thickness, .. }
    | Feature::Hollow {
        wall: thickness, ..
    } = f
    {
        let outer = before
            .source
            .ok_or_else(|| CadError::new("REGIONS_MISSING", "shell source"))?;
        let cavity = difference(k, &outer, body)?;
        return Ok(Some(Regions {
            outer,
            cavity,
            openings: vec![],
            open_faces: vec![],
            wall: thickness.resolve(p)?,
            exact: false,
        }));
    }
    let Some(mut r) = before.regions else {
        return Ok(None);
    };
    match f {
        Feature::NameTopology { .. } => {}
        Feature::Translate { offset, .. } => {
            r.each(|b| operations::transform(k, b, point(offset, p)?, Vec3::ZERO))?
        }
        Feature::Place { .. } | Feature::Rotate { .. } | Feature::Mirror { .. } => r.each(|b| {
            let input = f.inputs()[0].to_string();
            let mut bodies = BTreeMap::from([(input.clone(), copy(k, b)?)]);
            let mut uses = BTreeMap::from([(input, 1)]);
            *b = operations::execute(k, f, m, &mut bodies, &mut uses)?;
            Ok(())
        })?,
        Feature::Union { .. } => {
            r.cavity = boolean_difference(k, r.cavity, before.operand.unwrap())?;
            if brep_material_census(&r.cavity)?.0 != 1 {
                return Err(CadError::new(
                    "CAVITY_SPLIT",
                    "join must preserve one connected cavity",
                ));
            }
            rebuild(k, &mut r, body)?;
        }
        Feature::Cut { .. } => {
            if !opening(k, &mut r, before.operand.unwrap(), None)? {
                rebuild(k, &mut r, body)?;
            }
        }
        Feature::Fillet { .. } | Feature::Chamfer { .. } => rebuild(k, &mut r, body)?,
        Feature::Cutout {
            profile,
            frame,
            depth,
            ..
        } => {
            let tool = column(k, profile, frame, depth.resolve(p)?, p)?;
            let bb = brep_bounding_box(k, &r.outer, true)?;
            let col = column(k, profile, frame, (bb.max() - bb.min()).length() * 4., p)?;
            if !opening(k, &mut r, tool, Some(col))? {
                rebuild(k, &mut r, body)?;
            }
        }
        Feature::FaceCutout {
            face,
            center,
            profile,
            depth,
            ..
        } => {
            let source = before.source.as_ref().unwrap();
            let f = engineering::face_frame(
                k,
                source,
                face,
                [center[0].resolve(p)?, center[1].resolve(p)?],
                1.,
            )?;
            let tool = column(k, profile, &f, depth.resolve(p)? + 1.0001, p)?;
            let bb = brep_bounding_box(k, &r.outer, true)?;
            let col = column(k, profile, &f, 4. * (bb.max() - bb.min()).length(), p)?;
            if !opening(k, &mut r, tool, Some(col))? {
                rebuild(k, &mut r, body)?;
            }
        }
        Feature::Holes {
            positions,
            radius,
            depth,
            counterbore,
            ..
        }
        | Feature::FaceHoles {
            positions,
            radius,
            depth,
            counterbore,
            ..
        } => {
            let frames = positions
                .iter()
                .map(|xy| {
                    let xy = [xy[0].resolve(p)?, xy[1].resolve(p)?];
                    match f {
                        Feature::Holes { frame, .. } => positioned_frame(frame, xy, p),
                        Feature::FaceHoles { face, .. } => engineering::face_frame(
                            k,
                            before.source.as_ref().unwrap(),
                            face,
                            xy,
                            1.,
                        ),
                        _ => unreachable!(),
                    }
                })
                .collect::<Result<Vec<_>>>()?;
            let is_face = matches!(f, Feature::FaceHoles { .. });
            let extra = if is_face { 1.0001 } else { 0. };
            let bb = brep_bounding_box(k, &r.outer, true)?;
            let reach = 4. * (bb.max() - bb.min()).length();
            let profile = Profile::Circle {
                radius: radius.clone(),
            };
            let mut blind = false;
            for frame in frames {
                let mut tool = column(k, &profile, &frame, depth.resolve(p)? + extra, p)?;
                if let Some(cb) = counterbore {
                    let bore = column(
                        k,
                        &Profile::Circle {
                            radius: cb.radius.clone(),
                        },
                        &frame,
                        cb.depth.resolve(p)? + extra,
                        p,
                    )?;
                    tool = boolean_union(k, tool, bore)?;
                }
                let col = column(k, &profile, &frame, reach, p)?;
                blind |= !opening(k, &mut r, tool, Some(col))?;
            }
            if blind {
                rebuild(k, &mut r, body)?;
            }
        }
        Feature::HoleGrid {
            origin,
            radius,
            depth,
            count,
            spacing,
            ..
        } => {
            let o = point(origin, p)?;
            let [nx, ny] = grid_counts(count, p)?;
            let mut blind = false;
            for y in 0..ny {
                for x in 0..nx {
                    let at = o + Vec3::new(
                        x as f64 * spacing[0].resolve(p)?,
                        y as f64 * spacing[1].resolve(p)?,
                        0.,
                    );
                    let tool = create_cylinder(k, at, radius.resolve(p)?, depth.resolve(p)?)?;
                    let bb = brep_bounding_box(k, &r.outer, true)?;
                    let col = create_cylinder(
                        k,
                        at,
                        radius.resolve(p)?,
                        4. * (bb.max() - bb.min()).length(),
                    )?;
                    blind |= !opening(k, &mut r, tool, Some(col))?;
                }
            }
            if blind {
                rebuild(k, &mut r, body)?;
            }
        }
        Feature::Split {
            axis,
            value,
            keep_positive,
            ..
        } => {
            let i = axis.index();
            let v = value.resolve(p)?;
            let bb = brep_bounding_box(k, &r.outer, true)?;
            let clip = |k: &mut Kernel, extension: f64| -> Result<Brep> {
                let mut lo = [bb.min_at(0) - 1., bb.min_at(1) - 1., bb.min_at(2) - 1.];
                let mut hi = [bb.max_at(0) + 1., bb.max_at(1) + 1., bb.max_at(2) + 1.];
                if *keep_positive {
                    lo[i] = v - extension;
                } else {
                    hi[i] = v + extension;
                }
                Ok(create_box(
                    k,
                    Vec3::new(lo[0], lo[1], lo[2]),
                    hi[0] - lo[0],
                    hi[1] - lo[1],
                    hi[2] - lo[2],
                )?)
            };
            let cb = brep_bounding_box(k, &r.cavity, true)?;
            let crosses = v > cb.min_at(i) + 1e-6 && v < cb.max_at(i) - 1e-6;
            let outerclip = clip(k, if crosses { 2. * r.wall } else { 0. })?;
            r.outer = boolean_intersection(k, r.outer, outerclip)?;
            let cavityclip = clip(k, if crosses { r.wall } else { 0. })?;
            r.cavity = boolean_intersection(k, r.cavity, cavityclip)?;
            if vol(k, &mut r.cavity)? <= 1e-8 {
                return Ok(None);
            }
            let exact_half = clip(k, 0.)?;
            let mut kept = vec![];
            let mut open_faces = vec![];
            for (index, opening) in r.openings.iter().enumerate() {
                let clipped = common(k, opening, &exact_half)?;
                for piece in pieces(k, &clipped)? {
                    let mut outside = difference(k, &piece, &r.outer)?;
                    if vol(k, &mut outside)? > 1e-6 {
                        if r.open_faces.contains(&index) {
                            open_faces.push(kept.len());
                        }
                        kept.push(piece);
                    } else {
                        let mut material = difference(k, &piece, &r.cavity)?;
                        if vol(k, &mut material)? > 1e-6 {
                            return Err(CadError::new(
                                "REGIONS_FAILED",
                                "split opening traps wall material",
                            ));
                        }
                    }
                }
            }
            r.openings = kept;
            r.open_faces = open_faces;
            if !crosses {
                return Ok(Some(r));
            }
            let mut lo = [bb.min_at(0) - 1., bb.min_at(1) - 1., bb.min_at(2) - 1.];
            let mut hi = [bb.max_at(0) + 1., bb.max_at(1) + 1., bb.max_at(2) + 1.];
            if *keep_positive {
                hi[i] = v;
            } else {
                lo[i] = v;
            }
            let op = create_box(
                k,
                Vec3::new(lo[0], lo[1], lo[2]),
                hi[0] - lo[0],
                hi[1] - lo[1],
                hi[2] - lo[2],
            )?;
            r.open_faces.push(r.openings.len());
            r.openings.push(op);
        }
        Feature::Intersect { .. } => {
            let clip = before.operand.unwrap();
            r.each(|b| {
                *b = common(k, b, &clip)?;
                Ok(())
            })?;
        }
        _ => return Ok(None),
    }
    Ok(Some(r))
}
pub(crate) fn extract(
    k: &mut Kernel,
    map: &BTreeMap<String, Regions>,
    input: &str,
    kind: &RegionKind,
    index: usize,
    require_exact: bool,
) -> Result<Brep> {
    let r = map
        .get(input)
        .ok_or_else(|| CadError::new("REGIONS_MISSING", input))?;
    if require_exact && !r.exact {
        return Err(CadError::new(
            "REGIONS_NOT_EXACT",
            "regions are for placement only",
        ));
    }
    let b = match kind {
        RegionKind::Outer => &r.outer,
        RegionKind::Cavity => &r.cavity,
        RegionKind::Opening => r
            .openings
            .get(index)
            .ok_or_else(|| CadError::new("INVALID_REFERENCE", "opening index"))?,
    };
    copy(k, b)
}
