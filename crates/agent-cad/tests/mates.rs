use agent_cad::{build, Model};
use serde_json::json;
fn source(kind: &str, dof: usize) -> Model {
    serde_json::from_value(json!({"version":1,"units":"mm","parameters":{},"features":[{"id":"box","op":"box","origin":[0,0,0],"size":[4,4,4]}],"assembly":[{"id":"anchor","input":"box"},{"id":"moving","input":"box","translation":[12,3,2],"rotation_deg":[10,-15,20]}],"mates":{"anchors":["anchor"],"expected_degrees_of_freedom":dof,"constraints":[{"id":"mount","kind":kind,"a":{"instance":"anchor","frame":{"origin":[10,0,0],"x_axis":[1,0,0],"y_axis":[0,1,0]}},"b":{"instance":"moving"}}]}})).unwrap()
}
#[test]
fn fixed_mate_solves_pose_and_exports_exact_assembly() {
    let b = build(&source("fixed", 0)).unwrap();
    let r = &b.report["assembly"]["mate_solution"];
    assert_eq!(r["degrees_of_freedom"], 0);
    assert!(r["max_residual"].as_f64().unwrap() < 1e-6);
    let o = &r["placements"]["moving"]["origin_mm"];
    assert!((o[0].as_f64().unwrap() - 10.).abs() < 1e-6);
    assert!(o[1].as_f64().unwrap().abs() < 1e-6);
    assert!(b.step.contains("NEXT_ASSEMBLY_USAGE_OCCURRENCE"));
}
#[test]
fn revolute_mate_keeps_one_declared_motion_degree() {
    let b = build(&source("revolute", 1)).unwrap();
    assert_eq!(
        b.report["assembly"]["mate_solution"]["degrees_of_freedom"],
        1
    );
    assert_eq!(
        build(&source("revolute", 0)).err().unwrap().code,
        "MATE_DEGREES_OF_FREEDOM"
    );
}
#[test]
fn conflicting_fixed_anchors_fail() {
    let mut m = source("fixed", 0);
    m.mates.as_mut().unwrap().anchors.push("moving".into());
    assert_eq!(build(&m).err().unwrap().code, "MATE_INCONSISTENT");
}
