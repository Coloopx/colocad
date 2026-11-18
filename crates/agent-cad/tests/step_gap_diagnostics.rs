//! [INPUT]: A generated STEP box with controlled line displacement.
//! [OUTPUT]: Public assembly import failure with its native tolerance evidence.
//! [POS]: Regression for precise B-rep failure receipts.
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use agent_cad::{build, Model, StepAsset};
use serde_json::json;
use sha2::{Digest, Sha256};
use step_import::model::Curve;

fn stock() -> Model {
    serde_json::from_value(json!({
        "version": 1, "units": "mm", "parameters": {},
        "features": [{"id":"box","op":"box","origin":[0,0,0],"size":[10,8,6]}], "output":"box"
    }))
    .unwrap()
}

fn imported(step: String) -> Model {
    let asset = StepAsset {
        sha256: format!("{:x}", Sha256::digest(step.as_bytes())),
        step,
    };
    let info = agent_cad::step_assembly::inspect(&asset).unwrap();
    let mut model: Model = serde_json::from_value(info["model_template"].clone()).unwrap();
    model.assets.insert("source".into(), asset);
    model
}

#[test]
fn displaced_edge_reports_native_tolerance_evidence() {
    let step = build(&stock()).unwrap().step;
    let original = build(&imported(step.clone())).unwrap();
    assert_eq!(original.report["geometry"]["solids"], 1);
    assert_eq!(original.report["validation"]["valid"], true);

    let mut neutral = step_import::import(step.as_bytes(), &Default::default()).unwrap();
    let Curve::Line { origin, dir, .. } = &mut neutral.bodies[0].edges[0].curve else {
        panic!("box edge is not a line");
    };
    let axis = dir.iter().position(|n| n.abs() < 0.1).unwrap();
    origin[axis] += 0.01;

    let corrupt = imported(step_export::write(&neutral).unwrap());
    let error = build(&corrupt)
        .err()
        .expect("displaced edge must be rejected");
    assert_eq!(error.code, "GEOMETRY_VALIDATION_FAILED");
    assert!(error
        .message
        .contains("Edge tolerance relative to brep tolerance"));
    let exact = format!(
        "precise edge-to-BRep upper tolerance [mm]: value={:.17e}, tolerance={:.17e}",
        0.02_f64, 0.01_f64,
    );
    assert!(error.message.contains(&exact), "{}", error.message);
}
