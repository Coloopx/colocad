//! Direct analytic topology for proven rectangular through-hole stock.
//! The stock oracle has checked separation, containment and full depth first.
use crate::*;
use step_import::model as n;
pub(crate) fn stock(k: &mut Kernel, m: &Model, f: &Feature) -> Result<Brep> {
    let Feature::HoleGrid {
        input,
        origin,
        radius,
        count,
        spacing,
        ..
    } = f
    else {
        unreachable!()
    };
    let Some(Feature::Box {
        origin: base, size, ..
    }) = m.features.iter().find(|f| f.id() == input)
    else {
        unreachable!()
    };
    let p = &m.parameters;
    let base = point(base, p)?;
    let size = point(size, p)?;
    let o = point(origin, p)?;
    let radius = radius.resolve(p)?;
    let [nx, ny] = grid_counts(count, p)?;
    plate(
        k,
        base,
        size,
        o,
        radius,
        [nx, ny],
        [spacing[0].resolve(p)?, spacing[1].resolve(p)?],
    )
}
fn plate(
    k: &mut Kernel,
    base: Vec3,
    size: Vec3,
    o: Vec3,
    radius: f64,
    count: [usize; 2],
    spacing: [f64; 2],
) -> Result<Brep> {
    let [nx, ny] = count;
    let stock = create_box(k, base, size.x, size.y, size.z)?;
    let mut model =
        exchange::to_model(&stock).map_err(|e| CadError::new("GRID_TOPOLOGY", e.to_string()))?;
    let b = &mut model.bodies[0];
    for face in &mut b.faces {
        face.same_sense = !face.same_sense;
        for lp in &mut face.loops {
            if let n::Bound::Edges(edges) = &mut lp.bound {
                edges.reverse();
                for e in edges {
                    e.forward = !e.forward;
                }
            }
        }
    }
    let face = |z: f64| {
        b.faces.iter().position(|f|matches!(&f.surface,n::Surface::Plane{frame}if frame.z[2].abs()>0.99&&(frame.origin[2]-z).abs()<1e-6)).ok_or_else(||CadError::new("GRID_TOPOLOGY","stock horizontal faces missing"))
    };
    let bottom = face(base.z)?;
    let top = face(base.z + size.z)?;
    let next = 100_000_u64;
    for y in 0..ny {
        for x in 0..nx {
            let (cx, cy) = (o.x + x as f64 * spacing[0], o.y + y as f64 * spacing[1]);
            let mut circles = vec![];
            for z in [base.z, base.z + size.z] {
                let frame = n::Frame {
                    origin: [cx, cy, z],
                    x: [1., 0., 0.],
                    y: [0., 1., 0.],
                    z: [0., 0., 1.],
                };
                let vertex = b.vertices.len();
                b.vertices.push(n::Vertex {
                    step_id: next + vertex as u64,
                    point: [cx + radius, cy, z],
                });
                let edge = b.edges.len();
                b.edges.push(n::Edge {
                    step_id: 2 * next + edge as u64,
                    start: vertex,
                    end: vertex,
                    curve: n::Curve::Circle { frame, radius },
                    same_sense: true,
                    t_start: 0.,
                    t_end: 360.,
                });
                circles.push(edge);
            }
            let lp = |edge, forward| n::Loop {
                outer: false,
                orientation: true,
                bound: n::Bound::Edges(vec![n::OrientedEdge { edge, forward }]),
            };
            b.faces[bottom].loops.push(lp(circles[0], true));
            b.faces[top].loops.push(lp(circles[1], false));
            let face = b.faces.len();
            b.faces.push(n::Face {
                step_id: 3 * next + face as u64,
                surface: n::Surface::Cylinder {
                    frame: n::Frame {
                        origin: [cx, cy, base.z],
                        x: [1., 0., 0.],
                        y: [0., 1., 0.],
                        z: [0., 0., 1.],
                    },
                    radius,
                },
                same_sense: false,
                loops: vec![lp(circles[0], false), lp(circles[1], true)],
            });
            b.shells[0].faces.push(face);
        }
    }
    let built = step_brep::build_from_topology(b, step_brep::BuildOptions::default())
        .map_err(|e| CadError::new("GRID_TOPOLOGY", e.to_string()))?;
    if !built.face_failures.is_empty() {
        return Err(CadError::new(
            "GRID_TOPOLOGY",
            "failed to construct every bore face",
        ));
    }
    Ok(built.brep)
}

pub(crate) fn face_stock(
    k: &mut Kernel,
    m: &Model,
    f: &Feature,
    source: &Brep,
) -> Result<Option<Brep>> {
    let Feature::FaceHoleGrid {
        input,
        face,
        center,
        count,
        spacing,
        radius,
        depth,
        counterbore,
        ..
    } = f
    else {
        unreachable!()
    };
    if counterbore.is_some()
        || !matches!(
            m.features.iter().find(|f| f.id() == input),
            Some(Feature::Box { .. })
        )
    {
        return Ok(None);
    }
    let p = &m.parameters;
    let frame = engineering::face_frame(
        k,
        source,
        face,
        [center[0].resolve(p)?, center[1].resolve(p)?],
        1e-4,
    )?;
    let (origin, x, y) = frame.resolve(p)?;
    let t = placement::Rigid {
        origin,
        axes: [x, y, x.cross(y)],
    };
    let mut local = solidcore::topology::brep::p3::brep_copy(k, source)?;
    t.inverse().apply(k, &mut local)?;
    let bb = brep_bounding_box(k, &local, true)?;
    let base = bb.min();
    let size = bb.max() - base;
    let counts = grid_counts(count, p)?;
    let pitch = [spacing[0].resolve(p)?, spacing[1].resolve(p)?];
    let radius = radius.resolve(p)?;
    let o = Vec3::new(
        -((counts[0] - 1) as f64) * pitch[0] / 2.,
        -((counts[1] - 1) as f64) * pitch[1] / 2.,
        0.,
    );
    if depth.resolve(p)? + 2e-4 < bb.max_at(2) + 1e-5 {
        return Ok(None);
    }
    for i in 0..2 {
        let start = [o.x, o.y][i];
        let end = start + (counts[i] - 1) as f64 * pitch[i];
        if start - radius <= bb.min_at(i) + 1e-4 || end + radius >= bb.max_at(i) - 1e-4 {
            return Ok(None);
        }
    }
    let mut out = plate(k, base, size, o, radius, counts, pitch)?;
    let expected = size.x * size.y * size.z
        - counts[0] as f64 * counts[1] as f64 * std::f64::consts::PI * radius * radius * size.z;
    let actual = brep_volume_default(k, &mut out, 1e-6)?;
    if (actual - expected).abs() > expected * 1e-6 + 1e-8 {
        return Err(CadError::new("VOLUME_MISMATCH", "face grid stock oracle"));
    }
    t.apply(k, &mut out)?;
    Ok(Some(out))
}
