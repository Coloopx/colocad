//! Convert a disposable NURBS copy for the shared trimmed-face mesher.
use crate::*;
use solidcore::curve::curve::{p2::CurveKind, Curve};
use solidcore::nurbs::b_spline_surface_n_lib::p2::OrientType;
use solidcore::surface::surface::p2::SurfaceKind;
use solidcore::topology::edgeuse::{p1 as eu, p3 as trims};
use solidcore_record::brep as r;
fn missing() -> CadError {
    CadError::new("PREVIEW_TOPOLOGY", "missing geometry in display conversion")
}
fn knots(values: Vec<f64>, counts: Vec<usize>) -> Vec<f64> {
    values
        .into_iter()
        .zip(counts)
        .flat_map(|(v, n)| std::iter::repeat_n(v, n))
        .collect()
}
fn curve(c: &Curve) -> Result<(Vec<f64>, Vec<[f64; 4]>)> {
    let bs = match c.kind() {
        CurveKind::BSpline(b) => b.clone(),
        CurveKind::Line(l) => l.base_with_nurb()?.into_owned(),
        CurveKind::Ellipse(e) => e.base().clone(),
        _ => return Err(CadError::new("PREVIEW_CURVE", "curve is not a NURBS form")),
    };
    let (pts, weights) = bs.control_polygon()?;
    let (kv, mul) = bs.knots(None)?;
    Ok((
        knots(kv, mul),
        pts.iter()
            .enumerate()
            .map(|(i, p)| {
                let w = weights.get(i).copied().unwrap_or(1.);
                [p.x * w, p.y * w, p.z * w, w]
            })
            .collect(),
    ))
}
pub fn convert(k: &mut Kernel, body: &Brep) -> Result<r::Brep> {
    let mut b = solidcore::topology::brep::p3::brep_copy(k, body)?;
    solidcore::topology::api_files::turn_to_nurbs(k, Some(&mut b))?;
    let vids = b.vertex_list().to_vec();
    let eids = b.edge_list().to_vec();
    let fids = b.face_list().to_vec();
    let vi: BTreeMap<_, _> = vids
        .iter()
        .enumerate()
        .map(|(i, v)| (*v, i as u32))
        .collect();
    let ei: BTreeMap<_, _> = eids
        .iter()
        .enumerate()
        .map(|(i, v)| (*v, i as u32))
        .collect();
    let vertices = vids
        .iter()
        .map(|v| {
            let v = b.vertex(*v)?;
            Ok(r::Vertex {
                point: [v.point.x, v.point.y, v.point.z],
                tolerance: v.tolerance,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let mut edges = vec![];
    for id in &eids {
        let e = b.edge(*id)?;
        let (knots, homogeneous) = curve(e.curve.as_ref().ok_or_else(missing)?)?;
        edges.push(r::Edge {
            curve: r::Curve::RationalSpline { knots, homogeneous },
            nurbs: None,
            interval: [e.interval.min(), e.interval.max()],
            vertices: [vi[&e.vertices[0]], vi[&e.vertices[1]]],
            tolerance: e.tolerance,
            uses: vec![],
        });
    }
    let mut faces = vec![];
    let mut oriented = vec![];
    let mut material = BTreeMap::new();
    for sh in b.shells().ids() {
        let sh = b.shell(sh)?;
        if let Some(reg) = sh.region {
            if Some(reg) != b.infinite_region() && b.region(reg)?.is_void() == 0 {
                for fu in sh.get_faceuses()? {
                    if let Some(f) = b.faceuse(fu)?.face {
                        material.insert(f, fu);
                    }
                }
            }
        }
    }
    for (fi, id) in fids.iter().enumerate() {
        let face = b.face(*id)?;
        let surface = face.surface.as_ref().ok_or_else(missing)?;
        let bs = match surface.kind() {
            SurfaceKind::BSpline(b) => b,
            _ => {
                return Err(CadError::new(
                    "PREVIEW_SURFACE",
                    "surface conversion did not produce NURBS",
                ))
            }
        };
        let canon = bs.canonical()?;
        let net = bs.control_point_net()?;
        let homogeneous = net
            .points
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let w = net.weights.get(i).copied().unwrap_or(1.);
                [p.x * w, p.y * w, p.z * w, w]
            })
            .collect::<Vec<_>>()
            .chunks(net.v_count)
            .map(|v| v.to_vec())
            .collect();
        let surface = r::Surface::RationalSpline {
            u_knots: knots(canon.u_knots, canon.u_multiplicities),
            v_knots: knots(canon.v_knots, canon.v_multiplicities),
            homogeneous,
        };
        let domain = face.uv_domain;
        let domain = [
            [domain.min().x, domain.max().x],
            [domain.min().y, domain.max().y],
        ];
        let fu = material
            .get(id)
            .copied()
            .or(face.faceuse)
            .ok_or_else(missing)?;
        let forward = b.faceuse(fu)?.orientation == OrientType::Same;
        let natural = if forward {
            fu
        } else {
            b.faceuse(fu)?.mate.ok_or_else(missing)?
        };
        let lus = eu::faceuse_loopuses(&b, natural, false, None)?;
        let mut loops = vec![];
        for (li, lu) in lus.iter().enumerate() {
            if let solidcore::topology::LoopuseStart::Vertexuse(Some(vu)) = b.loopuse(*lu)?.start {
                let v = b.vertexuse(vu)?.vertex.ok_or_else(missing)?;
                loops.push(r::Loop::Vertex { vertex: vi[&v] });
                continue;
            }
            let eus = eu::loopuse_edgeuses(&b, *lu)?;
            let mut uses = vec![];
            for (position, euse) in eus.iter().enumerate() {
                trims::get_or_create_uv_trim_curve(k, &mut b, *euse, false, false, true)?.status?;
                let (uv, _) = eu::uv_trim_curve(&b, *euse)?;
                let (knots, pts) = curve(uv.ok_or_else(missing)?)?;
                let edge = ei[&eu::edge(&b, *euse)?.ok_or_else(missing)?];
                let forward = eu::orientation(&b, *euse)? == OrientType::Same;
                uses.push(r::OrientedEdge {
                    edge,
                    forward,
                    pcurve: r::Curve2::RationalSpline {
                        knots,
                        homogeneous: pts.into_iter().map(|p| [p[0], p[1], p[3]]).collect(),
                    },
                });
                edges[edge as usize].uses.push(r::EdgeUseRef {
                    face: fi as u32,
                    r#loop: li as u32,
                    position: position as u32,
                    upper: !forward,
                });
            }
            loops.push(r::Loop::Edges { edges: uses });
        }
        faces.push(r::Face {
            surface,
            nurbs: None,
            domain,
            loops,
        });
        oriented.push(r::OrientedFace {
            face: fi as u32,
            forward: if material.is_empty() {
                forward
            } else {
                !forward
            },
        });
    }
    Ok(r::Brep {
        version: r::VERSION,
        tolerance: b.tolerance(),
        vertices,
        edges,
        faces,
        shells: vec![r::Shell::Faces {
            faces: oriented,
            wires: vec![],
        }],
        regions: vec![
            r::Region {
                shells: vec![0],
                void: false,
            },
            r::Region {
                shells: vec![0],
                void: false,
            },
        ],
    })
}
