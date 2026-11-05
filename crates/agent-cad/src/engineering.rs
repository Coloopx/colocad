//! Product structure operations: exact offsets, openings and fitting lids.
use crate::*;
use operations::{profile_curves, Frame, Profile};
use solidcore::curve::curve::Curve;

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OpenFace {
    Top,
    Bottom,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Counterbore {
    pub radius: Scalar,
    pub depth: Scalar,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BoxFace {
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
}
pub(crate) fn face_frame(
    k: &Kernel,
    b: &Brep,
    face: &BoxFace,
    center: [f64; 2],
    outside: f64,
) -> Result<Frame> {
    let bb = brep_bounding_box(k, b, true)?;
    let [u, v] = center;
    let (o, x, n) = match face {
        BoxFace::PosX => ([bb.max_at(0) + outside, u, v], [0., -1., 0.], [-1., 0., 0.]),
        BoxFace::NegX => ([bb.min_at(0) - outside, u, v], [0., 1., 0.], [1., 0., 0.]),
        BoxFace::PosY => ([u, bb.max_at(1) + outside, v], [1., 0., 0.], [0., -1., 0.]),
        BoxFace::NegY => ([u, bb.min_at(1) - outside, v], [-1., 0., 0.], [0., 1., 0.]),
        BoxFace::PosZ => ([u, v, bb.max_at(2) + outside], [-1., 0., 0.], [0., 0., -1.]),
        BoxFace::NegZ => ([u, v, bb.min_at(2) - outside], [1., 0., 0.], [0., 0., 1.]),
    };
    let x = Vec3::new(x[0], x[1], x[2]);
    let n = Vec3::new(n[0], n[1], n[2]);
    let y = n.cross(x);
    Ok(Frame {
        origin: o.map(Scalar::Number),
        x_axis: [x.x, x.y, x.z].map(Scalar::Number),
        y_axis: [y.x, y.y, y.z].map(Scalar::Number),
    })
}
pub(crate) fn profile_tool(
    k: &mut Kernel,
    profile: &Profile,
    frame: &Frame,
    depth: f64,
    p: &BTreeMap<String, f64>,
) -> Result<Brep> {
    let mut tool = prism(k, profile, 0., depth, p)?;
    in_frame(k, &mut tool, frame, p)?;
    Ok(tool)
}
fn normalized_curves(
    k: &Kernel,
    profile: &Profile,
    p: &BTreeMap<String, f64>,
) -> Result<Vec<Curve>> {
    let mut curves = profile_curves(k, profile, &Frame::default(), p)?;
    let mut samples = vec![];
    for c in &curves {
        let iv = c.natural_interval();
        for i in 0..24 {
            samples.push(c.evaluate_point(k, iv.evaluate(i as f64 / 24.))?);
        }
    }
    let area = (0..samples.len())
        .map(|i| {
            let a = samples[i];
            let b = samples[(i + 1) % samples.len()];
            a.x * b.y - a.y * b.x
        })
        .sum::<f64>();
    if area.abs() < 1e-8 {
        return Err(CadError::new(
            "INVALID_PROFILE",
            "profile has zero signed area",
        ));
    }
    if area < 0. {
        curves.reverse();
        for c in &mut curves {
            let iv = c.natural_interval();
            c.reverse_parameterization(k, &iv)?;
        }
    }
    Ok(curves)
}
pub(crate) fn prism(
    k: &mut Kernel,
    profile: &Profile,
    inset: f64,
    height: f64,
    p: &BTreeMap<String, f64>,
) -> Result<Brep> {
    let curves = normalized_curves(k, profile, p)?;
    let refs = curves.iter().map(Some).collect::<Vec<_>>();
    if inset == 0. {
        return Ok(solidcore::sweep::api_brep::create_linear_sweep(
            k,
            &refs,
            Vec3::Z,
            height,
            true,
        )?);
    }
    let mut face = solidcore::primitive::api_primitives::create_offset_profile(
        k, &refs, inset, 1, false, false,
    )?;
    if face.face_list().len() != 1 {
        return Err(CadError::new(
            "OFFSET_FAILED",
            "inset must remain one connected profile",
        ));
    }
    let id = face.face_list()[0];
    let loops = solidcore::topology::face::p3::create_composites(k, &mut face, id)?;
    if loops.len() != 1 {
        return Err(CadError::new(
            "OFFSET_FAILED",
            "inset must remain one boundary loop",
        ));
    }
    let mut curves = vec![];
    for seg in loops[0].segments() {
        let mut c = seg.curve.clone();
        if !seg.same_sense {
            let iv = c.natural_interval();
            c.reverse_parameterization(k, &iv)?;
        }
        curves.push(c);
    }
    Ok(solidcore::sweep::api_brep::create_linear_sweep(
        k,
        &curves.iter().map(Some).collect::<Vec<_>>(),
        Vec3::Z,
        height,
        true,
    )?)
}
fn shift(k: &mut Kernel, b: &mut Brep, z: f64) -> Result<()> {
    operations::transform(k, b, Vec3::new(0., 0., z), Vec3::ZERO)
}
pub(crate) fn in_frame(
    k: &mut Kernel,
    b: &mut Brep,
    f: &Frame,
    p: &BTreeMap<String, f64>,
) -> Result<()> {
    let (o, x, y) = f.resolve(p)?;
    let z = x.cross(y);
    Ok(solidcore::topology::brep::mirror::transform_matrix(
        k,
        b,
        &[
            [x.x, x.y, x.z, 0.],
            [y.x, y.y, y.z, 0.],
            [z.x, z.y, z.z, 0.],
            [o.x, o.y, o.z, 1.],
        ],
        true,
    )?)
}
fn ensure_inset(k: &mut Kernel, outer: &Brep, inner: &Brep) -> Result<()> {
    let a = solidcore::topology::brep::p3::brep_copy(k, inner)?;
    let b = solidcore::topology::brep::p3::brep_copy(k, outer)?;
    let mut outside = boolean_difference(k, a, b)?;
    if !outside.face_list().is_empty() && brep_volume_default(k, &mut outside, 1e-6)? > 1e-6 {
        return Err(CadError::new(
            "OFFSET_FAILED",
            "inner profile is not contained in its outer profile",
        ));
    }
    Ok(())
}
pub(crate) fn expand_grid(f: &Feature, p: &BTreeMap<String, f64>) -> Result<Feature> {
    let Feature::FaceHoleGrid {
        id,
        input,
        face,
        center,
        count,
        spacing,
        radius,
        depth,
        counterbore,
    } = f
    else {
        unreachable!()
    };
    let [nx, ny] = grid_counts(count, p)?;
    let widest = 2.
        * counterbore
            .as_ref()
            .map_or_else(|| radius.resolve(p), |c| c.radius.resolve(p))?;
    let pitch = [
        positive(spacing[0].resolve(p)?)?,
        positive(spacing[1].resolve(p)?)?,
    ];
    if (nx > 1 && pitch[0] <= widest) || (ny > 1 && pitch[1] <= widest) {
        return Err(CadError::new(
            "OVERLAPPING_HOLES",
            "grid pitch must exceed widest bore diameter",
        ));
    }
    let center = [center[0].resolve(p)?, center[1].resolve(p)?];
    let mut positions = vec![];
    for y in 0..ny {
        for x in 0..nx {
            positions.push([
                Scalar::Number(center[0] + (x as f64 - (nx - 1) as f64 / 2.) * pitch[0]),
                Scalar::Number(center[1] + (y as f64 - (ny - 1) as f64 / 2.) * pitch[1]),
            ]);
        }
    }
    Ok(Feature::FaceHoles {
        id: id.clone(),
        input: input.clone(),
        face: face.clone(),
        positions,
        radius: radius.clone(),
        depth: depth.clone(),
        counterbore: counterbore.clone(),
    })
}
pub(crate) fn validate_feature(f: &Feature, p: &BTreeMap<String, f64>) -> Result<()> {
    match f {
        Feature::FaceHoleGrid { .. } => validate_feature(&expand_grid(f, p)?, p)?,
        Feature::DraftExtrude {
            profile,
            frame,
            height,
            angle_deg,
            ..
        } => {
            profile_curves(&Kernel::default(), profile, &Frame::default(), p)?;
            frame.resolve(p)?;
            positive(height.resolve(p)?)?;
            if angle_deg.resolve(p)?.abs() >= 80. {
                return Err(CadError::new(
                    "INVALID_ANGLE",
                    "draft angle must be within (-80,80) degrees",
                ));
            }
        }
        Feature::FaceCutout {
            center,
            profile,
            depth,
            ..
        }
        | Feature::FaceTool {
            center,
            profile,
            depth,
            ..
        } => {
            for v in center {
                v.resolve(p)?;
            }
            profile_curves(&Kernel::default(), profile, &Frame::default(), p)?;
            positive(depth.resolve(p)?)?;
        }
        Feature::FaceHoles {
            positions,
            radius,
            depth,
            counterbore,
            ..
        } => {
            validate_feature(
                &Feature::Holes {
                    id: "validation".into(),
                    input: "validation".into(),
                    frame: Frame::default(),
                    positions: positions.clone(),
                    radius: radius.clone(),
                    depth: depth.clone(),
                    counterbore: counterbore.clone(),
                },
                p,
            )?;
        }
        Feature::ShellBody {
            edge_radius,
            profile,
            height,
            wall,
            floor,
            frame,
            ..
        } => {
            profile_curves(&Kernel::default(), profile, &Frame::default(), p)?;
            frame.resolve(p)?;
            let h = positive(height.resolve(p)?)?;
            positive(wall.resolve(p)?)?;
            if edge_radius.resolve(p)? < 0. || edge_radius.resolve(p)? >= wall.resolve(p)? {
                return Err(CadError::new(
                    "INVALID_DIMENSION",
                    "rim edge radius must be nonnegative and below wall thickness",
                ));
            }
            let fl = positive(floor.resolve(p)?)?;
            if fl >= h {
                return Err(CadError::new(
                    "INVALID_DIMENSION",
                    "floor must be below shell height",
                ));
            }
        }
        Feature::LidFor {
            profile,
            wall,
            thickness,
            lip_height,
            clearance,
            frame,
            ..
        } => {
            profile_curves(&Kernel::default(), profile, &Frame::default(), p)?;
            frame.resolve(p)?;
            positive(wall.resolve(p)?)?;
            positive(thickness.resolve(p)?)?;
            positive(lip_height.resolve(p)?)?;
            if clearance.resolve(p)? < 0. {
                return Err(CadError::new(
                    "INVALID_DIMENSION",
                    "clearance must not be negative",
                ));
            }
        }
        Feature::Cutout {
            profile,
            frame,
            depth,
            ..
        } => {
            profile_curves(&Kernel::default(), profile, frame, p)?;
            positive(depth.resolve(p)?)?;
        }
        Feature::Holes {
            frame,
            positions,
            radius,
            depth,
            counterbore,
            ..
        } => {
            frame.resolve(p)?;
            let r = positive(radius.resolve(p)?)?;
            let d = positive(depth.resolve(p)?)?;
            if positions.is_empty() || positions.len() > MAX_CUTTERS {
                return Err(CadError::new("RESOURCE_LIMIT", "1..1024 holes required"));
            }
            let mut seen = Vec::new();
            for xy in positions {
                let q = [xy[0].resolve(p)?, xy[1].resolve(p)?];
                if seen
                    .iter()
                    .any(|v: &[f64; 2]| (v[0] - q[0]).hypot(v[1] - q[1]) < 1e-9)
                {
                    return Err(CadError::new(
                        "DUPLICATE_POSITION",
                        "hole positions must be distinct",
                    ));
                }
                seen.push(q);
            }
            if let Some(c) = counterbore {
                if positive(c.radius.resolve(p)?)? <= r || positive(c.depth.resolve(p)?)? >= d {
                    return Err(CadError::new(
                        "INVALID_DIMENSION",
                        "counterbore must be wider and no deeper than its hole",
                    ));
                }
            }
        }
        Feature::Split { value, .. } => {
            value.resolve(p)?;
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn execute(
    k: &mut Kernel,
    f: &Feature,
    m: &Model,
    bodies: &mut BTreeMap<String, Brep>,
    uses: &mut BTreeMap<String, usize>,
) -> Result<Brep> {
    let p = &m.parameters;
    Ok(match f {
        Feature::FaceHoleGrid { input, .. } => {
            if let Some(out) = grid::face_stock(k, m, f, &bodies[input])? {
                take_body(k, input, bodies, uses)?;
                out
            } else {
                execute(k, &expand_grid(f, p)?, m, bodies, uses)?
            }
        }
        Feature::DraftExtrude {
            profile,
            frame,
            height,
            angle_deg,
            ..
        } => {
            let curves = normalized_curves(k, profile, p)?;
            let mut b = solidcore::sweep::api_brep::create_taper_extrude(
                k,
                &curves.iter().map(Some).collect::<Vec<_>>(),
                height.resolve(p)?,
                angle_deg.resolve(p)?,
                3,
                false,
            )?;
            in_frame(k, &mut b, frame, p)?;
            b
        }
        Feature::FaceCutout {
            input,
            face,
            center,
            profile,
            depth,
            ..
        }
        | Feature::FaceTool {
            input,
            face,
            center,
            profile,
            depth,
            ..
        } => {
            let b = take_body(k, input, bodies, uses)?;
            let frame = face_frame(
                k,
                &b,
                face,
                [center[0].resolve(p)?, center[1].resolve(p)?],
                1e-4,
            )?;
            let tool = profile_tool(k, profile, &frame, depth.resolve(p)? + 2e-4, p)?;
            if matches!(f, Feature::FaceTool { .. }) {
                tool
            } else {
                operations::cut_checked(k, b, tool)?
            }
        }
        Feature::FaceHoles {
            input,
            face,
            positions,
            radius,
            depth,
            counterbore,
            ..
        } => {
            let mut b = take_body(k, input, bodies, uses)?;
            let frames = positions
                .iter()
                .map(|pos| face_frame(k, &b, face, [pos[0].resolve(p)?, pos[1].resolve(p)?], 1e-4))
                .collect::<Result<Vec<_>>>()?;
            for frame in frames {
                let mut tool = profile_tool(
                    k,
                    &Profile::Circle {
                        radius: radius.clone(),
                    },
                    &frame,
                    depth.resolve(p)? + 2e-4,
                    p,
                )?;
                if let Some(cb) = counterbore {
                    let bore = profile_tool(
                        k,
                        &Profile::Circle {
                            radius: cb.radius.clone(),
                        },
                        &frame,
                        cb.depth.resolve(p)? + 1e-4,
                        p,
                    )?;
                    tool = boolean_union(k, tool, bore)?;
                }
                b = operations::cut_checked(k, b, tool)?;
            }
            b
        }

        Feature::ShellBody {
            edge_radius,
            profile,
            height,
            wall,
            floor,
            open_face,
            frame,
            ..
        } => {
            let h = height.resolve(p)?;
            let wall = wall.resolve(p)?;
            let floor = floor.resolve(p)?;
            let outer = prism(k, profile, 0., h, p)?;
            let cavity_check = prism(k, profile, wall, h, p)?;
            ensure_inset(k, &outer, &cavity_check)?;
            let mut inner = prism(k, profile, wall, h - floor + 1., p)?;
            shift(
                k,
                &mut inner,
                match open_face {
                    OpenFace::Top => floor,
                    OpenFace::Bottom => -1.,
                },
            )?;
            let mut result = operations::cut_checked(k, outer, inner)?;
            let radius = edge_radius.resolve(p)?;
            if radius > 0. {
                let region = match open_face {
                    OpenFace::Top => operations::OuterEdges::Top,
                    OpenFace::Bottom => operations::OuterEdges::Bottom,
                };
                let count = profile_curves(k, profile, &Frame::default(), p)?.len();
                let edges = operations::select_edges(
                    k,
                    &result,
                    &operations::EdgeSelector::Outer {
                        region,
                        tolerance: Scalar::Number(1e-4),
                        expected_count: count,
                    },
                    p,
                )?;
                solidcore::blend::fillet_executive::api::fillet_edges(
                    k,
                    &mut result,
                    &edges,
                    radius,
                    1,
                    1,
                    1.,
                )?;
            }
            in_frame(k, &mut result, frame, p)?;
            result
        }
        Feature::LidFor {
            profile,
            wall,
            thickness,
            lip_height,
            clearance,
            frame,
            ..
        } => {
            let wall = wall.resolve(p)?;
            let thickness = thickness.resolve(p)?;
            let lip_height = lip_height.resolve(p)?;
            let clearance = clearance.resolve(p)?;
            let mut plate = prism(k, profile, 0., thickness, p)?;
            shift(k, &mut plate, lip_height)?;
            let outer = prism(k, profile, wall + clearance, lip_height, p)?;
            let inner = prism(k, profile, 2. * wall + clearance, lip_height, p)?;
            ensure_inset(k, &outer, &inner)?;
            let lip = operations::cut_checked(k, outer, inner)?;
            let mut result = boolean_union(k, plate, lip)?;
            in_frame(k, &mut result, frame, p)?;
            result
        }
        Feature::Cutout {
            input,
            profile,
            frame,
            depth,
            ..
        } => {
            let b = take_body(k, input, bodies, uses)?;
            let curves = profile_curves(k, profile, frame, p)?;
            let (_, x, y) = frame.resolve(p)?;
            let tool = solidcore::sweep::api_brep::create_linear_sweep(
                k,
                &curves.iter().map(Some).collect::<Vec<_>>(),
                x.cross(y),
                depth.resolve(p)?,
                true,
            )?;
            operations::cut_checked(k, b, tool)?
        }
        Feature::Holes {
            input,
            frame,
            positions,
            radius,
            depth,
            counterbore,
            ..
        } => {
            let mut b = take_body(k, input, bodies, uses)?;
            let (o, x, y) = frame.resolve(p)?;
            for xy in positions {
                let loc = o + x * xy[0].resolve(p)? + y * xy[1].resolve(p)?;
                let f = Frame {
                    origin: [
                        Scalar::Number(loc.x),
                        Scalar::Number(loc.y),
                        Scalar::Number(loc.z),
                    ],
                    x_axis: frame.x_axis.clone(),
                    y_axis: frame.y_axis.clone(),
                };
                let mut drill =
                    create_cylinder(k, Vec3::ZERO, radius.resolve(p)?, depth.resolve(p)?)?;
                if let Some(cb) = counterbore {
                    let bore = create_cylinder(
                        k,
                        Vec3::ZERO,
                        cb.radius.resolve(p)?,
                        cb.depth.resolve(p)?,
                    )?;
                    drill = boolean_union(k, drill, bore)?;
                }
                in_frame(k, &mut drill, &f, p)?;
                b = operations::cut_checked(k, b, drill)?;
            }
            b
        }
        Feature::Split {
            input,
            axis,
            value,
            keep_positive,
            ..
        } => {
            let b = take_body(k, input, bodies, uses)?;
            let bb = brep_bounding_box(k, &b, true)?;
            let i = axis.index();
            let v = value.resolve(p)?;
            if v <= bb.min_at(i) + 1e-6 || v >= bb.max_at(i) - 1e-6 {
                return Err(CadError::new(
                    "INVALID_SPLIT",
                    "split plane must cross body bounds",
                ));
            }
            let mut lo = [bb.min_at(0) - 1., bb.min_at(1) - 1., bb.min_at(2) - 1.];
            let mut hi = [bb.max_at(0) + 1., bb.max_at(1) + 1., bb.max_at(2) + 1.];
            if *keep_positive {
                lo[i] = v;
            } else {
                hi[i] = v;
            }
            let clip = create_box(
                k,
                Vec3::new(lo[0], lo[1], lo[2]),
                hi[0] - lo[0],
                hi[1] - lo[1],
                hi[2] - lo[2],
            )?;
            boolean_intersection(k, b, clip)?
        }
        _ => {
            return Err(CadError::new(
                "UNSUPPORTED_OPERATION",
                "no structural operation implementation",
            ))
        }
    })
}
