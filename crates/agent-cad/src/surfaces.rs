//! Tensor-product Bézier patches and exact parametric C2 bridge construction.
use crate::*;
use operations::V3;
use step_import::model as n;
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ControlEdit {
    pub u: usize,
    pub v: usize,
    pub point: V3,
}
fn net(m: &Model, id: &str, depth: usize) -> Result<Vec<Vec<Vec3>>> {
    if depth > 32 {
        return Err(CadError::new("SURFACE_BUDGET", "patch dependency depth"));
    }
    let f = m
        .features
        .iter()
        .find(|f| f.id() == id)
        .ok_or_else(|| CadError::new("INVALID_REFERENCE", id))?;
    let p = &m.parameters;
    match f {
        Feature::SurfaceNetwork {
            u_curves,
            v_curves,
            u_stations,
            v_stations,
            ..
        } => crate::network::interpolate(u_curves, v_curves, u_stations, v_stations, p),
        Feature::SurfacePatch { control_points, .. } => {
            if !(2..=8).contains(&control_points.len())
                || control_points
                    .iter()
                    .any(|r| !(2..=8).contains(&r.len()) || r.len() != control_points[0].len())
            {
                return Err(CadError::new(
                    "INVALID_SURFACE",
                    "rectangular 2..8 by 2..8 control net required",
                ));
            }
            control_points
                .iter()
                .map(|r| r.iter().map(|v| point(v, p)).collect())
                .collect()
        }
        Feature::SurfaceEdit { input, edits, .. } => {
            let mut out = net(m, input, depth + 1)?;
            if edits.is_empty() || edits.len() > 64 {
                return Err(CadError::new("SURFACE_BUDGET", "1..64 control edits"));
            }
            let mut seen = BTreeSet::new();
            for e in edits {
                if !seen.insert((e.u, e.v)) {
                    return Err(CadError::new(
                        "DUPLICATE_CONTROL_POINT",
                        "control point edited twice",
                    ));
                }
                let q = out
                    .get_mut(e.u)
                    .and_then(|r| r.get_mut(e.v))
                    .ok_or_else(|| CadError::new("INVALID_REFERENCE", "surface control point"))?;
                *q = point(&e.point, p)?;
            }
            Ok(out)
        }
        Feature::SurfaceBridge {
            start,
            end,
            start_scale,
            end_scale,
            ..
        } => {
            let a = net(m, start, depth + 1)?;
            let b = net(m, end, depth + 1)?;
            if a[0].len() != b[0].len() || a.len() < 3 || b.len() < 3 {
                return Err(CadError::new(
                    "CONTINUITY_INPUT",
                    "C2 bridge needs degree >=2 in U and matching V degree",
                ));
            }
            let sa = positive(start_scale.resolve(p)?)?;
            let sb = positive(end_scale.resolve(p)?)?;
            let n = a.len() - 1;
            let q = b.len() - 1;
            let mut out = vec![vec![Vec3::ZERO; a[0].len()]; 6];
            for j in 0..a[0].len() {
                out[0][j] = a[n][j];
                out[1][j] = out[0][j] + (a[n][j] - a[n - 1][j]) * (n as f64 * sa / 5.);
                out[2][j] = (a[n][j] - a[n - 1][j] * 2. + a[n - 2][j])
                    * ((n * (n - 1)) as f64 * sa * sa / 20.)
                    + out[1][j] * 2.
                    - out[0][j];
                out[5][j] = b[0][j];
                out[4][j] = b[0][j] - (b[1][j] - b[0][j]) * (q as f64 * sb / 5.);
                out[3][j] = (b[2][j] - b[1][j] * 2. + b[0][j])
                    * ((q * (q - 1)) as f64 * sb * sb / 20.)
                    + out[4][j] * 2.
                    - out[5][j];
            }
            Ok(out)
        }
        _ => Err(CadError::new(
            "SURFACE_REFERENCE",
            "expected a surface_patch, surface_edit or surface_bridge",
        )),
    }
}
fn xyz(p: Vec3) -> [f64; 3] {
    [p.x, p.y, p.z]
}
fn body(net: &[Vec<Vec3>]) -> n::Body {
    let (nu, nv) = (net.len(), net[0].len());
    let rows = net
        .iter()
        .map(|r| r.iter().copied().map(xyz).collect())
        .collect();
    let surface = n::Surface::BSpline(n::BSplineSurface {
        u_degree: (nu - 1) as u32,
        v_degree: (nv - 1) as u32,
        points: rows,
        weights: None,
        u_knots: vec![0., 1.],
        u_mults: vec![nu as u32; 2],
        v_knots: vec![0., 1.],
        v_mults: vec![nv as u32; 2],
    });
    let vertices = [
        net[0][0],
        net[nu - 1][0],
        net[nu - 1][nv - 1],
        net[0][nv - 1],
    ]
    .into_iter()
    .enumerate()
    .map(|(i, p)| n::Vertex {
        step_id: i as u64 + 1,
        point: xyz(p),
    })
    .collect();
    let boundaries = vec![
        (0..nu).map(|i| net[i][0]).collect::<Vec<_>>(),
        net[nu - 1].clone(),
        (0..nu).rev().map(|i| net[i][nv - 1]).collect(),
        net[0].iter().copied().rev().collect(),
    ];
    let edges = boundaries
        .into_iter()
        .enumerate()
        .map(|(i, points)| {
            let n = points.len();
            n::Edge {
                step_id: 10 + i as u64,
                start: i,
                end: (i + 1) % 4,
                curve: n::Curve::BSpline(n::BSplineCurve {
                    degree: (n - 1) as u32,
                    points: points.into_iter().map(xyz).collect(),
                    weights: None,
                    knots: vec![0., 1.],
                    mults: vec![n as u32; 2],
                }),
                same_sense: true,
                t_start: 0.,
                t_end: 1.,
            }
        })
        .collect();
    n::Body {
        step_id: 1,
        name: Some("parametric-patch".into()),
        kind: n::BodyKind::ShellModel,
        unit_mm: 1.,
        uncertainty_mm: Some(1e-6),
        vertices,
        edges,
        faces: vec![n::Face {
            step_id: 20,
            surface,
            same_sense: true,
            loops: vec![n::Loop {
                outer: true,
                orientation: true,
                bound: n::Bound::Edges(
                    (0..4)
                        .map(|edge| n::OrientedEdge {
                            edge,
                            forward: true,
                        })
                        .collect(),
                ),
            }],
        }],
        shells: vec![n::Shell {
            step_id: 30,
            closed: false,
            orientation: true,
            faces: vec![0],
        }],
    }
}
pub(crate) fn validate(f: &Feature, m: &Model) -> Result<()> {
    match f {
        Feature::SurfaceNetwork { .. }
        | Feature::SurfacePatch { .. }
        | Feature::SurfaceBridge { .. }
        | Feature::SurfaceEdit { .. } => {
            net(m, f.id(), 0)?;
        }
        Feature::Thicken { thickness, .. } => {
            positive(thickness.resolve(&m.parameters)?)?;
        }
        Feature::Sew { inputs, .. }
            if inputs.len() < 2
                || inputs.len() > 64
                || inputs.iter().collect::<BTreeSet<_>>().len() != inputs.len() =>
        {
            return Err(CadError::new(
                "SURFACE_BUDGET",
                "2..64 distinct surface inputs",
            ));
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
    match f {
        Feature::SurfaceNetwork { .. }
        | Feature::SurfacePatch { .. }
        | Feature::SurfaceBridge { .. }
        | Feature::SurfaceEdit { .. } => {
            let net = net(m, f.id(), 0)?;
            for input in f.inputs() {
                let _ = take_body(k, input, bodies, uses)?;
            }
            let b = step_brep::build_from_topology(&body(&net), step_brep::BuildOptions::default())
                .map_err(|e| CadError::new("SURFACE_BUILD", e.to_string()))?;
            if !b.face_failures.is_empty() {
                return Err(CadError::new("SURFACE_BUILD", "patch face or trims failed"));
            }
            Ok(b.brep)
        }
        Feature::Thicken {
            input, thickness, ..
        } => {
            let b = take_body(k, input, bodies, uses)?;
            use solidcore::offset::offset_executive::p2::{
                shell_brep_simple, OffsetOptions, ShellOptions,
            };
            let previous = k.fixes.shell_one_construction;
            k.fixes.shell_one_construction = true;
            let result = shell_brep_simple(
                k,
                &b,
                ShellOptions {
                    offset: OffsetOptions {
                        distance: thickness.resolve(&m.parameters)?,
                        extend_convex_edges: true,
                        merge_results: true,
                    },
                    create_offset_solid: true,
                    merge_sheet_side_walls: true,
                },
                &[],
            );
            k.fixes.shell_one_construction = previous;
            Ok(result?)
        }
        Feature::Sew { inputs, .. } => {
            let mut b = take_body(k, &inputs[0], bodies, uses)?;
            for input in &inputs[1..] {
                let part = take_body(k, input, bodies, uses)?;
                compound::append(k, &mut b, &part)?;
            }
            solidcore::topology::brep::p5::stitch_and_orient(k, &mut b, &Default::default())?;
            Ok(b)
        }
        _ => Err(CadError::new("UNSUPPORTED_OPERATION", "surface operation")),
    }
}
/// Control net and analytic boundary derivatives, not a screenshot-based continuity grade.
pub(crate) fn report(m: &Model, id: &str) -> Result<Value> {
    let n = net(m, id, 0)?;
    let degree = n.len() - 1;
    let jets = |end: bool| {
        (0..n[0].len())
            .map(|j| {
                let (p, d, dd) = if end {
                    (
                        n[degree][j],
                        (n[degree][j] - n[degree - 1][j]) * degree as f64,
                        if degree >= 2 {
                            (n[degree][j] - n[degree - 1][j] * 2. + n[degree - 2][j])
                                * (degree * (degree - 1)) as f64
                        } else {
                            Vec3::ZERO
                        },
                    )
                } else {
                    (
                        n[0][j],
                        (n[1][j] - n[0][j]) * degree as f64,
                        if degree >= 2 {
                            (n[2][j] - n[1][j] * 2. + n[0][j]) * (degree * (degree - 1)) as f64
                        } else {
                            Vec3::ZERO
                        },
                    )
                };
                json!({"position":xyz(p),"first_derivative":xyz(d),"second_derivative":xyz(dd)})
            })
            .collect::<Vec<_>>()
    };
    Ok(
        json!({"kind":"bezier_patch","u_degree":degree,"v_degree":n[0].len()-1,"u_start_jets":jets(false),"u_end_jets":jets(true),"class_a_certified":false}),
    )
}
