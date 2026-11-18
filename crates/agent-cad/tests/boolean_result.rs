//! [INPUT]: 合成斜管、近切球和可解析长方体。
//! [OUTPUT]: 拒绝原生错误成功，保留两序合法交并的精确体积。
//! [POS]: 从公开 Model/Build 入口执行的布尔结果回归。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use agent_cad::{build, Model};
use serde_json::{json, Value};

fn model(left: Value, right: Value, op: &str, ids: [&str; 2]) -> Model {
    serde_json::from_value(json!({
        "version": 1, "units": "mm", "parameters": {},
        "features": [left, right, {"id":"result","op":op,"left":ids[0],"right":ids[1]}],
        "output": "result"
    }))
    .unwrap()
}

#[test]
fn intersection_cannot_extend_outside_either_operand() {
    let axis_length = 371_f64.sqrt();
    let axis_factor = 1. - 2. / axis_length;
    let pipe = json!({
        "id":"pipe","op":"pipe",
        "path":{"kind":"line","start":[0,0,0],"end":[3,-1,19]},"radius":2
    });
    let ball = json!({
        "id":"ball","op":"sphere",
        "center":[3. * axis_factor,-axis_factor,19. * axis_factor],"radius":2.02
    });
    let error = build(&model(pipe, ball, "intersect", ["pipe", "ball"]))
        .err()
        .expect("the pinned kernel returned intersection vertices outside the sphere");
    assert_eq!(error.code, "GEOMETRY_VALIDATION_FAILED");
    assert_eq!(error.feature_id.as_deref(), Some("result"));
    let details = error.details.unwrap();
    assert_eq!(details["reason"], "boolean_vertices_outside_bounds");
    assert_eq!(details["operation"], "intersect");
    let x = details["vertices_bbox_mm"][1][0].as_f64().unwrap();
    let max_x = details["enclosing_bbox_mm"][1][0].as_f64().unwrap();
    assert!((max_x - (3. * axis_factor + 2.02)).abs() < 1e-4);
    assert!(x > max_x + 0.2);
}

#[test]
fn overlapping_boxes_preserve_union_and_intersection_in_both_orders() {
    let a = json!({"id":"a","op":"box","origin":[0,0,0],"size":[10,4,3]});
    let b = json!({"id":"b","op":"box","origin":[8,0,0],"size":[5,4,3]});
    for order in [["a", "b"], ["b", "a"]] {
        for (op, expected) in [("union", 156.), ("intersect", 24.)] {
            let product = build(&model(a.clone(), b.clone(), op, order)).unwrap();
            let geometry = &product.report["geometry"];
            assert_eq!(geometry["solids"], 1);
            assert!((geometry["volume_mm3"].as_f64().unwrap() - expected).abs() < 1e-6);
        }
    }
}
