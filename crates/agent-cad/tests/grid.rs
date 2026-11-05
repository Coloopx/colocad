use agent_cad::{build, Model};
use serde_json::json;
#[test]
fn large_grid_has_all_1024_holes_with_exact_volume() {
    let m:Model=serde_json::from_value(json!({"version":1,"units":"mm","parameters":{},"features":[{"op":"box","id":"stock","origin":[0,0,0],"size":[100,100,2]},{"op":"hole_grid","id":"holes","input":"stock","origin":[3,3,-1],"radius":0.5,"depth":4,"count":[32,32],"spacing":[3,3]}],"output":"holes"})).unwrap();
    let b = build(&m).unwrap();
    let expected = 20_000. - 1024. * std::f64::consts::PI * 0.5;
    assert!(
        (b.report["geometry"]["volume_mm3"].as_f64().unwrap() - expected).abs() < expected * 1e-6
    );
    assert_eq!(b.report["geometry"]["faces"], 1030);
}
