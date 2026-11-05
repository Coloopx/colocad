//! Geometric operations with explicit units, frames and re-evaluated selectors.
use crate::*;
use solidcore::curve::{curve::Curve, ellipse::Ellipse};
use solidcore::geom::{axis2_placement::Axis2Placement, extent1d::Extent1d};
use solidcore::query::api_queries::{edge_bounding_box, edge_closest_point, face_bounding_box};
use solidcore::topology::{EdgeId, FaceId};

pub type V3 = [Scalar; 3];
pub type V2 = [Scalar; 2];
pub(crate) fn zero_vector() -> V3 {
    vector(0., 0., 0.)
}
pub(crate) fn zero() -> Scalar {
    Scalar::Number(0.)
}
fn vector(x: f64, y: f64, z: f64) -> V3 {
    [Scalar::Number(x), Scalar::Number(y), Scalar::Number(z)]
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub origin: V3,
    pub x_axis: V3,
    pub y_axis: V3,
}
impl Default for Frame {
    fn default() -> Self {
        Self {
            origin: vector(0., 0., 0.),
            x_axis: vector(1., 0., 0.),
            y_axis: vector(0., 1., 0.),
        }
    }
}
impl Frame {
    pub(crate) fn resolve(&self, p: &BTreeMap<String, f64>) -> Result<(Vec3, Vec3, Vec3)> {
        let o = point(&self.origin, p)?;
        let x = unit(point(&self.x_axis, p)?)?;
        let y = unit(point(&self.y_axis, p)?)?;
        if x.dot(y).abs() > 1e-8 {
            return Err(CadError::new(
                "INVALID_FRAME",
                "workplane axes must be orthogonal",
            ));
        }
        Ok((o, x, y))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Segment {
    Line {
        to: V2,
    },
    /// A circular arc through three points, with orientation determined by via.
    Arc {
        via: V2,
        to: V2,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Profile {
    Sketch {
        sketch: Box<crate::sketch::Sketch>,
    },
    Rectangle {
        width: Scalar,
        depth: Scalar,
        #[serde(default = "zero")]
        radius: Scalar,
    },
    Circle {
        radius: Scalar,
    },
    Ellipse {
        radii: V2,
    },
    Polygon {
        points: Vec<V2>,
    },
    Outline {
        start: V2,
        segments: Vec<Segment>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PathCurve {
    Line { start: V3, end: V3 },
    Arc { start: V3, via: V3, end: V3 },
    Bezier { control_points: Vec<V3> },
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Axis {
    X,
    Y,
    Z,
}
impl Axis {
    pub(crate) fn index(&self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EdgeSelector {
    Named {
        name: String,
        expected_count: usize,
    },
    Outer {
        region: OuterEdges,
        tolerance: Scalar,
        expected_count: usize,
    },
    All,
    Parallel {
        axis: V3,
        expected_count: usize,
    },
    OnPlane {
        axis: Axis,
        value: Scalar,
        tolerance: Scalar,
        expected_count: usize,
    },
    Near {
        points: Vec<V3>,
        tolerance: Scalar,
    },
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OuterEdges {
    Vertical,
    Top,
    Bottom,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
pub enum FaceSelector {
    Named {
        name: String,
        expected_count: usize,
    },
    OnPlane {
        axis: Axis,
        value: Scalar,
        tolerance: Scalar,
        expected_count: usize,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProfileSection {
    pub profile: Profile,
    #[serde(default)]
    pub frame: Frame,
}

pub(crate) fn unit(v: Vec3) -> Result<Vec3> {
    let n = v.length();
    if n < 1e-10 {
        return Err(CadError::new(
            "INVALID_DIRECTION",
            "nonzero direction required",
        ));
    }
    Ok(v / n)
}
fn p2(v: &V2, p: &BTreeMap<String, f64>) -> Result<[f64; 2]> {
    Ok([v[0].resolve(p)?, v[1].resolve(p)?])
}
fn line(a: Vec3, b: Vec3) -> Result<Curve> {
    if (b - a).length() < 1e-6 {
        return Err(CadError::new("INVALID_PROFILE", "zero-length segment"));
    }
    Ok(Curve::from(
        solidcore::nurbs::b_spline_curve::BSplineCurve::create_line_segment(3, a, b)?,
    ))
}
pub(crate) fn arc(k: &Kernel, a: Vec3, m: Vec3, b: Vec3) -> Result<Curve> {
    let u = m - a;
    let v = b - a;
    let n = u.cross(v);
    let n2 = n.dot(n);
    if n2 < 1e-16 {
        return Err(CadError::new(
            "INVALID_PROFILE",
            "three arc points must not be collinear",
        ));
    }
    let c = a + (v.cross(n) * u.dot(u) + n.cross(u) * v.dot(v)) / (2. * n2);
    let x = unit(a - c)?;
    let z = unit(n)?;
    let y = z.cross(x);
    let d = b - c;
    let mut angle = d.dot(y).atan2(d.dot(x));
    if angle <= 0. {
        angle += std::f64::consts::TAU;
    }
    Ok(Curve::from(Ellipse::create_canonical_circle(
        k,
        &Axis2Placement::new(c, x, y),
        (a - c).length(),
        Some(&Extent1d::from_min_max(0., angle.to_degrees())),
        None,
    )?))
}

pub(crate) fn profile_curves(
    k: &Kernel,
    profile: &Profile,
    frame: &Frame,
    p: &BTreeMap<String, f64>,
) -> Result<Vec<Curve>> {
    let (o, x, y) = frame.resolve(p)?;
    let world = |v: [f64; 2]| o + x * v[0] + y * v[1];
    let ellipse = |rx: f64, ry: f64| -> Result<Vec<Curve>> {
        positive(rx)?;
        positive(ry)?;
        Ok(vec![Curve::from(Ellipse::create_canonical(
            k,
            &Axis2Placement::new(o, x, y),
            rx,
            ry,
            None,
            None,
        )?)])
    };
    match profile {
        Profile::Sketch { sketch } => {
            profile_curves(k, &crate::sketch::solve(sketch, p)?.profile, frame, p)
        }
        Profile::Circle { radius } => ellipse(radius.resolve(p)?, radius.resolve(p)?),
        Profile::Ellipse { radii } => ellipse(radii[0].resolve(p)?, radii[1].resolve(p)?),
        Profile::Rectangle {
            width,
            depth,
            radius,
        } => {
            let w = positive(width.resolve(p)?)? / 2.;
            let h = positive(depth.resolve(p)?)? / 2.;
            let r = radius.resolve(p)?;
            if r < 0. || r > w.min(h) {
                return Err(CadError::new(
                    "INVALID_PROFILE",
                    "rectangle radius must be within half the smaller dimension",
                ));
            }
            if r < 1e-8 {
                let pts = [[-w, -h], [w, -h], [w, h], [-w, h]];
                return (0..4)
                    .map(|i| line(world(pts[i]), world(pts[(i + 1) % 4])))
                    .collect();
            }
            let centers = [
                [w - r, h - r],
                [-w + r, h - r],
                [-w + r, -h + r],
                [w - r, -h + r],
            ];
            let mut curves = vec![];
            for (i, c) in centers.iter().enumerate() {
                let a = i as f64 * std::f64::consts::FRAC_PI_2;
                let s = [c[0] + r * a.cos(), c[1] + r * a.sin()];
                let e = [
                    c[0] + r * (a + std::f64::consts::FRAC_PI_2).cos(),
                    c[1] + r * (a + std::f64::consts::FRAC_PI_2).sin(),
                ];
                let mid = [
                    c[0] + r * (a + std::f64::consts::FRAC_PI_4).cos(),
                    c[1] + r * (a + std::f64::consts::FRAC_PI_4).sin(),
                ];
                curves.push(arc(k, world(s), world(mid), world(e))?);
                let c2 = centers[(i + 1) % 4];
                let na = a + std::f64::consts::FRAC_PI_2;
                let next = [c2[0] + r * na.cos(), c2[1] + r * na.sin()];
                if (world(e) - world(next)).length() > 1e-6 {
                    curves.push(line(world(e), world(next))?);
                }
            }
            Ok(curves)
        }
        Profile::Polygon { points } => {
            if !(3..=128).contains(&points.len()) {
                return Err(CadError::new(
                    "RESOURCE_LIMIT",
                    "polygon requires 3..128 points",
                ));
            }
            let pts: Vec<_> = points
                .iter()
                .map(|v| p2(v, p).map(world))
                .collect::<Result<_>>()?;
            (0..pts.len())
                .map(|i| line(pts[i], pts[(i + 1) % pts.len()]))
                .collect()
        }
        Profile::Outline { start, segments } => {
            if segments.is_empty() || segments.len() > 128 {
                return Err(CadError::new(
                    "RESOURCE_LIMIT",
                    "outline requires 1..128 segments",
                ));
            }
            let first = world(p2(start, p)?);
            let mut at = first;
            let mut curves = vec![];
            for segment in segments {
                let (c, end) = match segment {
                    Segment::Line { to } => {
                        let end = world(p2(to, p)?);
                        (line(at, end)?, end)
                    }
                    Segment::Arc { via, to } => {
                        let end = world(p2(to, p)?);
                        (arc(k, at, world(p2(via, p)?), end)?, end)
                    }
                };
                curves.push(c);
                at = end;
            }
            if (at - first).length() > 1e-6 {
                curves.push(line(at, first)?);
            }
            Ok(curves)
        }
    }
}

pub(crate) fn path_curve(k: &Kernel, path: &PathCurve, p: &BTreeMap<String, f64>) -> Result<Curve> {
    match path {
        PathCurve::Line { start, end } => line(point(start, p)?, point(end, p)?),
        PathCurve::Arc { start, via, end } => {
            arc(k, point(start, p)?, point(via, p)?, point(end, p)?)
        }
        PathCurve::Bezier { control_points } => {
            use solidcore::nurbs::b_spline_curve::BSplineCurve;
            use solidcore::nurbs::math::CPoint;
            if !(2..=8).contains(&control_points.len()) {
                return Err(CadError::new(
                    "RESOURCE_LIMIT",
                    "Bezier requires 2..8 control points",
                ));
            }
            let pts = control_points
                .iter()
                .map(|v| point(v, p).map(|v| CPoint::new(v.x, v.y, v.z, 1.)))
                .collect::<Result<Vec<_>>>()?;
            let degree = pts.len() - 1;
            let mut knots = vec![0.; degree + 1];
            knots.extend(vec![1.; degree + 1]);
            Ok(Curve::from(BSplineCurve::from_data(
                3,
                degree,
                knots.len() - 1,
                degree,
                &pts,
                &knots,
            )?))
        }
    }
}

pub(crate) fn select_edges(
    k: &Kernel,
    b: &Brep,
    s: &EdgeSelector,
    p: &BTreeMap<String, f64>,
) -> Result<Vec<EdgeId>> {
    let all = b.edge_list();
    let mut selected = vec![];
    match s {
        EdgeSelector::Named {
            name,
            expected_count,
        } => {
            selected = all
                .iter()
                .copied()
                .filter(|e| crate::naming::has(b, solidcore::EntityRef::Edge(*e), name))
                .collect();
            expected(selected.len(), *expected_count)?;
        }
        EdgeSelector::Outer {
            region,
            tolerance,
            expected_count,
        } => {
            let tol = positive(tolerance.resolve(p)?)?;
            let bb = brep_bounding_box(k, b, true)?;
            for &e in all {
                let eb = edge_bounding_box(k, b, e, true)?;
                let exterior = (0..2).any(|i| {
                    (eb.min_at(i) - bb.min_at(i)).abs() <= tol
                        || (eb.max_at(i) - bb.max_at(i)).abs() <= tol
                });
                if !exterior {
                    continue;
                }
                let matches = match region {
                    OuterEdges::Vertical => {
                        (eb.max_at(0) - eb.min_at(0)).abs() <= tol
                            && (eb.max_at(1) - eb.min_at(1)).abs() <= tol
                            && eb.max_at(2) - eb.min_at(2) > tol
                    }
                    OuterEdges::Top => {
                        (eb.min_at(2) - bb.max_at(2)).abs() <= tol
                            && (eb.max_at(2) - bb.max_at(2)).abs() <= tol
                    }
                    OuterEdges::Bottom => {
                        (eb.min_at(2) - bb.min_at(2)).abs() <= tol
                            && (eb.max_at(2) - bb.min_at(2)).abs() <= tol
                    }
                };
                if matches {
                    selected.push(e);
                }
            }
            expected(selected.len(), *expected_count)?;
        }
        EdgeSelector::All => selected = all.to_vec(),
        EdgeSelector::Near { points, tolerance } => {
            let tol = positive(tolerance.resolve(p)?)?;
            if points.is_empty() || points.len() > 64 {
                return Err(CadError::new(
                    "RESOURCE_LIMIT",
                    "1..64 edge selection points required",
                ));
            }
            for pos in points {
                let q = point(pos, p)?;
                let matches: Vec<_> = all
                    .iter()
                    .copied()
                    .filter(|&e| {
                        edge_closest_point(k, b, e, q).is_ok_and(|d| d.closest.distance <= tol)
                    })
                    .collect();
                if matches.len() != 1 {
                    return Err(CadError::new(
                        "SELECTION_AMBIGUOUS",
                        format!("edge point matched {} edges; expected one", matches.len()),
                    ));
                }
                if !selected.contains(&matches[0]) {
                    selected.push(matches[0]);
                }
            }
        }
        EdgeSelector::OnPlane {
            axis,
            value,
            tolerance,
            expected_count,
        } => {
            let value = value.resolve(p)?;
            let tol = positive(tolerance.resolve(p)?)?;
            let i = axis.index();
            for &e in all {
                let bb = edge_bounding_box(k, b, e, true)?;
                if (bb.min_at(i) - value).abs() <= tol && (bb.max_at(i) - value).abs() <= tol {
                    selected.push(e);
                }
            }
            expected(selected.len(), *expected_count)?;
        }
        EdgeSelector::Parallel {
            axis,
            expected_count,
        } => {
            let axis = unit(point(axis, p)?)?;
            for &e in all {
                let ed = b.edge(e)?;
                if let Some(c) = ed.curve.as_ref() {
                    let a = c.evaluate_point(k, ed.interval.min())?;
                    let z = c.evaluate_point(k, ed.interval.max())?;
                    let delta = z - a;
                    if delta.length() < 1e-6 {
                        continue;
                    }
                    if unit(delta)?.cross(axis).length() > 1e-7 {
                        continue;
                    }
                    let straight = [0.25, 0.5, 0.75].into_iter().all(|t| {
                        c.evaluate_point(k, ed.interval.evaluate(t))
                            .is_ok_and(|v| (v - a).cross(axis).length() < 1e-6)
                    });
                    if straight {
                        selected.push(e);
                    }
                }
            }
            expected(selected.len(), *expected_count)?;
        }
    }
    if selected.is_empty() {
        return Err(CadError::new("SELECTION_EMPTY", "no edges matched"));
    }
    Ok(selected)
}
fn expected(actual: usize, want: usize) -> Result<()> {
    if want == 0 || actual != want {
        return Err(CadError::new(
            "SELECTION_AMBIGUOUS",
            format!("expected {want}, matched {actual}"),
        ));
    }
    Ok(())
}
pub(crate) fn select_faces(
    k: &Kernel,
    b: &Brep,
    selectors: &[FaceSelector],
    p: &BTreeMap<String, f64>,
) -> Result<Vec<FaceId>> {
    let mut result = vec![];
    for s in selectors {
        let matches = match s {
            FaceSelector::Named {
                name,
                expected_count,
            } => {
                let matches = b
                    .face_list()
                    .iter()
                    .copied()
                    .filter(|f| crate::naming::has(b, solidcore::EntityRef::Face(*f), name))
                    .collect::<Vec<_>>();
                expected(matches.len(), *expected_count)?;
                matches
            }
            FaceSelector::OnPlane {
                axis,
                value,
                tolerance,
                expected_count,
            } => {
                let v = value.resolve(p)?;
                let tol = positive(tolerance.resolve(p)?)?;
                let i = axis.index();
                let mut matches = vec![];
                for &f in b.face_list() {
                    let bb = face_bounding_box(k, b, f, true)?;
                    if (bb.min_at(i) - v).abs() <= tol && (bb.max_at(i) - v).abs() <= tol {
                        matches.push(f);
                    }
                }
                expected(matches.len(), *expected_count)?;
                matches
            }
        };
        for f in matches {
            if !result.contains(&f) {
                result.push(f);
            }
        }
    }
    Ok(result)
}

pub(crate) fn transform(
    k: &mut Kernel,
    b: &mut Brep,
    translation: Vec3,
    rotation: Vec3,
) -> Result<()> {
    let (sx, cx) = rotation.x.to_radians().sin_cos();
    let (sy, cy) = rotation.y.to_radians().sin_cos();
    let (sz, cz) = rotation.z.to_radians().sin_cos();
    // Row-vector matrix: rotate X, then Y, then Z, then translate.
    let apply = |v: Vec3| {
        let a = Vec3::new(v.x, cx * v.y - sx * v.z, sx * v.y + cx * v.z);
        let q = Vec3::new(cy * a.x + sy * a.z, a.y, -sy * a.x + cy * a.z);
        Vec3::new(cz * q.x - sz * q.y, sz * q.x + cz * q.y, q.z)
    };
    let x = apply(Vec3::X);
    let y = apply(Vec3::Y);
    let z = apply(Vec3::Z);
    matrix(k, b, [x, y, z], translation)
}
pub(crate) fn matrix(k: &mut Kernel, b: &mut Brep, axes: [Vec3; 3], o: Vec3) -> Result<()> {
    let [x, y, z] = axes;
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

pub(crate) fn validate_selector(s: &EdgeSelector, p: &BTreeMap<String, f64>) -> Result<()> {
    match s {
        EdgeSelector::Named {
            name,
            expected_count,
        } => {
            if name.is_empty()
                || name.len() > 129
                || *expected_count == 0
                || *expected_count > 20_000
            {
                return Err(CadError::new(
                    "INVALID_SELECTOR",
                    "name and expected count required",
                ));
            }
        }
        EdgeSelector::Outer {
            tolerance,
            expected_count,
            ..
        } => {
            positive(tolerance.resolve(p)?)?;
            if *expected_count == 0 || *expected_count > 20_000 {
                return Err(CadError::new("INVALID_SELECTOR", "expected count 1..20000"));
            }
        }
        EdgeSelector::All => {}
        EdgeSelector::Parallel {
            axis,
            expected_count,
        } => {
            unit(point(axis, p)?)?;
            if *expected_count == 0 || *expected_count > 20_000 {
                return Err(CadError::new(
                    "INVALID_SELECTOR",
                    "expected_count must be 1..20000",
                ));
            }
        }
        EdgeSelector::OnPlane {
            value,
            tolerance,
            expected_count,
            ..
        } => {
            value.resolve(p)?;
            positive(tolerance.resolve(p)?)?;
            if *expected_count == 0 || *expected_count > 20_000 {
                return Err(CadError::new(
                    "INVALID_SELECTOR",
                    "expected_count must be 1..20000",
                ));
            }
        }
        EdgeSelector::Near { points, tolerance } => {
            positive(tolerance.resolve(p)?)?;
            if points.is_empty() || points.len() > 64 {
                return Err(CadError::new(
                    "RESOURCE_LIMIT",
                    "1..64 edge selection points required",
                ));
            }
            for v in points {
                point(v, p)?;
            }
        }
    }
    Ok(())
}
pub(crate) fn validate_feature(f: &Feature, p: &BTreeMap<String, f64>) -> Result<()> {
    let k = Kernel::default();
    match f {
        Feature::Extrude {
            profile,
            holes,
            frame,
            height,
            ..
        } => {
            profile_curves(&k, profile, frame, p)?;
            positive(height.resolve(p)?)?;
            if holes.len() > 32 {
                return Err(CadError::new("RESOURCE_LIMIT", "at most 32 profile holes"));
            }
            for h in holes {
                profile_curves(&k, h, frame, p)?;
            }
        }
        Feature::Revolve {
            profile,
            frame,
            axis_origin,
            axis,
            angle_deg,
            ..
        } => {
            profile_curves(&k, profile, frame, p)?;
            point(axis_origin, p)?;
            unit(point(axis, p)?)?;
            let a = angle_deg.resolve(p)?;
            if a <= 0. || a > 360. {
                return Err(CadError::new(
                    "INVALID_ANGLE",
                    "revolution angle must be in (0,360]",
                ));
            }
        }
        Feature::Pipe { path, radius, .. } => {
            path_curve(&k, path, p)?;
            positive(radius.resolve(p)?)?;
        }
        Feature::CurveSweep {
            profile,
            frame,
            path,
            ..
        } => {
            profile_curves(&k, profile, frame, p)?;
            path_curve(&k, path, p)?;
        }
        Feature::Sweep {
            profile,
            frame,
            path,
            ..
        } => {
            profile_curves(&k, profile, frame, p)?;
            if path.is_empty() || path.len() > 64 {
                return Err(CadError::new(
                    "RESOURCE_LIMIT",
                    "1..64 path segments required",
                ));
            }
            for c in path {
                path_curve(&k, c, p)?;
            }
        }
        Feature::ProfileLoft {
            sections, degree, ..
        } => {
            if !(2..=12).contains(&sections.len())
                || *degree == 0
                || *degree > 3
                || *degree >= sections.len()
            {
                return Err(CadError::new(
                    "INVALID_LOFT",
                    "2..12 sections; degree 1..3 below section count",
                ));
            }
            let mut count = None;
            for s in sections {
                let n = profile_curves(&k, &s.profile, &s.frame, p)?.len();
                if count.is_some_and(|c| c != n) {
                    return Err(CadError::new(
                        "INVALID_LOFT",
                        "sections must have corresponding curve counts",
                    ));
                }
                count = Some(n);
            }
        }
        Feature::Place {
            translation,
            rotation_deg,
            ..
        } => {
            point(translation, p)?;
            point(rotation_deg, p)?;
        }
        Feature::Rotate {
            origin,
            axis,
            angle_deg,
            ..
        } => {
            point(origin, p)?;
            unit(point(axis, p)?)?;
            angle_deg.resolve(p)?;
        }
        Feature::Mirror { origin, normal, .. } => {
            point(origin, p)?;
            unit(point(normal, p)?)?;
        }
        Feature::Fillet { radius, select, .. } => {
            positive(radius.resolve(p)?)?;
            validate_selector(select, p)?;
        }
        Feature::Chamfer { length, select, .. } => {
            positive(length.resolve(p)?)?;
            validate_selector(select, p)?;
        }
        Feature::Shell {
            thickness,
            open_faces,
            ..
        } => {
            positive(thickness.resolve(p)?)?;
            for s in open_faces {
                match s {
                    FaceSelector::Named {
                        name,
                        expected_count,
                    } => validate_selector(
                        &EdgeSelector::Named {
                            name: name.clone(),
                            expected_count: *expected_count,
                        },
                        p,
                    )?,
                    FaceSelector::OnPlane {
                        axis,
                        value,
                        tolerance,
                        expected_count,
                    } => validate_selector(
                        &EdgeSelector::OnPlane {
                            axis: axis.clone(),
                            value: value.clone(),
                            tolerance: tolerance.clone(),
                            expected_count: *expected_count,
                        },
                        p,
                    )?,
                }
            }
            if open_faces.len() > 32 {
                return Err(CadError::new(
                    "RESOURCE_LIMIT",
                    "at most 32 open-face selectors",
                ));
            }
        }
        Feature::Boss {
            origin,
            outer_radius,
            hole_radius,
            height,
            hole_depth,
            ..
        } => {
            point(origin, p)?;
            let r = positive(outer_radius.resolve(p)?)?;
            let hole = positive(hole_radius.resolve(p)?)?;
            let h = positive(height.resolve(p)?)?;
            let d = positive(hole_depth.resolve(p)?)?;
            if hole >= r || d > h {
                return Err(CadError::new(
                    "INVALID_DIMENSION",
                    "boss hole must fit inside boss",
                ));
            }
        }
        Feature::Pattern { positions, .. } => {
            if positions.is_empty() || positions.len() > 64 {
                return Err(CadError::new(
                    "RESOURCE_LIMIT",
                    "pattern requires 1..64 positions",
                ));
            }
            let mut seen = Vec::new();
            for pos in positions {
                let q = point(pos, p)?;
                if seen.iter().any(|v: &Vec3| (*v - q).length() < 1e-9) {
                    return Err(CadError::new(
                        "DUPLICATE_POSITION",
                        "pattern positions must be distinct",
                    ));
                }
                seen.push(q);
            }
        }
        Feature::ComponentCad {
            size,
            frame_rotation_deg,
            ..
        } => {
            for s in size {
                positive(s.resolve(p)?)?;
            }
            for a in frame_rotation_deg {
                let a = a.resolve(p)? / 90.;
                if (a - a.round()).abs() > 1e-9 {
                    return Err(CadError::new(
                        "INVALID_ROTATION",
                        "component frame rotation must be a multiple of 90 degrees",
                    ));
                }
            }
        }
        Feature::ImportStep { .. } => {}
        _ => crate::engineering::validate_feature(f, p)?,
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
    use solidcore::sweep::api_brep::{
        create_linear_sweep, create_pipe_sweep, create_rotational_sweep,
        create_sweep_along_planar_path,
    };
    let p = &m.parameters;
    Ok(match f {
        Feature::Extrude {
            profile,
            holes,
            frame,
            height,
            ..
        } => {
            let (_, x, y) = frame.resolve(p)?;
            let curves = profile_curves(k, profile, frame, p)?;
            let refs = curves.iter().map(Some).collect::<Vec<_>>();
            let mut b = create_linear_sweep(k, &refs, x.cross(y), height.resolve(p)?, true)?;
            for h in holes {
                let c = profile_curves(k, h, frame, p)?;
                let inner = create_linear_sweep(
                    k,
                    &c.iter().map(Some).collect::<Vec<_>>(),
                    x.cross(y),
                    height.resolve(p)?,
                    true,
                )?;
                b = cut_checked(k, b, inner)?;
            }
            b
        }
        Feature::Revolve {
            profile,
            frame,
            axis_origin,
            axis,
            angle_deg,
            ..
        } => {
            let c = profile_curves(k, profile, frame, p)?;
            create_rotational_sweep(
                k,
                &c.iter().map(Some).collect::<Vec<_>>(),
                point(axis_origin, p)?,
                unit(point(axis, p)?)?,
                angle_deg.resolve(p)?,
                true,
            )?
        }
        Feature::Pipe { path, radius, .. } => {
            let c = path_curve(k, path, p)?;
            let r = radius.resolve(p)?;
            if let PathCurve::Arc { start, .. } = path {
                if let solidcore::curve::curve::p2::CurveKind::Ellipse(e) = c.kind() {
                    let position = e.position();
                    let axis = position.z_axis();
                    let center = position.origin();
                    let at = point(start, p)?;
                    let radial = unit(at - center)?;
                    let tangent = axis.cross(radial);
                    if r >= e.x_radius() {
                        return Err(CadError::new(
                            "INVALID_DIMENSION",
                            "pipe radius must be below arc bend radius",
                        ));
                    }
                    let section = Curve::from(Ellipse::create_canonical_circle(
                        k,
                        &Axis2Placement::new(at, axis, tangent.cross(axis)),
                        r,
                        None,
                        None,
                    )?);
                    let end = c.evaluate_point(k, c.natural_interval().max())? - center;
                    let mut angle = end
                        .dot(axis.cross(radial))
                        .atan2(end.dot(radial))
                        .to_degrees();
                    if angle <= 0. {
                        angle += 360.;
                    }
                    create_rotational_sweep(k, &[Some(&section)], center, axis, angle, true)?
                } else {
                    return Err(CadError::new(
                        "INVALID_PATH",
                        "arc did not create a circular path",
                    ));
                }
            } else {
                create_pipe_sweep(k, r, Some(&c), true)?
            }
        }
        Feature::CurveSweep {
            profile,
            frame,
            path,
            translational,
            ..
        } => {
            let c = profile_curves(k, profile, frame, p)?;
            let path = path_curve(k, path, p)?;
            solidcore::sweep::api_brep::create_curve_sweep(
                k,
                &c.iter().map(Some).collect::<Vec<_>>(),
                Some(&path),
                None,
                None,
                Some(&solidcore::primitive::primitive_creation::SweepOptions {
                    cap_ends: true,
                    profile_at_path_start: true,
                    translational_sweep: *translational,
                    ..Default::default()
                }),
            )?
            .brep
        }
        Feature::Sweep {
            profile,
            frame,
            path,
            ..
        } => {
            let c = profile_curves(k, profile, frame, p)?;
            let path = path
                .iter()
                .map(|v| path_curve(k, v, p))
                .collect::<Result<Vec<_>>>()?;
            create_sweep_along_planar_path(
                k,
                &c.iter().collect::<Vec<_>>(),
                &path.iter().collect::<Vec<_>>(),
                false,
                3,
            )?
        }
        Feature::ProfileLoft {
            sections, degree, ..
        } => {
            let mut curves = vec![];
            let mut counts = vec![];
            for s in sections {
                let c = profile_curves(k, &s.profile, &s.frame, p)?;
                counts.push(c.len());
                curves.extend(c);
            }
            solidcore::sweep::api_surfaces::create_skin_primitive(
                k,
                &counts,
                &curves.iter().map(Some).collect::<Vec<_>>(),
                *degree,
                true,
            )?
            .brep
        }
        Feature::Place {
            input,
            translation,
            rotation_order,
            rotation_deg,
            ..
        } => {
            let mut b = take_body(k, input, bodies, uses)?;
            crate::placement::Rigid::rotation(
                point(translation, p)?,
                point(rotation_deg, p)?,
                *rotation_order,
            )
            .apply(k, &mut b)?;
            b
        }
        Feature::Rotate {
            input,
            origin,
            axis,
            angle_deg,
            ..
        } => {
            let mut b = take_body(k, input, bodies, uses)?;
            let n = unit(point(axis, p)?)?;
            let o = point(origin, p)?;
            let (s, c) = angle_deg.resolve(p)?.to_radians().sin_cos();
            let rot = |v: Vec3| v * c + n.cross(v) * s + n * (n.dot(v) * (1. - c));
            matrix(
                k,
                &mut b,
                [rot(Vec3::X), rot(Vec3::Y), rot(Vec3::Z)],
                o - rot(o),
            )?;
            b
        }
        Feature::Mirror {
            input,
            origin,
            normal,
            ..
        } => {
            let mut b = take_body(k, input, bodies, uses)?;
            let n = unit(point(normal, p)?)?;
            let seed = if n.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
            let x = unit(seed.cross(n))?;
            solidcore::topology::brep::mirror::mirror(
                k,
                &mut b,
                &Axis2Placement::new(point(origin, p)?, x, n.cross(x)),
            )?;
            b
        }
        Feature::Fillet {
            input,
            select,
            radius,
            ..
        }
        | Feature::Chamfer {
            input,
            select,
            length: radius,
            ..
        } => {
            let mut b = take_body(k, input, bodies, uses)?;
            let edges = select_edges(k, &b, select, p)?;
            let linear = matches!(f, Feature::Chamfer { .. });
            let radius = radius.resolve(p)?;
            solidcore::blend::fillet_executive::api::fillet_edges(
                k,
                &mut b,
                &edges,
                radius,
                if linear { 0 } else { 1 },
                1,
                1.,
            )?;
            b
        }
        Feature::Shell {
            input,
            thickness,
            open_faces,
            ..
        } => {
            use solidcore::offset::offset_executive::p2::{
                shell_brep_simple, OffsetOptions, ShellOptions,
            };
            let b = take_body(k, input, bodies, uses)?;
            let faces = select_faces(k, &b, open_faces, p)?;
            shell_brep_simple(
                k,
                &b,
                ShellOptions {
                    offset: OffsetOptions {
                        distance: -thickness.resolve(p)?,
                        extend_convex_edges: true,
                        merge_results: true,
                    },
                    create_offset_solid: true,
                    merge_sheet_side_walls: true,
                },
                &faces,
            )?
        }
        Feature::Boss {
            origin,
            outer_radius,
            height,
            hole_radius,
            hole_depth,
            ..
        } => {
            let o = point(origin, p)?;
            let h = height.resolve(p)?;
            let d = hole_depth.resolve(p)?;
            let b = create_cylinder(k, o, outer_radius.resolve(p)?, h)?;
            let tool = create_cylinder(k, o + Vec3::Z * (h - d), hole_radius.resolve(p)?, d + 1.)?;
            cut_checked(k, b, tool)?
        }
        Feature::Pattern {
            input, positions, ..
        } => {
            let b = take_body(k, input, bodies, uses)?;
            let mut out = None;
            for pos in positions {
                let mut part = solidcore::topology::brep::p3::brep_copy(k, &b)?;
                transform(k, &mut part, point(pos, p)?, Vec3::new(0., 0., 0.))?;
                out = Some(match out {
                    None => part,
                    Some(mut a) => {
                        crate::compound::append(k, &mut a, &part)?;
                        a
                    }
                });
            }
            out.ok_or_else(|| CadError::new("INVALID_PATTERN", "empty pattern"))?
        }
        Feature::ComponentCad {
            asset,
            size,
            frame_rotation_deg,
            ..
        } => {
            let a = m
                .assets
                .get(asset)
                .ok_or_else(|| CadError::new("ASSET_NOT_FOUND", asset))?;
            let mut b = crate::step_assembly::build(k, &a.step)?;
            crate::placement::Rigid::rotation(
                Vec3::ZERO,
                point(frame_rotation_deg, p)?,
                crate::placement::RotationOrder::Zyx,
            )
            .apply(k, &mut b)?;
            let bb = brep_bounding_box(k, &b, true)?;
            let dims = point(size, p)?;
            for (i, n) in [dims.x, dims.y, dims.z].iter().enumerate() {
                let measured = bb.max_at(i) - bb.min_at(i);
                if (measured - n).abs() > (n * 1e-4).max(0.05) {
                    return Err(CadError::new("ASSET_MISMATCH",format!("axis {i}: native {measured} mm, expected {n} mm; components are never rescaled")));
                }
            }
            let shift = Vec3::new(
                -(bb.min_at(0) + bb.max_at(0)) / 2.,
                -(bb.min_at(1) + bb.max_at(1)) / 2.,
                -bb.min_at(2),
            );
            transform(k, &mut b, shift, Vec3::ZERO)?;
            b
        }
        Feature::ImportStep {
            asset,
            body_index,
            assembly,
            definition,
            ..
        } => {
            let a = m
                .assets
                .get(asset)
                .ok_or_else(|| CadError::new("ASSET_NOT_FOUND", asset))?;
            if *assembly && *definition {
                return Err(CadError::new(
                    "INVALID_IMPORT_MODE",
                    "choose assembly or definition",
                ));
            }
            if *assembly {
                return crate::step_assembly::build(k, &a.step);
            }
            let limits = step_import::Limits {
                max_entities: 100_000,
                max_list_depth: 24,
                max_ref_depth: 32,
                max_list_len: 16_384,
            };
            let parsed = step_import::import(a.step.as_bytes(), &limits)
                .map_err(|e| CadError::new("IMPORT_FAILED", e.to_string()))?;
            if !parsed.skipped.is_empty()
                || parsed.diagnostics.iter().any(|d|!(*definition && d.step_id.is_none() && d.message==format!("{} assembly instance(s) are counted but not applied; bodies are in their defining representation coordinates",parsed.assembly_instances)))
                || (parsed.assembly_instances > 0 && !definition)
            {
                return Err(CadError::new("IMPORT_INCOMPLETE","STEP contains skipped entities, ambiguous units or unapplied assembly transforms"));
            }
            let source = parsed.bodies.get(*body_index).ok_or_else(|| {
                CadError::new("INVALID_REFERENCE", "STEP body index out of range")
            })?;
            exchange::build_body(k, source)?
        }
        _ => crate::engineering::execute(k, f, m, bodies, uses)?,
    })
}

pub(crate) fn cut_checked(k: &mut Kernel, mut a: Brep, b: Brep) -> Result<Brep> {
    let before = brep_volume_default(k, &mut a, 1e-6)?;
    let mut out = boolean_difference(k, a, b)?;
    let after = brep_volume_default(k, &mut out, 1e-6)?;
    if before - after <= before.abs() * 1e-9 + 1e-9 {
        return Err(CadError::new(
            "NO_MATERIAL_REMOVED",
            "cut did not remove material",
        ));
    }
    Ok(out)
}
