use agent_cad::{build, Model};
use serde_json::json;
#[test]
fn names_follow_native_split_and_translation() {
    let m:Model=serde_json::from_value(json!({"version":1,"units":"mm","parameters":{},"features":[{"id":"stock","op":"box","origin":[0,0,0],"size":[10,10,10]},{"id":"named","op":"name_topology","input":"stock","faces":{"left":{"axis":"x","value":0,"tolerance":0.001,"expected_count":1}}},{"id":"tool","op":"box","origin":[-1,-1,4],"size":[12,12,2]},{"id":"split","op":"cut","left":"named","right":"tool"},{"id":"moved","op":"translate","input":"split","offset":[20,0,0]}],"output":"moved"})).unwrap();
    let b = build(&m).unwrap();
    assert_eq!(
        b.report["features"][3]["topology_names"]["named/left"]["faces"],
        2
    );
    assert_eq!(
        b.report["features"][4]["topology_names"]["named/left"]["faces"],
        2
    );
}
#[test]
fn named_edges_rebind_after_parameter_replay() {
    let mut m:Model=serde_json::from_value(json!({"version":1,"units":"mm","parameters":{"height":10},"features":[{"id":"stock","op":"box","origin":[0,0,0],"size":[20,20,{"param":"height"}]},{"id":"named","op":"name_topology","input":"stock","edges":{"rim":{"kind":"on_plane","axis":"z","value":{"param":"height"},"tolerance":0.001,"expected_count":4}}},{"id":"moved","op":"translate","input":"named","offset":[50,0,0]},{"id":"round","op":"fillet","input":"moved","select":{"kind":"named","name":"named/rim","expected_count":4},"radius":1}],"output":"round"})).unwrap();
    assert!(build(&m).is_ok());
    m.parameters.insert("height".into(), 15.);
    let b = build(&m).unwrap();
    assert!((b.report["geometry"]["bbox_mm"][1][2].as_f64().unwrap() - 15.).abs() < 1e-6);
}
#[test]
fn a_deleted_name_is_not_replaced_by_nearest_edge() {
    let m:Model=serde_json::from_value(json!({"version":1,"units":"mm","parameters":{},"features":[{"id":"stock","op":"box","origin":[0,0,0],"size":[10,10,10]},{"id":"named","op":"name_topology","input":"stock","edges":{"rim":{"kind":"on_plane","axis":"z","value":10,"tolerance":0.001,"expected_count":4}}},{"id":"tool","op":"box","origin":[-1,-1,5],"size":[12,12,6]},{"id":"cut","op":"cut","left":"named","right":"tool"},{"id":"round","op":"fillet","input":"cut","select":{"kind":"named","name":"named/rim","expected_count":4},"radius":1}],"output":"round"})).unwrap();
    let e = build(&m).err().unwrap();
    assert_eq!(e.code, "SELECTION_AMBIGUOUS");
}
#[test]
fn names_survive_independent_pattern_copies() {
    let m:Model=serde_json::from_value(json!({"version":1,"units":"mm","parameters":{},"features":[{"id":"stock","op":"box","origin":[0,0,0],"size":[10,10,10]},{"id":"named","op":"name_topology","input":"stock","faces":{"top":{"axis":"z","value":10,"tolerance":0.001,"expected_count":1}}},{"id":"copies","op":"pattern","input":"named","positions":[[0,0,0],[20,0,0]]}],"output":"copies"})).unwrap();
    let result = build(&m).unwrap();
    assert_eq!(
        result.report["features"][2]["topology_names"]["named/top"]["faces"],
        2
    );
}
#[test]
fn union_retains_names_from_both_operands() {
    let m:Model=serde_json::from_value(json!({"version":1,"units":"mm","parameters":{},"features":[{"id":"a","op":"box","origin":[0,0,0],"size":[10,10,10]},{"id":"left","op":"name_topology","input":"a","faces":{"top":{"axis":"z","value":10,"tolerance":0.001,"expected_count":1}}},{"id":"b","op":"box","origin":[5,0,0],"size":[10,10,10]},{"id":"right","op":"name_topology","input":"b","faces":{"top":{"axis":"z","value":10,"tolerance":0.001,"expected_count":1}}},{"id":"join","op":"union","left":"left","right":"right"}],"output":"join"})).unwrap();
    let result = build(&m).unwrap();
    let names = &result.report["features"][4]["topology_names"];
    assert!(names["left/top"]["faces"].as_u64().unwrap() > 0);
    assert!(names["right/top"]["faces"].as_u64().unwrap() > 0);
}
