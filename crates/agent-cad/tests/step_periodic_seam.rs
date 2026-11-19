//! [INPUT]: 公开解析圆环、任意周期环起点/等价方向、原生import/export/build。
//! [OUTPUT]: 孔洞/拓扑/体积/bbox不变、往返有效及真正顶点破坏仍被拒绝。
//! [POS]: STEP周期接缝表示回归；无私有CAD、无容差变更。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use agent_cad::{build, Model};
use serde_json::json;
use sha2::{Digest, Sha256};
use solidcore::boolean::api_brep::boolean_difference;
use solidcore::primitive::api_primitives::create_cylinder;
use solidcore::topology::brep::validation::{check_brep, CheckLevel};
use solidcore::{Kernel, Vec3};
use step_import::model::{Body, Bound};

// 先由真实原生构造及Level-1独立确认源拓扑，再生成STEP；不手写区域/面向。
fn annulus(angle: f64) -> Body {
    let mut kernel = Kernel::default();
    kernel.fixes.uv_trim_checks_on_surface = true;
    let outer = create_cylinder(&mut kernel, Vec3::ZERO, 2., 0.025).unwrap();
    let cutter = create_cylinder(&mut kernel, Vec3::new(0., 0., -1.), 1., 2.025).unwrap();
    let mut ring = boolean_difference(&mut kernel, outer, cutter).unwrap();
    let (s, c) = angle.to_radians().sin_cos();
    solidcore::topology::brep::mirror::transform_matrix(
        &mut kernel,
        &mut ring,
        &[
            [c, s, 0., 0.],
            [-s, c, 0., 0.],
            [0., 0., 1., 0.],
            [3., 2., 5., 1.],
        ],
        true,
    )
    .unwrap();
    let checked = check_brep(&mut kernel, &mut ring, CheckLevel::Level1).unwrap();
    assert!(
        checked.is_valid(),
        "reference source must be valid before STEP: {:?}",
        checked.lines()
    );
    let mut neutral = step_brep::brep_to_model(&ring).unwrap();
    assert_eq!(neutral.bodies.len(), 1);
    neutral.bodies.remove(0)
}
fn step(body: Body) -> String {
    step_export::write(&step_import::model::Model {
        bodies: vec![body],
        ..Default::default()
    })
    .unwrap()
}
fn model(raw: &str) -> Model {
    serde_json::from_value(json!({"version":1,"units":"mm","parameters":{},
        "assets":{"source":{"step":raw,"sha256":format!("{:x}",Sha256::digest(raw.as_bytes()))}},
        "features":[{"id":"ring","op":"import_step","asset":"source","body_index":0,"definition":true}],"output":"ring"})).unwrap()
}
fn check(report: &serde_json::Value) {
    assert_eq!(report["validation"]["valid"], true, "{report}");
    assert_eq!(report["validation"]["manifold"], true);
    assert_eq!(report["geometry"]["solids"], 1);
    assert_eq!(report["geometry"]["faces"], 4);
    assert_eq!(report["geometry"]["edges"], 6);
    let expected = std::f64::consts::PI * (4. - 1.) * 0.025;
    assert!(
        (report["geometry"]["volume_mm3"].as_f64().unwrap() - expected).abs() < 1e-8,
        "{report}"
    );
    for (side, corner) in [[1., 0., 5.], [5., 4., 5.025]].iter().enumerate() {
        for (axis, v) in corner.iter().enumerate() {
            assert!(
                (report["geometry"]["bbox_mm"][side][axis].as_f64().unwrap() - v).abs() < 1e-6,
                "{report}"
            );
        }
    }
}
#[test]
fn cyclic_seam_starts_and_equivalent_loop_directions_keep_the_annular_hole() {
    for angle in [0., 37.] {
        for shift in 0..4 {
            for reverse in [false, true] {
                let mut body = annulus(angle);
                for face in &mut body.faces {
                    for loop_ in &mut face.loops {
                        if let Bound::Edges(edges) = &mut loop_.bound {
                            let n = edges.len();
                            edges.rotate_left(shift % n);
                            if reverse {
                                edges.reverse();
                                for e in edges {
                                    e.forward = !e.forward;
                                }
                                loop_.orientation = !loop_.orientation;
                            }
                        }
                    }
                }
                let raw = step(body);
                let first = build(&model(&raw)).unwrap_or_else(|e| {
                    panic!("angle={angle} shift={shift} reverse={reverse} import {e:?}")
                });
                check(&first.report);
                let second = build(&model(&first.step)).unwrap_or_else(|e| {
                    panic!("angle={angle} shift={shift} reverse={reverse} readback {e:?}")
                });
                check(&second.report);
            }
        }
    }
}
#[test]
fn a_real_annular_vertex_gap_is_still_rejected() {
    let mut body = annulus(37.);
    body.vertices[0].point[0] += 0.02;
    let error = build(&model(&step(body)))
        .err()
        .expect("true off-curve vertex must fail");
    assert!(
        matches!(
            error.code,
            "GEOMETRY_VALIDATION_FAILED" | "IMPORT_INCOMPLETE" | "IMPORT_FAILED"
        ),
        "{error:?}"
    );
}
