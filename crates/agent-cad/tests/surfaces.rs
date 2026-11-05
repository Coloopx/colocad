use agent_cad::{build, Model};
use serde_json::{json, Value};
fn model(features: Vec<Value>, output: &str) -> Model {
    serde_json::from_value(
        json!({"version":1,"units":"mm","parameters":{},"features":features,"output":output}),
    )
    .unwrap()
}
#[test]
fn planar_patch_thickens_to_exact_solid() {
    let m = model(
        vec![
            json!({"id":"patch","op":"surface_patch","control_points":[[[0,0,0],[0,10,0]],[[20,0,0],[20,10,0]]]}),
            json!({"id":"solid","op":"thicken","input":"patch","thickness":2}),
        ],
        "solid",
    );
    let b = build(&m).unwrap();
    assert!((b.report["geometry"]["volume_mm3"].as_f64().unwrap() - 400.).abs() < 1e-5);
}
#[test]
fn c2_bridge_has_matching_analytic_boundary_jets() {
    let a = json!({"id":"a","op":"surface_patch","control_points":[[[0,0,0],[0,10,0]],[[5,0,0],[5,10,0]],[[10,0,0],[10,10,0]]]});
    let b = json!({"id":"b","op":"surface_patch","control_points":[[[30,0,0],[30,10,0]],[[35,0,0],[35,10,0]],[[40,0,0],[40,10,0]]]});
    let m = model(
        vec![
            a,
            b,
            json!({"id":"bridge","op":"surface_bridge","start":"a","end":"b","start_scale":1,"end_scale":1}),
            json!({"id":"solid","op":"thicken","input":"bridge","thickness":2}),
        ],
        "solid",
    );
    let result = build(&m).unwrap();
    let t = &result.report["features"];
    for j in 0..2 {
        for field in ["position", "first_derivative", "second_derivative"] {
            for c in 0..3 {
                let x = t[0]["surface"]["u_end_jets"][j][field][c].as_f64().unwrap();
                let y = t[2]["surface"]["u_start_jets"][j][field][c]
                    .as_f64()
                    .unwrap();
                assert!((x - y).abs() < 1e-10);
                let x = t[1]["surface"]["u_start_jets"][j][field][c]
                    .as_f64()
                    .unwrap();
                let y = t[2]["surface"]["u_end_jets"][j][field][c].as_f64().unwrap();
                assert!((x - y).abs() < 1e-10);
            }
        }
    }
    assert!((result.report["geometry"]["volume_mm3"].as_f64().unwrap() - 400.).abs() < 1e-4);
}
#[test]
fn sew_six_surface_patches_forms_closed_box() {
    let patches = [
        [[0, 0, 0], [10, 0, 0], [0, 10, 0], [10, 10, 0]],
        [[0, 0, 5], [0, 10, 5], [10, 0, 5], [10, 10, 5]],
        [[0, 0, 0], [0, 0, 5], [10, 0, 0], [10, 0, 5]],
        [[0, 10, 0], [10, 10, 0], [0, 10, 5], [10, 10, 5]],
        [[0, 0, 0], [0, 10, 0], [0, 0, 5], [0, 10, 5]],
        [[10, 0, 0], [10, 0, 5], [10, 10, 0], [10, 10, 5]],
    ];
    let mut features=patches.iter().enumerate().map(|(i,p)|json!({"id":format!("patch-{i}"),"op":"surface_patch","control_points":[[p[0],p[1]],[p[2],p[3]]]})).collect::<Vec<_>>();
    features.push(json!({"id":"solid","op":"sew","inputs":(0..6).map(|i|format!("patch-{i}")).collect::<Vec<_>>()}));
    let b = build(&model(features, "solid")).unwrap();
    assert!((b.report["geometry"]["volume_mm3"].as_f64().unwrap() - 500.).abs() < 1e-5);
}
#[test]
fn curved_surface_and_control_edit_export_as_surface_model() {
    let source = json!({"version":1,"units":"mm","parameters":{"rise":3},"features":[{"id":"patch","op":"surface_patch","control_points":[[[0,0,0],[0,5,1],[0,10,0]],[[10,0,1],[10,5,{"param":"rise"}],[10,10,1]],[[20,0,0],[20,5,1],[20,10,0]]]},{"id":"edited","op":"surface_edit","input":"patch","edits":[{"u":1,"v":1,"point":[10,5,5]}]}],"output":"edited","output_kind":"surface","preview":{}});
    let m: Model = serde_json::from_value(source).unwrap();
    let b = build(&m).unwrap();
    assert!(b.step.contains("SHELL_BASED_SURFACE_MODEL"));
    assert!(b.step.contains("OPEN_SHELL"));
    assert_eq!(b.step.matches("ADVANCED_FACE(").count(), 1);
    assert_eq!(b.report["geometry"]["solids"], 0);
    assert_eq!(b.report["validation"]["manifold"], false);
    assert!(b.artifacts.contains_key("preview.png"));
}
#[test]
fn nonplanar_c2_bridge_preserves_scaled_jets_and_sheet_placement() {
    let mut source = serde_json::json!({"version":1,"units":"mm","parameters":{},"features":[
        {"id":"a","op":"surface_patch","control_points":[[[0,0,0],[0,5,2],[0,10,0]],[[5,0,1],[5,5,4],[5,10,1]],[[10,0,3],[10,5,6],[10,10,3]]]},
        {"id":"b","op":"surface_patch","control_points":[[[30,0,2],[30,5,5],[30,10,2]],[[35,0,1],[35,5,4],[35,10,1]],[[40,0,0],[40,5,2],[40,10,0]]]},
        {"id":"bridge","op":"surface_bridge","start":"a","end":"b","start_scale":0.75,"end_scale":1.25},
        {"id":"moved","op":"place","input":"bridge","translation":[3,5,7],"rotation_deg":[15,20,30]}
    ],"output":"moved","output_kind":"surface","preview":{}});
    let result = build(&serde_json::from_value(source.clone()).unwrap()).unwrap();
    let trace = &result.report["features"];
    for j in 0..3 {
        for (field, power) in [
            ("position", 0),
            ("first_derivative", 1),
            ("second_derivative", 2),
        ] {
            for c in 0..3 {
                let a = trace[0]["surface"]["u_end_jets"][j][field][c]
                    .as_f64()
                    .unwrap()
                    * 0.75_f64.powi(power);
                let b = trace[2]["surface"]["u_start_jets"][j][field][c]
                    .as_f64()
                    .unwrap();
                assert!((a - b).abs() < 1e-9);
                let a = trace[1]["surface"]["u_start_jets"][j][field][c]
                    .as_f64()
                    .unwrap()
                    * 1.25_f64.powi(power);
                let b = trace[2]["surface"]["u_end_jets"][j][field][c]
                    .as_f64()
                    .unwrap();
                assert!((a - b).abs() < 1e-9);
            }
        }
    }
    assert!(result.artifacts.contains_key("preview.png"));
    source["features"].as_array_mut().unwrap().pop();
    source["features"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"solid","op":"thicken","input":"bridge","thickness":0.5}));
    source["output"] = json!("solid");
    source["output_kind"] = json!("solid");
    source.as_object_mut().unwrap().remove("preview");
    let solid = build(&serde_json::from_value(source).unwrap()).unwrap();
    assert!(solid.report["geometry"]["volume_mm3"].as_f64().unwrap() > 90.);
}
