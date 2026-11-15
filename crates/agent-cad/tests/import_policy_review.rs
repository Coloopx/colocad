/*
 * [INPUT]: Public host import policy and synthetic identity group chains; no private STEP assets.
 * [OUTPUT]: Query path admission before feature execution, inclusive boundary and shallow/preview controls.
 * [POS]: Review I2 regression; the invalid STEP sentinel proves preflight precedes all feature execution.
 * [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
 */
use agent_cad::{build_revision_with_import_limits, incremental::Checkpoints, Model, StepImportLimits};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn model(count: usize, shallow: bool) -> Model {
    serde_json::from_value(json!({
        "version":1,"units":"mm","parameters":{},
        "features":[{"id":"box","op":"box","origin":[0,0,0],"size":[1,1,1]}],
        "groups":(0..count).map(|i|json!({"id":format!("g{i}"),
            "parent":if shallow || i==0 {None} else {Some(format!("g{}",i-1))}})).collect::<Vec<_>>(),
        "assembly":[{"id":"part","input":"box","parent":format!("g{}",count-1)}]
    })).unwrap()
}
fn limits() -> StepImportLimits { StepImportLimits { max_groups:513, ..Default::default() } }
fn path_limit(error: agent_cad::CadError) {
    assert_eq!(error.code,"RESOURCE_LIMIT");
    let detail=&error.details.unwrap()["resource_limit"];
    assert_eq!(detail["resource"],"query_instance_path");
    assert_eq!(detail["actual"],514);assert_eq!(detail["allowed"],513);
    assert_eq!(detail["stage"],"downstream_capacity");
}

#[test]
fn review_i2_deep_query_is_rejected_before_any_feature_execution() {
    let mut m=model(513,false);
    let broken="deliberately invalid STEP sentinel";
    m.assets=serde_json::from_value(json!({"sentinel":{"step":broken,
        "sha256":format!("{:x}",Sha256::digest(broken.as_bytes()))}})).unwrap();
    m.features=serde_json::from_value(json!([{"id":"box","op":"import_step","asset":"sentinel","definition":true}])).unwrap();
    // Static admission succeeds. Without query preflight the real feature parser must fail.
    agent_cad::validate_with_import_limits(&m,limits()).unwrap();
    let plain=build_revision_with_import_limits(&m,&mut Checkpoints::default(),Some(false),limits()).err().expect("sentinel cannot execute");
    assert_eq!(plain.code,"STEP_ASSEMBLY");
    let query=build_revision_with_import_limits(&m,&mut Checkpoints::default(),Some(true),limits()).err().expect("unsupported query path");
    path_limit(query);
    m.preview=serde_json::from_value(json!({"chord_mm":0.4})).ok();
    let query_preview=build_revision_with_import_limits(&m,&mut Checkpoints::default(),Some(true),limits()).err().expect("unsupported query path with preview");
    path_limit(query_preview);
}

#[test]
fn review_i2_plain_deep_chain_remains_supported_and_query_is_structurally_refused() {
    let m=model(513,false);
    let plain=build_revision_with_import_limits(&m,&mut Checkpoints::default(),Some(false),limits()).unwrap();
    assert_eq!(plain.report["geometry"]["solids"],1);
    assert_eq!(plain.report["geometry"]["volume_mm3"],1.0);
    let query=build_revision_with_import_limits(&m,&mut Checkpoints::default(),Some(true),limits()).err().expect("path length514");
    path_limit(query);
}

#[test]
fn review_i2_inclusive_query_path_shallow_groups_and_flat_preview_controls() {
    for (count,shallow,want_path) in [(512,false,513),(513,true,2)] {
        let built=build_revision_with_import_limits(&model(count,shallow),&mut Checkpoints::default(),Some(true),limits()).unwrap();
        let manifest: Value=serde_json::from_slice(&built.artifacts["query-manifest.json"]).unwrap();
        assert_eq!(manifest["body"]["occurrences"]["part"]["instance_path"].as_array().unwrap().len(),want_path);
    }
    // Display stores a flat world pose and does not consume query.instance_path.
    let mut preview=model(513,false);preview.preview=serde_json::from_value(json!({"chord_mm":0.4})).ok();
    let shown=build_revision_with_import_limits(&preview,&mut Checkpoints::default(),Some(false),limits()).unwrap();
    assert_eq!(shown.report["geometry"]["solids"],1);assert_eq!(shown.report["preview"]["complete"],true);
    let scene: Value=serde_json::from_slice(&shown.artifacts["preview.mesh.json"]).unwrap();
    assert_eq!(scene["instances"].as_array().unwrap().len(),1);
}
