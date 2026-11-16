//! [INPUT]: 公开完整实体、17度seam正例及单顶点错误负例。
//! [OUTPUT]: 原生import/build/export/readback断言与供Win独立OCP核验的原字节。
//! [POS]: G2完整交付无回归控制；不是材料等价已证明或新的G1修复。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
#[path = "fixtures/planar_conic_seam.rs"]
mod fixture;
use agent_cad::{build, Model};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::Write;

fn model(step: &str) -> Model {
    serde_json::from_value(json!({"version":1,"units":"mm","parameters":{},
        "assets":{"source":{"sha256":format!("{:x}",Sha256::digest(step.as_bytes())),"step":step}},
        "features":[{"id":"part","op":"import_step","asset":"source","assembly":true}],"output":"part"})).unwrap()
}
fn geometry(report: &Value) {
    assert_eq!(report["validation"]["valid"], true);
    assert_eq!(report["validation"]["manifold"], true);
    assert_eq!(report["geometry"]["solids"], 1);
    let volume = 3200. - 40. * std::f64::consts::PI;
    assert!((report["geometry"]["volume_mm3"].as_f64().unwrap() - volume).abs() < 1e-5, "{report}");
    for (i, corner) in [[0., 0., 0.], [20., 16., 10.]].iter().enumerate() {
        for (j, value) in corner.iter().enumerate() {
            assert!((report["geometry"]["bbox_mm"][i][j].as_f64().unwrap() - value).abs() < 1e-6);
        }
    }
}
fn artifact(name: &str, bytes: &[u8]) {
    if let Some(root) = std::env::var_os("COLOCAD_G2_SOLID_EVIDENCE") {
        let root = std::path::PathBuf::from(root);
        std::fs::create_dir_all(&root).unwrap();
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(root.join(name)).unwrap();
        file.write_all(bytes).unwrap();
    }
}
#[test]
fn closed_solid_seam_import_export_and_readback() {
    assert_eq!(format!("{:x}", Sha256::digest(fixture::REFERENCE.as_bytes())), fixture::REFERENCE_SHA256);
    let step = fixture::rotated_seam();
    // 先留输入原字节，即使原生构建失败也可继续审计本轮输入。
    artifact("reference.step", fixture::REFERENCE.as_bytes());
    artifact("seam17.step", step.as_bytes());
    artifact("negative.step", fixture::inconsistent_seam_vertex().as_bytes());
    // 完整原生入口，不复用第一次导入的BRep或checkpoint。
    let reference = build(&model(fixture::REFERENCE)).unwrap();
    let result = build(&model(&step)).unwrap();
    let readback = build(&model(&result.step)).unwrap();
    for result in [&reference, &result, &readback] { geometry(&result.report); }
    artifact("export.step", result.step.as_bytes());
    artifact("readback.step", readback.step.as_bytes());
    artifact("native-reports.json", &serde_json::to_vec_pretty(&json!({"reference":reference.report,"candidate":result.report,"readback":readback.report,"material_equivalence":"unknown_until_independent_OCP","source_transform":"only eight explicitly listed STEP records; closed circles/cylinder frame and matching seam geometry","seam_angle_deg":17,"original_fixture_sha256":format!("{:x}",Sha256::digest(fixture::REFERENCE.as_bytes()))})).unwrap());
}
#[test]
fn closed_solid_inconsistent_vertex_is_rejected_by_native_import() {
    let result = build(&model(&fixture::inconsistent_seam_vertex()));
    let error = result.err().expect("inconsistent closed STEP must be rejected");
    assert!(matches!(error.code, "GEOMETRY_VALIDATION_FAILED" | "IMPORT_INCOMPLETE" | "IMPORT_FAILED"), "{error:?}");
    artifact("negative-receipt.json", &serde_json::to_vec_pretty(&json!({"expected_rejection":true,"error":format!("{error:?}"),"source_inconsistency_mm":0.25})).unwrap());
}
