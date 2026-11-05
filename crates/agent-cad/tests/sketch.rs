use agent_cad::{
    build,
    sketch::{solve, Sketch},
    Model,
};
use serde_json::json;
use std::collections::BTreeMap;
fn rectangle() -> serde_json::Value {
    json!({"points":{"a":[0.2,-0.3],"b":[39,0.4],"c":[38,21],"d":[-0.5,19]},"constraints":[{"id":"anchor","kind":"fixed","point":"a","at":[0,0]},{"id":"bottom","kind":"horizontal","a":"a","b":"b"},{"id":"right","kind":"vertical","a":"b","b":"c"},{"id":"top","kind":"horizontal","a":"c","b":"d"},{"id":"left","kind":"vertical","a":"d","b":"a"},{"id":"width","kind":"dx","a":"a","b":"b","value":{"param":"width"}},{"id":"height","kind":"dy","a":"a","b":"d","value":20}],"boundary":{"kind":"polygon","points":["a","b","c","d"]}})
}
#[test]
fn parametric_sketch_drives_exact_solid() {
    let s: Sketch = serde_json::from_value(rectangle()).unwrap();
    let p = BTreeMap::from([("width".into(), 40.)]);
    let result = solve(&s, &p).unwrap();
    assert_eq!(result.diagnostics["degrees_of_freedom"], 0);
    assert!((result.points["c"][0] - 40.).abs() < 1e-6);
    let mut m:Model=serde_json::from_value(json!({"version":1,"units":"mm","parameters":p,"features":[{"id":"part","op":"extrude","profile":{"kind":"sketch","sketch":s},"height":5}],"output":"part"})).unwrap();
    let a = build(&m).unwrap();
    assert!((a.report["geometry"]["volume_mm3"].as_f64().unwrap() - 4000.).abs() < 1e-3);
    m.parameters.insert("width".into(), 55.);
    let b = build(&m).unwrap();
    assert!((b.report["geometry"]["volume_mm3"].as_f64().unwrap() - 5500.).abs() < 1e-3);
}
#[test]
fn underconstraint_and_conflict_are_explicit() {
    let p = BTreeMap::from([("width".into(), 40.)]);
    let mut v = rectangle();
    v["constraints"].as_array_mut().unwrap().remove(0);
    let s: Sketch = serde_json::from_value(v.clone()).unwrap();
    assert_eq!(solve(&s, &p).err().unwrap().code, "SKETCH_UNDERCONSTRAINED");
    v["allow_underconstrained"] = json!(true);
    let s: Sketch = serde_json::from_value(v).unwrap();
    assert_eq!(solve(&s, &p).unwrap().diagnostics["degrees_of_freedom"], 2);
    let mut v = rectangle();
    v["constraints"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"conflict","kind":"dx","a":"a","b":"b","value":42}));
    let s: Sketch = serde_json::from_value(v).unwrap();
    let e = solve(&s, &p).err().unwrap();
    assert_eq!(e.code, "SKETCH_INCONSISTENT");
    assert!(e.message.contains("conflict"));
}
#[test]
fn circle_radius_and_tangency_solve() {
    let s:Sketch=serde_json::from_value(json!({"points":{"a":[-10,0],"b":[10,0],"center":[1,4]},"circles":{"hole":{"center":"center","radius":3}},"constraints":[{"id":"a","kind":"fixed","point":"a","at":[-10,0]},{"id":"b","kind":"fixed","point":"b","at":[10,0]},{"id":"x","kind":"dx","a":"a","b":"center","value":10},{"id":"r","kind":"radius","circle":"hole","value":5},{"id":"tangent","kind":"tangent","circle":"hole","line":["a","b"]}],"boundary":{"kind":"circle","circle":"hole"}})).unwrap();
    let r = solve(&s, &BTreeMap::new()).unwrap();
    assert!((r.points["center"][1] - 5.).abs() < 1e-6);
    assert!((r.radii["hole"] - 5.).abs() < 1e-6);
    assert_eq!(r.diagnostics["degrees_of_freedom"], 0);
}
