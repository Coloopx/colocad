use agent_cad::{build, Model};
use serde_json::{json, Value};
fn m(v: Value) -> Model {
    serde_json::from_value(v).unwrap()
}
#[test]
fn requested_degradation_is_recorded_but_never_implicit() {
    let mut v = json!({"version":1,"units":"mm","parameters":{},"features":[{"id":"box","op":"box","origin":[0,0,0],"size":[10,10,10]},{"id":"round","op":"fillet","input":"box","radius":50,"select":{"kind":"all"}}],"output":"round"});
    assert!(build(&m(v.clone())).is_err());
    v["features"][1]["on_fail"] = json!({"kind":"record","reason":"optional cosmetic edge treatment","omitted":["50 mm fillet"],"affects":["appearance"]});
    let b = build(&m(v)).unwrap();
    assert!(!b.report["features"][1]["degradation"]["error"].is_null());
    assert!((b.report["geometry"]["volume_mm3"].as_f64().unwrap() - 1000.).abs() < 1e-6);
}
#[test]
fn tapered_extrusion_matches_frustum_volume() {
    let h = 10.;
    let t = 20. - 2. * h * 5_f64.to_radians().tan();
    let source = m(
        json!({"version":1,"units":"mm","parameters":{},"features":[{"id":"draft","op":"draft_extrude","profile":{"kind":"rectangle","width":20,"depth":20},"height":h,"angle_deg":5}],"output":"draft"}),
    );
    let b = build(&source).unwrap();
    let expected = h * (400. + 20. * t + t * t) / 3.;
    assert!(
        (b.report["geometry"]["volume_mm3"].as_f64().unwrap() - expected).abs() < expected * 1e-6
    );
}
#[test]
fn exploded_visibility_does_not_hide_collision() {
    let source = m(
        json!({"version":1,"units":"mm","parameters":{},"features":[{"id":"box","op":"box","origin":[0,0,0],"size":[10,10,10]}],"assembly":[{"id":"a","input":"box"},{"id":"b","input":"box","poses":{"normal":{},"internal":{"visible":false},"exploded":{"translation":[100,0,0]}}}],"checks":[{"kind":"collision","parts":["a","b"],"max_overlap_mm3":0}],"preview":{"pose":"exploded"}}),
    );
    assert_eq!(build(&source).err().unwrap().code, "ASSEMBLY_COLLISION");
}
#[test]
fn step_structure_emits_replayable_nested_model() {
    use sha2::{Digest, Sha256};
    let step = include_str!("fixtures/nested-assembly.step");
    let asset = agent_cad::StepAsset {
        sha256: format!("{:x}", Sha256::digest(step)),
        step: step.into(),
    };
    let info = agent_cad::step_assembly::inspect(&asset).unwrap();
    assert_eq!(info["definitions"], 1);
    let mut model: Model = serde_json::from_value(info["model_template"].clone()).unwrap();
    model.assets.insert("source".into(), asset);
    let b = build(&model).unwrap();
    assert_eq!(b.report["geometry"]["solids"], 2);
    assert!((b.report["geometry"]["volume_mm3"].as_f64().unwrap() - 2000.).abs() < 1e-6);
}
#[test]
fn face_grids_cut_each_of_the_six_directions() {
    for face in ["pos_x", "neg_x", "pos_y", "neg_y", "pos_z", "neg_z"] {
        let source = m(
            json!({"version":1,"units":"mm","parameters":{},"features":[{"id":"stock","op":"box","origin":[0,0,0],"size":[20,20,20]},{"id":"grid","op":"face_hole_grid","input":"stock","face":face,"center":[10,10],"count":[3,2],"spacing":[5,5],"radius":1,"depth":21}],"output":"grid"}),
        );
        let b = build(&source).unwrap();
        assert_eq!(b.report["geometry"]["faces"], 12);
        assert!(
            (b.report["geometry"]["volume_mm3"].as_f64().unwrap()
                - (8000. - 120. * std::f64::consts::PI))
                .abs()
                < 8000. * 1e-6,
            "{face}"
        );
    }
}
#[test]
fn face_cutting_tool_has_same_effect_as_direct_cut() {
    let features = json!([{"id":"stock","op":"box","origin":[0,0,0],"size":[20,20,20]},{"id":"tool","op":"face_tool","input":"stock","face":"neg_x","center":[10,10],"profile":{"kind":"circle","radius":2},"depth":5},{"id":"cut","op":"cut","left":"stock","right":"tool"}]);
    let b = build(&m(
        json!({"version":1,"units":"mm","parameters":{},"features":features,"output":"cut"}),
    ))
    .unwrap();
    assert!(
        (b.report["geometry"]["volume_mm3"].as_f64().unwrap()
            - (8000. - 4. * std::f64::consts::PI * 5.0001))
            .abs()
            < 1e-5
    );
}
#[test]
fn shell_rim_radius_is_applied_and_bounded_by_wall() {
    let mut source = json!({"version":1,"units":"mm","parameters":{},"features":[{"id":"shell","op":"shell_body","profile":{"kind":"rectangle","width":40,"depth":30,"radius":3},"height":20,"wall":2,"floor":2,"open_face":"top","edge_radius":0.5}],"output":"shell"});
    let rounded = build(&m(source.clone())).unwrap();
    source["features"][0]["edge_radius"] = json!(0);
    let plain = build(&m(source.clone())).unwrap();
    assert!(
        rounded.report["geometry"]["volume_mm3"].as_f64().unwrap()
            < plain.report["geometry"]["volume_mm3"].as_f64().unwrap()
    );
    source["features"][0]["edge_radius"] = json!(2);
    assert!(agent_cad::validate(&m(source)).is_err());
}
#[test]
fn all_hidden_preview_is_an_explicit_error() {
    let source = m(
        json!({"version":1,"units":"mm","parameters":{},"features":[{"id":"box","op":"box","origin":[0,0,0],"size":[1,1,1]}],"assembly":[{"id":"box","input":"box","poses":{"normal":{"visible":false},"internal":{},"exploded":{}}}],"preview":{}}),
    );
    assert_eq!(build(&source).err().unwrap().code, "PREVIEW_EMPTY");
}
