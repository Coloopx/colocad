//! SDD 0004 E01: outward surface normals are not automatically aligned mate axes.
use agent_cad::{build, Model};
use serde_json::{json, Value};

fn model(
    kind: &str,
    alignment: Option<&str>,
    twist: Option<f64>,
    movable: bool,
    dof: usize,
) -> Value {
    let mut mate = json!({"id":"mount","kind":kind,
        "a":{"instance":"a","frame":{"origin":[0,0,4],"x_axis":[1,0,0],"y_axis":[0,1,0]}},
        "b":{"instance":"b","frame":{"origin":[0,0,0],"x_axis":[1,0,0],"y_axis":[0,-1,0]}}});
    if let Some(alignment) = alignment {
        mate["alignment"] = json!(alignment);
    }
    if let Some(twist) = twist {
        mate["twist_deg"] = json!(twist);
    }
    json!({"version":1,"units":"mm","parameters":{},
        "features":[{"id":"block","op":"box","origin":[-2,-2,0],"size":[4,4,4]}],
        "assembly":[{"id":"a","input":"block"},{"id":"b","input":"block","translation":[0,0,4]}],
        "mates":{"anchors":if movable {vec!["a"]} else {vec!["a","b"]},
            "expected_degrees_of_freedom":dof,"constraints":[mate]}})
}
fn execute(v: Value) -> agent_cad::Result<agent_cad::Build> {
    let m: Model = serde_json::from_value(v).expect("additive direction schema");
    build(&m)
}
#[test]
fn legacy_direction_defaults_remain_aligned() {
    for kind in ["fixed", "concentric", "revolute", "prismatic"] {
        assert_eq!(
            execute(model(kind, None, None, false, 0))
                .err()
                .unwrap()
                .code,
            "MATE_INCONSISTENT"
        );
    }
}
#[test]
fn opposed_and_unoriented_axes_accept_outward_surface_frames() {
    for kind in ["concentric", "revolute"] {
        for alignment in ["opposed", "unoriented"] {
            execute(model(kind, Some(alignment), None, false, 0)).unwrap();
        }
    }
}
#[test]
fn opposed_full_frame_is_a_proper_rotation_with_explicit_twist() {
    for kind in ["fixed", "prismatic"] {
        execute(model(kind, Some("opposed"), Some(0.), false, 0)).unwrap();
        let mut v = model(kind, Some("opposed"), Some(90.), false, 0);
        v["mates"]["constraints"][0]["b"]["frame"]["x_axis"] = json!([0, 1, 0]);
        v["mates"]["constraints"][0]["b"]["frame"]["y_axis"] = json!([1, 0, 0]);
        execute(v).unwrap();
    }
}
#[test]
fn direction_policies_preserve_motion_degrees_of_freedom() {
    for (kind, dof) in [
        ("concentric", 2),
        ("revolute", 1),
        ("prismatic", 1),
        ("fixed", 0),
    ] {
        let r = execute(model(kind, Some("opposed"), None, true, dof)).unwrap();
        assert_eq!(
            r.report["assembly"]["mate_solution"]["degrees_of_freedom"],
            dof
        );
    }
}
#[test]
fn opposed_plane_and_opposed_concentric_are_compatible() {
    let mut v = model("concentric", Some("opposed"), None, true, 1);
    let mut plane = v["mates"]["constraints"][0].clone();
    plane.as_object_mut().unwrap().remove("alignment");
    plane["kind"] = json!("plane");
    plane["id"] = json!("surface");
    plane["opposed"] = json!(true);
    plane["distance"] = json!(0);
    v["mates"]["constraints"]
        .as_array_mut()
        .unwrap()
        .push(plane);
    execute(v).unwrap();
}
#[test]
fn incompatible_direction_still_fails_and_unoriented_fixed_is_invalid() {
    assert_eq!(
        execute(model("concentric", Some("aligned"), None, false, 0))
            .err()
            .unwrap()
            .code,
        "MATE_INCONSISTENT"
    );
    assert!(
        serde_json::from_value::<Model>(model("fixed", Some("unoriented"), None, false, 0))
            .is_err()
    );
}

#[test]
fn perturbed_poses_converge_and_report_physical_residuals() {
    for (kind, dof) in [
        ("fixed", 0),
        ("concentric", 2),
        ("revolute", 1),
        ("prismatic", 1),
    ] {
        let mut v = model(kind, Some("opposed"), None, true, dof);
        v["assembly"][1]["translation"] = json!([2, -3, 5]);
        v["assembly"][1]["rotation_deg"] = json!([10, -15, 20]);
        let r = execute(v).unwrap();
        let solution = &r.report["assembly"]["mate_solution"];
        assert!(solution["iterations"].as_u64().unwrap() > 0);
        let physical = &solution["constraints"][0]["physical_residual"];
        assert!(physical["linear_mm"].as_f64().unwrap() < 1e-6);
        assert!(physical["angular_rad"].as_f64().unwrap() < 1e-6);
        assert_eq!(solution["degrees_of_freedom"], dof);
    }
}

#[test]
fn unoriented_branch_is_reported_and_legacy_defaults_do_not_rewrite_ir() {
    for (rotation, branch) in [(vec![0, 0, 0], "opposed"), (vec![180, 0, 0], "aligned")] {
        let mut v = model("concentric", Some("unoriented"), None, true, 2);
        v["assembly"][1]["rotation_deg"] = json!(rotation);
        let r = execute(v).unwrap();
        assert_eq!(
            r.report["assembly"]["mate_solution"]["constraints"][0]["unoriented_branch"],
            branch
        );
    }
    for kind in ["fixed", "concentric", "revolute", "prismatic"] {
        let v = model(kind, None, None, false, 0);
        let m: Model = serde_json::from_value(v).unwrap();
        let serialized = serde_json::to_value(m).unwrap();
        let mate = &serialized["mates"]["constraints"][0];
        assert!(mate.get("alignment").is_none());
        assert!(mate.get("twist_deg").is_none());
    }
}
