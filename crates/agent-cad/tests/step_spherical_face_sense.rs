//! [INPUT]: 合成圆角块、带闭合内腔的胶囊体与精确镜像。
//! [OUTPUT]: STEP 回读保留实体数、孔腔、体积与几何边界。
//! [POS]: 原生 STEP 球面方向回归。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use agent_cad::{build, Build, Model};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn make_model(features: Vec<Value>, output: &str) -> Model {
    serde_json::from_value(json!({
        "version": 1, "units": "mm", "parameters": {},
        "features": features, "output": output
    }))
    .unwrap()
}

fn roundtrip(source: &Build) -> Build {
    let imported = serde_json::from_value(json!({
        "version": 1, "units": "mm", "parameters": {},
        "assets": {
            "source": {"step": source.step, "sha256": format!("{:x}", Sha256::digest(&source.step))}
        },
        "features": [{"id": "part", "op": "import_step", "asset": "source"}],
        "output": "part"
    }))
    .unwrap();
    build(&imported).unwrap()
}

fn assert_close(left: &Value, right: &Value, tolerance: f64) {
    let (a, b) = (left.as_f64().unwrap(), right.as_f64().unwrap());
    assert!((a - b).abs() <= tolerance, "{a} != {b}");
}

fn assert_roundtrip(features: Vec<Value>, output: &str, reflected: bool, voids: u64) {
    let mut features = features;
    let output = if reflected {
        features.push(json!({
            "id": "reflected", "op": "mirror", "input": output,
            "origin": [0,0,0], "normal": [1,0,0]
        }));
        "reflected"
    } else {
        output
    };
    let source = build(&make_model(features, output)).unwrap();
    let imported = roundtrip(&source);
    let original = &source.report["geometry"];
    let result = &imported.report["geometry"];
    assert_eq!(result["solids"], 1);
    assert_eq!(result["void_regions"], voids);
    assert_eq!(imported.report["validation"]["valid"], true);
    assert_close(&original["volume_mm3"], &result["volume_mm3"], 1e-4);
    for side in 0..2 {
        for axis in 0..3 {
            assert_close(
                &original["bbox_mm"][side][axis],
                &result["bbox_mm"][side][axis],
                1e-5,
            );
        }
    }
}

#[test]
fn reflected_rounded_block_preserves_spherical_corners_through_step() {
    let features = vec![
        json!({"id":"stock","op":"box","origin":[8,-6,0],"size":[12,12,24]}),
        json!({"id":"rounded","op":"fillet","input":"stock","select":{"kind":"all"},"radius":4}),
    ];
    assert_roundtrip(features.clone(), "rounded", false, 0);
    assert_roundtrip(features, "rounded", true, 0);
}

fn capsule(id: &str, radius: f64) -> Vec<Value> {
    let tube = format!("{id}_tube");
    let bottom = format!("{id}_bottom");
    let top = format!("{id}_top");
    let half = format!("{id}_half");
    vec![
        json!({"id":tube,"op":"cylinder","origin":[12,0,2],"radius":radius,"height":22}),
        json!({"id":bottom,"op":"sphere","center":[12,0,2],"radius":radius}),
        json!({"id":half,"op":"union","left":tube,"right":bottom}),
        json!({"id":top,"op":"sphere","center":[12,0,24],"radius":radius}),
        json!({"id":id,"op":"union","left":half,"right":top}),
    ]
}

#[test]
fn reflected_hollow_capsule_preserves_closed_cavity_through_step() {
    let mut features = capsule("outer", 7.);
    features.extend(capsule("inner", 5.));
    features.push(json!({"id":"hollow","op":"cut","left":"outer","right":"inner"}));
    assert_roundtrip(features.clone(), "hollow", false, 1);
    assert_roundtrip(features, "hollow", true, 1);
}
