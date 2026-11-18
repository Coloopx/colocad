//! [INPUT]: 小型原生装配和确定性的后继失败特征。
//! [OUTPUT]: 依赖就绪早拒、固定变换/allowed pair与最终检查一致性。
//! [POS]: 精确几何与失败顺序回归，不用耗时推断检查时机。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use agent_cad::{build, Model};
use serde_json::{json, Value};
fn model() -> Value {
    json!({"version":1,"units":"mm","parameters":{},"features":[
 {"id":"a","op":"box","origin":[0,0,0],"size":[10,10,10]},
 {"id":"b","op":"box","origin":[5,0,0],"size":[10,10,10]},
 {"id":"later","op":"cut","left":"a","right":"a"}],
 "assembly":[{"id":"first","input":"a"},{"id":"second","input":"b"},{"id":"last","input":"later"}],
 "checks":[{"kind":"collision","parts":["first","second"],"max_overlap_mm3":0.001}]})
}
#[test]
fn rejects_collision_before_a_later_unrelated_geometry_error() {
    let error = build(&serde_json::from_value::<Model>(model()).unwrap())
        .err()
        .unwrap();
    assert_eq!(error.code, "ASSEMBLY_COLLISION");
    assert_eq!(error.feature_id.as_deref(), Some("b"));
    assert_eq!(
        error.details.as_ref().unwrap()["source_features"],
        json!(["a", "b"])
    );
    assert!(
        (error.details.as_ref().unwrap()["overlap_mm3"]
            .as_f64()
            .unwrap()
            - 500.)
            .abs()
            < 0.0001
    );
}
#[test]
fn fixed_placement_separation_and_allowed_overlap_preserve_the_final_checks() {
    for allowed in [false, true] {
        let mut v = model();
        v["features"].as_array_mut().unwrap().pop();
        v["assembly"].as_array_mut().unwrap().pop();
        if allowed {
            v["checks"][0]["allowed_pairs"] = json!([["first", "second"]]);
        } else {
            v["assembly"][1]["translation"] = json!([20, 0, 0]);
        }
        let result = build(&serde_json::from_value::<Model>(v).unwrap()).unwrap();
        let row = &result.report["assembly"]["checks"][0];
        assert_eq!(row["allowed"], json!(allowed));
        assert!(
            (row["overlap_mm3"].as_f64().unwrap() - if allowed { 500. } else { 0. }).abs() < 0.0001
        );
    }
}

#[test]
fn mate_models_keep_post_solve_checks_and_rigid_group_placements_are_respected() {
    let mut v = model();
    v["features"].as_array_mut().unwrap().pop();
    v["assembly"].as_array_mut().unwrap().pop();
    v["mates"] = json!({"anchors":["first"],"constraints":[{"id":"fixed","kind":"fixed","a":{"instance":"first"},"b":{"instance":"second"}}],"expected_degrees_of_freedom":0});
    let error = build(&serde_json::from_value::<Model>(v.clone()).unwrap())
        .err()
        .unwrap();
    assert_eq!(error.code, "ASSEMBLY_COLLISION");
    assert!(error.feature_id.is_none());
    assert!(error
        .details
        .as_ref()
        .unwrap()
        .get("source_features")
        .is_none());
    v.as_object_mut().unwrap().remove("mates");
    v["groups"] = json!([{"id":"moving","pose":{"translation":[40,0,0],"rotation_deg":[0,0,90]}}]);
    v["assembly"][1]["parent"] = json!("moving");
    let result = build(&serde_json::from_value::<Model>(v).unwrap()).unwrap();
    assert_eq!(
        result.report["assembly"]["checks"][0]["overlap_mm3"],
        json!(0.0)
    );
}
