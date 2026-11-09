use agent_cad::{build, Model};
use serde_json::{json, Value};
fn decl(def: &str, normal: [i32; 3]) -> Value {
    json!({"id":"mount","definition":def,"profile":"planar_hole_mount/v1","selectors":{"plane":{"where":{"surface_type":"plane","planar_normal_aligned_with":normal},"cardinality":{"min":1,"max":1}},"hole_group":{"opening_on":"plane","where":{"pattern":"rectangular","member_count":4,"equal_diameter":true},"cardinality":{"min":1,"max":1}}},"datum":{"origin":{"kind":"hole_group_center"},"normal":"outward","x_axis":{"kind":"orientation_datum","name":"key"}},"orientation_datums":[{"name":"key","kind":"direction","vector":[1,0,0]}],"requirements":{"verification_tolerance":{"linear_mm":1e-5,"angular_rad":1e-7},"pattern":{"kind":"rectangular","count":[2,2],"pitch_mm":[20,12]},"bore_diameter_mm":4,"termination":"through"}})
}
fn fixture() -> Value {
    let mut fs = vec![];
    for id in ["a", "b"] {
        let raw = format!("{id}Raw");
        fs.push(json!({"id":raw,"op":"box","origin":[-20,-15,0],"size":[40,30,8]}));
        fs.push(json!({"id":id,"op":"holes","input":raw,"frame":{"origin":[0,0,-1],"x_axis":[1,0,0],"y_axis":[0,1,0]},"positions":[[-10,-6],[10,-6],[-10,6],[10,6]],"radius":2,"depth":10}));
    }
    json!({"version":1,"units":"mm","parameters":{},"features":fs,"interfaces":[decl("a",[0,0,1]),decl("b",[0,0,-1])],"assembly":[{"id":"base","input":"a"},{"id":"motor","input":"b","translation":[2,1,10],"rotation_deg":[5,-8,12]}],"mates":{"anchors":["base"],"constraints":[{"id":"mount","kind":"fixed","alignment":"opposed","a":{"instance":"base","interface":"mount"},"b":{"instance":"motor","interface":"mount"}}]}})
}
#[test]
fn declared_interfaces_drive_fixed_mate_and_bind_final_snapshot_refs() {
    let m: Model = serde_json::from_value(fixture()).unwrap();
    let b = build(&m).unwrap();
    let s = &b.report["assembly"]["mate_solution"];
    assert_eq!(s["degrees_of_freedom"], 0);
    let p = &s["placements"]["motor"]["origin_mm"];
    for (i, want) in [0., 0., 8.].iter().enumerate() {
        assert!((p[i].as_f64().unwrap() - want).abs() < 1e-5);
    }
    let iface = &b.report["interfaces"]["a"]["mount"];
    assert_eq!(iface["provenance"], "declared");
    assert_eq!(iface["group"]["members"].as_array().unwrap().len(), 4);
    assert_eq!(
        iface["support_face"]["query_revision_id"],
        b.report["query_revision_id"]
    );
    assert_eq!(iface["orientation"]["source"], "declared_numeric_direction");
}
#[test]
fn symmetric_interface_without_key_cannot_silently_fix_rotation() {
    let mut v = fixture();
    for i in [0, 1] {
        v["interfaces"][i]["datum"]
            .as_object_mut()
            .unwrap()
            .remove("x_axis");
    }
    let m: Model = serde_json::from_value(v.clone()).unwrap();
    assert_eq!(build(&m).err().unwrap().code, "INTERFACE_AMBIGUOUS");
    v["mates"]["constraints"][0]["b"]["orientation"] = json!({"kind":"datum","name":"key"});
    v["mates"]["constraints"][0]["a"]["orientation"] = json!({"kind":"datum","name":"key"});
    let m: Model = serde_json::from_value(v).unwrap();
    assert!(build(&m).is_ok());
}
#[test]
fn declared_wrong_pitch_missing_hole_and_invalid_direction_fail_before_solve() {
    let mut v = fixture();
    v["interfaces"][0]["requirements"]["pattern"]["pitch_mm"] = json!([21, 12]);
    let m: Model = serde_json::from_value(v).unwrap();
    assert_eq!(build(&m).err().unwrap().code, "INTERFACE_INCOMPATIBLE");
    let mut v = fixture();
    v["features"][1]["positions"].as_array_mut().unwrap().pop();
    let m: Model = serde_json::from_value(v).unwrap();
    assert_eq!(build(&m).err().unwrap().code, "INTERFACE_NOT_FOUND");
    let mut v = fixture();
    v["interfaces"][0]["orientation_datums"][0]["vector"] = json!([0, 0, 1]);
    let m: Model = serde_json::from_value(v).unwrap();
    assert_eq!(build(&m).err().unwrap().code, "DATUM_DEGENERATE");
}
fn check() -> Value {
    json!({"kind":"interface_compatibility","a":{"instance":"base","interface":"mount"},"b":{"instance":"motor","interface":"mount"},"plane_relation":"opposed","correspondence":{"kind":"pattern","require_all":true,"symmetry":"require_unique"},"tolerance":{"linear_mm":1e-5,"angular_rad":1e-7},"requirements":{"max_plane_gap_mm":0.02,"max_hole_axis_offset_mm":0.05,"hole_size":{"kind":"independent_ranges","a_diameter_mm":{"min":3.9,"max":4.1},"b_diameter_mm":{"min":3.9,"max":4.1}}}})
}
#[test]
fn compatibility_measures_actual_transformed_pattern_and_preserves_failure_details() {
    let mut v = fixture();
    v["checks"] = json!([check()]);
    let m: Model = serde_json::from_value(v.clone()).unwrap();
    let b = build(&m).unwrap();
    let r = &b.report["assembly"]["checks"][0];
    assert_eq!(r["passed"], true);
    assert_eq!(r["correspondence"].as_array().unwrap().len(), 4);
    // Both interfaces remain valid individually; a changed pitch makes the pair incompatible.
    v["features"][3]["positions"] = json!([[-10.1, -6], [10.1, -6], [-10.1, 6], [10.1, 6]]);
    v["interfaces"][1]["requirements"]["pattern"]["pitch_mm"] = json!([20.2, 12]);
    let m: Model = serde_json::from_value(v).unwrap();
    let e = build(&m).err().unwrap();
    assert_eq!(e.code, "INTERFACE_INCOMPATIBLE");
    assert!(e.details.unwrap().is_object());
}
fn registered_fixture() -> Value {
    let mut v = fixture();
    v["features"][1]["id"] = json!("aDrilled");
    v["features"][3]["id"] = json!("bDrilled");
    v["features"].as_array_mut().unwrap().extend([
  json!({"id":"a","op":"holes","input":"aDrilled","frame":{"origin":[0,0,-1],"x_axis":[1,0,0],"y_axis":[0,1,0]},"positions":[[0,0]],"radius":5,"depth":10}),
  json!({"id":"boss","op":"cylinder","origin":[0,0,-2],"radius":4.8,"height":2}),
  json!({"id":"b","op":"union","left":"bDrilled","right":"boss"})]);
    for (i, r, side) in [(0, 5., "internal"), (1, 4.8, "external")] {
        v["interfaces"][i]["selectors"]["plane"]["where"]["area_mm2"] = json!({"min":1000});
        v["interfaces"][i]["selectors"]["register"] = json!({"where":{"surface_type":"cylinder","radius_mm":{"min":r-1e-5,"max":r+1e-5}},"cardinality":{"min":1,"max":1},"material_side":side});
        v["interfaces"][i]["datum"]["origin"] =
            json!({"kind":"axis_plane_intersection","axis_slot":"register"});
    }
    let mut c = check();
    c["requirements"]["register"] =
        json!({"radial_clearance_mm":{"min":0.19,"max":0.21},"max_axis_offset_mm":0.01});
    v["checks"] = json!([c]);
    v
}
#[test]
fn internal_external_register_fit_is_measured_and_wrong_radius_or_normal_is_rejected() {
    let v = registered_fixture();
    let m: Model = serde_json::from_value(v.clone()).unwrap();
    let b = build(&m).unwrap();
    let r = &b.report["assembly"]["checks"][0]["register"];
    assert_eq!(r["passed"], true);
    assert!((r["radial_clearance_mm"].as_f64().unwrap() - 0.2).abs() < 1e-5);
    let mut bad = v.clone();
    bad["features"][5]["radius"] = json!(5.2);
    bad["interfaces"][1]["selectors"]["register"]["where"]["radius_mm"] =
        json!({"min":5.19999,"max":5.20001});
    let m: Model = serde_json::from_value(bad).unwrap();
    assert_eq!(build(&m).err().unwrap().code, "INTERFACE_INCOMPATIBLE");
    let mut bad = v;
    bad["mates"]["constraints"][0]["alignment"] = json!("aligned");
    let m: Model = serde_json::from_value(bad).unwrap();
    assert_eq!(build(&m).err().unwrap().code, "INTERFACE_INCOMPATIBLE");
}
#[test]
fn plane_scoped_recognition_keeps_unrelated_recesses_out_of_mount_completeness() {
    let mut v = fixture();
    v["features"][1]["id"] = json!("aDrilled");
    v["features"].as_array_mut().unwrap().extend([
        json!({"id":"recess","op":"cylinder","origin":[20,0,1],"radius":3,"height":3}),
        json!({"id":"a","op":"cut","left":"aDrilled","right":"recess"}),
    ]);
    v["interfaces"][0]["selectors"]["plane"]["where"]["area_mm2"] = json!({"min":1000});
    let m: Model = serde_json::from_value(v.clone()).unwrap();
    assert!(build(&m).is_ok());
    v["features"][4]["origin"] = json!([20, 0, -1]);
    v["features"][4]["height"] = json!(10);
    let m: Model = serde_json::from_value(v).unwrap();
    assert_eq!(build(&m).err().unwrap().code, "INTERFACE_UNSUPPORTED");
}
