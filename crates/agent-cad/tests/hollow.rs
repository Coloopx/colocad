use agent_cad::{build, Model};
use serde_json::json;
#[test]
fn enclosed_cavity_exports_as_one_solid_with_two_shells() {
    let source:Model=serde_json::from_value(json!({"version":1,"units":"mm","parameters":{},"features":[{"id":"box","op":"box","origin":[0,0,0],"size":[20,20,20]},{"id":"hollow","op":"shell","input":"box","thickness":2}],"output":"hollow"})).unwrap();
    let b = build(&source).unwrap();
    assert_eq!(b.report["geometry"]["solids"], 1);
    assert!((b.report["geometry"]["volume_mm3"].as_f64().unwrap() - 3904.).abs() < 1e-5);
    let m = step_import::import(b.step.as_bytes(), &Default::default()).unwrap();
    assert_eq!(m.bodies.len(), 1);
    assert_eq!(m.bodies[0].shells.len(), 2);
    assert!(b.step.contains("BREP_WITH_VOIDS"));
    assert!(!m.bodies[0].shells[1].orientation);
    use sha2::{Digest, Sha256};
    let sha = format!("{:x}", Sha256::digest(&b.step));
    let imported:Model=serde_json::from_value(json!({"version":1,"units":"mm","parameters":{},"assets":{"source":{"step":b.step,"sha256":sha}},"features":[{"id":"part","op":"import_step","asset":"source"}],"output":"part"})).unwrap();
    let roundtrip = build(&imported).unwrap();
    assert_eq!(roundtrip.report["geometry"]["solids"], 1);
    assert!((roundtrip.report["geometry"]["volume_mm3"].as_f64().unwrap() - 3904.).abs() < 1e-4);
}
#[test]
fn scaled_closed_cavity_is_explicit_and_wall_is_measured() {
    let source:Model=serde_json::from_value(json!({"version":1,"units":"mm","parameters":{},"features":[{"id":"box","op":"box","origin":[0,0,0],"size":[20,30,40]},{"id":"hollow","op":"hollow","input":"box","wall":2,"cavity_strategy":"scaled"}],"output":"hollow"})).unwrap();
    let b = build(&source).unwrap();
    let h = &b.report["features"][1]["hollow"];
    assert_eq!(h["cavity_method"], "affine_scaled_cavity");
    assert_eq!(h["variable_wall"], true);
    assert!((h["minimum_wall_mm"].as_f64().unwrap() - 2.).abs() < 1e-5);
    assert!(
        (b.report["geometry"]["volume_mm3"].as_f64().unwrap() - (24000. - 16. * 26. * 36.)).abs()
            < 1e-4
    );
}
#[test]
fn multiple_internal_voids_remain_one_material_solid() {
    let source:Model=serde_json::from_value(json!({"version":1,"units":"mm","parameters":{},"features":[{"id":"box","op":"box","origin":[0,0,0],"size":[30,20,20]},{"id":"cavity","op":"box","origin":[2,2,2],"size":[8,16,16]},{"id":"cavities","op":"pattern","input":"cavity","positions":[[0,0,0],[18,0,0]]},{"id":"result","op":"cut","left":"box","right":"cavities"}],"output":"result"})).unwrap();
    let b = build(&source).unwrap();
    assert_eq!(b.report["geometry"]["solids"], 1);
    let m = step_import::import(b.step.as_bytes(), &Default::default()).unwrap();
    assert_eq!(m.bodies.len(), 1);
    assert_eq!(m.bodies[0].shells.len(), 3);
}
#[test]
fn scaled_concave_cavity_is_refused_when_it_protrudes() {
    let source:Model=serde_json::from_value(json!({"version":1,"units":"mm","parameters":{},"features":[{"id":"stock","op":"extrude","profile":{"kind":"polygon","points":[[0,0],[30,0],[30,10],[10,10],[10,30],[0,30]]},"height":20},{"id":"hollow","op":"hollow","input":"stock","wall":2,"cavity_strategy":"scaled"}],"output":"hollow"})).unwrap();
    assert_eq!(build(&source).err().unwrap().code, "HOLLOW_OUTSIDE");
}
