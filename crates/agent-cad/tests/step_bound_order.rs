//! [INPUT]: 自有平面环集合、pinned step-import/step-brep及原Level1检查。
//! [OUTPUT]: 唯一包含外环的正例与歧义/交叉/坏绕向拒绝回归。
//! [POS]: G3-A导入桥接专项；只由Chandrasekhar Win统一队列运行。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use solidcore::topology::brep::validation::{check_brep, CheckLevel};
use solidcore::Kernel;
use step_brep::{BuildMode, BuildOptions};
use step_import::model::{
    Body, BodyKind, Bound, Curve, Edge, Face, Frame, Loop, OrientedEdge, Shell, Surface, Vertex,
};

const INNER_FIRST: &str = include_str!("fixtures/step-bound-order/annulus-inner-first.step");
const OUTER_FIRST: &str = include_str!("fixtures/step-bound-order/annulus-outer-first.step");
const WRONG_WINDING: &str = include_str!("fixtures/step-bound-order/annulus-wrong-winding.step");

fn imported(text: &str) -> Body {
    step_import::import(text.as_bytes(), &step_import::Limits::default())
        .unwrap()
        .bodies
        .remove(0)
}

fn checked(body: &Body) -> bool {
    let made = step_brep::build_from_topology(body, BuildOptions::default()).unwrap();
    assert!(made.face_failures.is_empty());
    let mut b = made.brep;
    check_brep(&mut Kernel::default(), &mut b, CheckLevel::Level1)
        .unwrap()
        .is_valid()
}

fn rejected_every_path(body: &Body) {
    let before = body.clone();
    assert!(step_brep::build_from_topology(body, BuildOptions::default()).is_err());
    for mode in [BuildMode::Default, BuildMode::Strict, BuildMode::StitchOnly] {
        let result = step_brep::build_body(
            body,
            BuildOptions {
                mode,
                ..Default::default()
            },
        );
        let error = result
            .err()
            .expect("ambiguous bounds must not enter fallback");
        assert!(error.message.contains("STEP_BOUND_ORDER"), "{error}");
        assert!(step_brep::build_body_with_file_pcurves(
            body,
            &Default::default(),
            BuildOptions {
                mode,
                file_pcurves: true,
                ..Default::default()
            }
        )
        .is_err());
    }
    assert_eq!(
        *body, before,
        "preflight must leave source geometry and winding intact"
    );
}

#[test]
fn untyped_inner_first_annulus_passes_without_changing_source() {
    let body = imported(INNER_FIRST);
    assert!(body.faces[0].loops.iter().all(|l| !l.outer));
    let before = body.clone();
    assert!(checked(&body));
    assert_eq!(body, before);
}

#[test]
fn already_ordered_annulus_stays_valid() {
    assert!(checked(&imported(OUTER_FIRST)));
}

#[test]
fn bad_winding_is_never_flipped_into_a_valid_face() {
    // 同向outer/hole在face sense和全部bound方向同时变化时仍必须拒绝。
    for same_sense in [true, false] {
        for bound_orientation in [true, false] {
            let mut body = imported(WRONG_WINDING);
            body.faces[0].same_sense = same_sense;
            for bound in &mut body.faces[0].loops {
                bound.orientation = bound_orientation;
            }
            let before = body.clone();
            rejected_every_path(&body);
            assert_eq!(body, before);
        }
    }
}

fn circles(inner_x: f64, inner_radius: f64, outer_radius: f64) -> Body {
    let mut body = imported(INNER_FIRST);
    for edge in &mut body.edges {
        let Curve::Circle { frame, radius } = &mut edge.curve else {
            panic!("circle fixture");
        };
        if *radius == 1.0 {
            frame.origin[0] = inner_x;
            *radius = inner_radius;
        } else {
            *radius = outer_radius;
        }
    }
    body.vertices[0].point = [inner_x + inner_radius, 0.0, 0.0];
    body.vertices[1].point = [outer_radius, 0.0, 0.0];
    body
}

#[test]
fn crossing_tangent_duplicate_and_disjoint_circles_are_rejected() {
    for body in [
        circles(1.5, 1.0, 2.0),
        circles(1.0, 1.0, 2.0),
        circles(0.0, 2.0, 2.0),
        circles(4.0, 1.0, 2.0),
    ] {
        rejected_every_path(&body);
    }
}

fn polygons(rings: &[&[[f64; 2]]]) -> Body {
    let frame = Frame {
        origin: [0.0; 3],
        x: [1.0, 0.0, 0.0],
        y: [0.0, 1.0, 0.0],
        z: [0.0, 0.0, 1.0],
    };
    let mut body = Body {
        step_id: 1,
        name: None,
        kind: BodyKind::ShellModel,
        unit_mm: 1.0,
        uncertainty_mm: Some(1e-5),
        shells: vec![Shell {
            step_id: 2,
            closed: false,
            orientation: true,
            faces: vec![0],
        }],
        faces: vec![],
        edges: vec![],
        vertices: vec![],
    };
    let mut loops = vec![];
    for ring in rings {
        let first = body.vertices.len();
        for p in *ring {
            body.vertices.push(Vertex {
                step_id: body.vertices.len() as u64 + 10,
                point: [p[0], p[1], 0.0],
            });
        }
        let mut uses = vec![];
        for (i, a) in ring.iter().enumerate() {
            let b = ring[(i + 1) % ring.len()];
            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            let length = dx.hypot(dy);
            let edge = body.edges.len();
            body.edges.push(Edge {
                step_id: edge as u64 + 100,
                start: first + i,
                end: first + (i + 1) % ring.len(),
                curve: Curve::Line {
                    origin: [a[0], a[1], 0.0],
                    dir: [dx / length, dy / length, 0.0],
                    magnitude: length,
                },
                same_sense: true,
                t_start: 0.0,
                t_end: 1.0,
            });
            uses.push(OrientedEdge {
                edge,
                forward: true,
            });
        }
        loops.push(Loop {
            outer: false,
            orientation: true,
            bound: Bound::Edges(uses),
        });
    }
    body.faces.push(Face {
        step_id: 3,
        surface: Surface::Plane { frame },
        same_sense: true,
        loops,
    });
    body
}

const OUTER: [[f64; 2]; 4] = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
const HOLE: [[f64; 2]; 4] = [[1.0, 1.0], [1.0, 2.0], [2.0, 2.0], [2.0, 1.0]];
const HOLE2: [[f64; 2]; 4] = [[6.0, 6.0], [6.0, 8.0], [8.0, 8.0], [8.0, 6.0]];

#[test]
fn two_separate_holes_and_concave_outer_have_true_containment() {
    let body = polygons(&[&HOLE, &OUTER, &HOLE2]);
    let before = body.clone();
    assert!(checked(&body));
    assert_eq!(body, before);
    let concave = [
        [0.0, 0.0],
        [10.0, 0.0],
        [10.0, 3.0],
        [3.0, 3.0],
        [3.0, 10.0],
        [0.0, 10.0],
    ];
    assert!(checked(&polygons(&[&HOLE, &concave])));
    // Inside the bounding box and outside the concave material: must be rejected.
    rejected_every_path(&polygons(&[&HOLE2, &concave]));
}

#[test]
fn nested_holes_and_multiple_explicit_outer_bounds_are_rejected() {
    let small = [[6.5, 6.5], [6.5, 7.0], [7.0, 7.0], [7.0, 6.5]];
    rejected_every_path(&polygons(&[&OUTER, &HOLE2, &small]));
    let mut body = polygons(&[&OUTER, &HOLE]);
    for bound in &mut body.faces[0].loops {
        bound.outer = true;
    }
    rejected_every_path(&body);
}

#[test]
fn bowtie_open_and_repeated_vertex_boundaries_are_rejected() {
    let bowtie = [[4.0, 4.0], [9.0, 9.0], [4.0, 9.0], [9.0, 4.0]];
    rejected_every_path(&polygons(&[&OUTER, &bowtie]));
    let mut open = polygons(&[&HOLE, &OUTER]);
    let Bound::Edges(edges) = &mut open.faces[0].loops[0].bound else {
        unreachable!()
    };
    edges.pop();
    rejected_every_path(&open);
    let repeated = [[1.0, 1.0], [1.0, 2.0], [2.0, 2.0], [1.0, 1.0], [2.0, 1.0]];
    rejected_every_path(&polygons(&[&OUTER, &repeated]));
}

#[test]
fn negative_face_sense_and_bound_orientation_are_preserved() {
    let mut body = imported(INNER_FIRST);
    body.faces[0].same_sense = false;
    for bound in &mut body.faces[0].loops {
        bound.orientation = false;
    }
    let before = body.clone();
    assert!(checked(&body));
    assert_eq!(body, before);
}

#[test]
fn two_arcs_sharing_both_endpoints_form_one_valid_ring() {
    let mut body = imported(INNER_FIRST);
    let original = body.edges[1].clone();
    let vertex = body.vertices.len();
    body.vertices.push(Vertex {
        step_id: 999,
        point: [-2.0, 0.0, 0.0],
    });
    body.edges[1].end = vertex;
    body.edges[1].t_end = 180.0;
    let mut second = original;
    second.start = vertex;
    second.t_start = 180.0;
    second.step_id = 1000;
    let edge = body.edges.len();
    body.edges.push(second);
    let Bound::Edges(uses) = &mut body.faces[0].loops[1].bound else {
        unreachable!()
    };
    uses.push(OrientedEdge {
        edge,
        forward: true,
    });
    assert!(checked(&body));
}

#[test]
fn degenerate_and_off_plane_untagged_boundaries_are_rejected() {
    let flat = [[1.0, 1.0], [2.0, 1.0], [3.0, 1.0]];
    rejected_every_path(&polygons(&[&OUTER, &flat]));
    let mut body = polygons(&[&HOLE, &OUTER]);
    body.vertices[0].point[2] = 0.1;
    rejected_every_path(&body);
}

#[test]
fn explicit_outer_and_single_bound_keep_the_original_path() {
    let mut body = imported(OUTER_FIRST);
    body.faces[0].loops[0].outer = true;
    let before = body.clone();
    assert!(checked(&body));
    assert_eq!(body, before);
    assert!(checked(&polygons(&[&OUTER])));
}
