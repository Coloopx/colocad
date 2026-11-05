use agent_cad::{build, Model};
use serde_json::{json, Value};
fn shell() -> Value {
    json!({"op":"shell_body","id":"shell","profile":{"kind":"rectangle","width":40,"depth":30,"radius":3},"height":20,"wall":2,"floor":2,"open_face":"top"})
}
fn model(f: Vec<Value>, output: &str) -> Model {
    serde_json::from_value(
        json!({"version":1,"units":"mm","parameters":{},"features":f,"output":output}),
    )
    .unwrap()
}
fn reconstruct(mut features: Vec<Value>, id: &str, openings: usize) -> Model {
    features.extend([json!({"id":"outer","op":"inspection_region","input":id,"region":"outer","require_exact":true}),json!({"id":"cavity","op":"inspection_region","input":id,"region":"cavity","require_exact":true}),json!({"id":"wall","op":"cut","left":"outer","right":"cavity"})]);
    let mut current = "wall".to_string();
    for i in 0..openings {
        let op = format!("opening-{i}");
        let next = format!("subtract-{i}");
        features.push(json!({"id":op,"op":"inspection_region","input":id,"region":"opening","index":i,"require_exact":true}));
        features.push(json!({"id":next,"op":"cut","left":current,"right":op}));
        current = next;
    }
    model(features, &current)
}
fn same_geometry(a: &Model, b: &Model) {
    let a = build(a).unwrap();
    let b = build(b).unwrap();
    let va = a.report["geometry"]["volume_mm3"].as_f64().unwrap();
    let vb = b.report["geometry"]["volume_mm3"].as_f64().unwrap();
    assert!((va - vb).abs() < va * 1e-6, "{va}, {vb}");
    for i in 0..2 {
        for j in 0..3 {
            assert!(
                (a.report["geometry"]["bbox_mm"][i][j].as_f64().unwrap()
                    - b.report["geometry"]["bbox_mm"][i][j].as_f64().unwrap())
                .abs()
                    < 1e-5
            );
        }
    }
}
#[test]
fn shell_regions_reconstruct_after_face_cut_join_and_placement() {
    let features = vec![
        shell(),
        json!({"id":"port","op":"face_cutout","input":"shell","face":"pos_y","center":[0,10],"profile":{"kind":"rectangle","width":10,"depth":4},"depth":3}),
        json!({"id":"boss","op":"boss","origin":[12,7,1],"outer_radius":3,"height":7,"hole_radius":1,"hole_depth":4}),
        json!({"id":"joined","op":"union","left":"port","right":"boss"}),
        json!({"id":"placed","op":"place","input":"joined","translation":[10,0,2],"rotation_deg":[0,0,90]}),
    ];
    same_geometry(
        &model(features.clone(), "placed"),
        &reconstruct(features, "placed", 2),
    );
}
#[test]
fn blind_recess_does_not_become_an_opening() {
    let features = vec![
        shell(),
        json!({"id":"recess","op":"face_cutout","input":"shell","face":"pos_y","center":[0,10],"profile":{"kind":"rectangle","width":10,"depth":4},"depth":0.5}),
    ];
    same_geometry(
        &model(features.clone(), "recess"),
        &reconstruct(features, "recess", 1),
    );
}
#[test]
fn split_regions_reconstruct_and_extend_to_new_open_face() {
    let features = vec![
        shell(),
        json!({"id":"half","op":"split","input":"shell","axis":"z","value":10,"keep_positive":false}),
    ];
    same_geometry(
        &model(features.clone(), "half"),
        &reconstruct(features, "half", 1),
    );
}
#[test]
fn opening_reaches_internal_hardware() {
    let mut features = vec![
        shell(),
        json!({"id":"port","op":"face_cutout","input":"shell","face":"pos_y","center":[0,10],"profile":{"kind":"rectangle","width":10,"depth":4},"depth":3}),
        json!({"id":"opening","op":"inspection_region","input":"port","region":"opening","index":1}),
        json!({"id":"target","op":"box","origin":[-2,0,9],"size":[4,2,2]}),
    ];
    features.push(json!({"id":"touch","op":"intersect","left":"opening","right":"target"}));
    let b = build(&model(features, "touch")).unwrap();
    assert!((b.report["geometry"]["volume_mm3"].as_f64().unwrap() - 16.).abs() < 1e-5);
}
