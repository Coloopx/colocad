use agent_cad::{build, validate, Model};
use serde_json::{json, Value};
fn model(v: Value) -> Model {
    serde_json::from_value(v).unwrap()
}
fn plate() -> Model {
    serde_json::from_str(include_str!("../../../examples/plate.json")).unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < b.abs().max(1.) * 1e-6, "{a} != {b}");
}
#[test]
fn holes_and_parameter_replay_preserve_design() {
    let original = plate();
    let a = build(&original).unwrap();
    close(
        a.report["geometry"]["volume_mm3"].as_f64().unwrap(),
        80. * 50. * 6. - 4. * std::f64::consts::PI * 9. * 6.,
    );
    let mut changed = original.clone();
    changed.parameters.insert("hole_radius".into(), 4.);
    let b = build(&changed).unwrap();
    close(
        b.report["geometry"]["volume_mm3"].as_f64().unwrap(),
        80. * 50. * 6. - 4. * std::f64::consts::PI * 16. * 6.,
    );
    assert_ne!(a.report["model_hash"], b.report["model_hash"]);
    assert_eq!(original.parameters["hole_radius"], 3.);
    assert_eq!(a.report["peak_live_feature_bodies"], 1);
}
#[test]
fn enclosure_derived_dimensions_follow_wall() {
    let mut m: Model =
        serde_json::from_str(include_str!("../../../examples/enclosure.json")).unwrap();
    m.parameters.insert("wall".into(), 3.);
    let a = build(&m).unwrap();
    close(
        a.report["geometry"]["volume_mm3"].as_f64().unwrap(),
        80. * 50. * 24. - 74. * 44. * 21.,
    );
}
#[test]
fn replay_is_deterministic() {
    let m = plate();
    assert_eq!(build(&m).unwrap().step, build(&m).unwrap().step);
}
#[test]
fn references_unknown_fields_and_budgets_fail_before_geometry() {
    let mut v = serde_json::to_value(plate()).unwrap();
    v["features"][1]["input"] = json!("future");
    assert_eq!(validate(&model(v)).unwrap_err().code, "INVALID_REFERENCE");
    let mut m = plate();
    m.parameters.insert("holes_x".into(), 1025.);
    assert_eq!(validate(&m).unwrap_err().code, "RESOURCE_LIMIT");
    let mut v = serde_json::to_value(plate()).unwrap();
    v["features"][0]["typo"] = json!(1);
    assert!(serde_json::from_value::<Model>(v).is_err());
}
#[test]
fn missing_holes_are_not_silent_success() {
    let mut v = serde_json::to_value(plate()).unwrap();
    v["features"][1]["origin"] = json!([100, 100, -1]);
    assert_eq!(build(&model(v)).err().unwrap().code, "NO_MATERIAL_REMOVED");
}
#[test]
fn branches_and_intersection_preserve_operands() {
    let m = model(json!({"version":1,"units":"mm","parameters":{},"features":[
 {"op":"box","id":"a","origin":[0,0,0],"size":[10,10,10]},
 {"op":"box","id":"b","origin":[5,0,0],"size":[10,10,10]},
 {"op":"intersect","id":"i","left":"a","right":"b"},
 {"op":"union","id":"u","left":"a","right":"i"}],"output":"u"}));
    close(
        build(&m).unwrap().report["geometry"]["volume_mm3"]
            .as_f64()
            .unwrap(),
        1000.,
    );
}
#[test]
fn curved_loft_exports_exact_surfaces() {
    let m: Model = serde_json::from_str(include_str!("../../../examples/loft.json")).unwrap();
    let b = build(&m).unwrap();
    assert!(b.step.contains("B_SPLINE_SURFACE"));
    assert!(!b.step.contains("FACETED_BREP"));
    assert!(b.report["geometry"]["volume_mm3"].as_f64().unwrap() > 1000.);
}
