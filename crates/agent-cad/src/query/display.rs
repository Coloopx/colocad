//! Optional build-time outlines bound to exact snapshot refs. Display-only; never measurement authority.
use super::snapshot::LoadedDefinition;
use crate::*;
use solidcore::EntityRef;
fn arr(v: Vec3) -> [f64; 3] {
    [v.x, v.y, v.z]
}
fn point(k: &Kernel, c: &solidcore::curve::curve::Curve, t: f64) -> Result<Vec3> {
    let p = c.evaluate_point(k, t)?;
    if ![p.x, p.y, p.z].into_iter().all(f64::is_finite) {
        return Err(CadError::new(
            "QUERY_UNSUPPORTED",
            "non-finite display curve",
        ));
    }
    Ok(p)
}
#[allow(clippy::too_many_arguments)]
fn segment(
    k: &Kernel,
    c: &solidcore::curve::curve::Curve,
    t: [f64; 2],
    p: [Vec3; 2],
    chord: f64,
    depth: usize,
    out: &mut Vec<[f64; 3]>,
    budget: &mut usize,
) -> Result<()> {
    let mid = (t[0] + t[1]) * 0.5;
    let q = point(k, c, mid)?;
    if (q - (p[0] + p[1]) * 0.5).length() > chord {
        if depth >= 14 {
            return Err(CadError::new(
                "QUERY_LIMIT",
                "display curve subdivision budget exceeded",
            ));
        }
        segment(k, c, [t[0], mid], [p[0], q], chord, depth + 1, out, budget)?;
        segment(k, c, [mid, t[1]], [q, p[1]], chord, depth + 1, out, budget)?;
    } else {
        if *budget == 0 {
            return Err(CadError::new(
                "QUERY_LIMIT",
                "query display exceeds 200000 points",
            ));
        }
        *budget -= 1;
        out.push(arr(p[1]));
    }
    Ok(())
}
pub(crate) fn definition(
    k: &Kernel,
    d: &LoadedDefinition,
    chord: f64,
    budget: &mut usize,
) -> Result<Value> {
    let reverse = d
        .entities
        .iter()
        .map(|(t, e)| (*e, format!("t{t}")))
        .collect::<BTreeMap<_, _>>();
    let mut faces = BTreeMap::new();
    let mut edges = BTreeMap::new();
    let mut vertices = BTreeMap::new();
    let mut loops = BTreeMap::new();
    for (t, e) in &d.entities {
        let id = format!("t{t}");
        match e {
            EntityRef::Edge(e) => {
                let edge = d.body.edge(*e)?;
                let curve = edge.curve.as_ref().ok_or_else(|| {
                    CadError::new("QUERY_UNSUPPORTED", "display edge without curve")
                })?;
                let mut points = vec![];
                let a = edge.interval.min();
                let b = edge.interval.max();
                let mut previous = point(k, curve, a)?;
                points.push(arr(previous));
                for i in 0..8 {
                    let lo = a + (b - a) * (i as f64) / 8.;
                    let hi = a + (b - a) * ((i + 1) as f64) / 8.;
                    let next = point(k, curve, hi)?;
                    segment(
                        k,
                        curve,
                        [lo, hi],
                        [previous, next],
                        chord,
                        0,
                        &mut points,
                        budget,
                    )?;
                    previous = next;
                }
                edges.insert(id, json!({"points_mm":points}));
            }
            EntityRef::Face(f) => {
                let f = d.body.face(*f)?;
                let es = f
                    .loops
                    .iter()
                    .map(|id| d.body.loop_(*id))
                    .collect::<solidcore::Result<Vec<_>>>()?
                    .into_iter()
                    .flat_map(|l| {
                        l.edges
                            .iter()
                            .map(|e| reverse[&EntityRef::Edge(e.edge)].clone())
                    })
                    .collect::<BTreeSet<_>>();
                faces.insert(id, json!({"edges":es}));
            }
            EntityRef::Loop(l) => {
                loops.insert(id,json!({"edges":d.body.loop_(*l)?.edges.iter().map(|e|reverse[&EntityRef::Edge(e.edge)].clone()).collect::<Vec<_>>()}));
            }
            EntityRef::Vertex(v) => {
                vertices.insert(id, json!({"point_mm":arr(d.body.vertex(*v)?.point)}));
            }
            _ => {}
        }
    }
    Ok(json!({"faces":faces,"edges":edges,"loops":loops,"vertices":vertices}))
}
