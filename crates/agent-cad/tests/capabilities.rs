use agent_cad::{build, validate, Model};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
fn m(features: Value, output: &str) -> Model {
    serde_json::from_value(
        json!({"version":1,"units":"mm","parameters":{},"features":features,"output":output}),
    )
    .unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < b.abs().max(1.) * 2e-5, "{a} != {b}");
}
fn volume(m: &Model) -> f64 {
    build(m).unwrap().report["geometry"]["volume_mm3"]
        .as_f64()
        .unwrap()
}
fn box_feature() -> Value {
    json!({"op":"box","id":"base","origin":[0,0,0],"size":[10,10,10]})
}
fn frame_z(z: f64) -> Value {
    json!({"origin":[0,0,z],"x_axis":[1,0,0],"y_axis":[0,1,0]})
}

#[test]
fn enclosure_and_fitting_lid_have_exact_wall_and_floor() {
    let profile = json!({"kind":"rectangle","width":20,"depth":16});
    for open in ["top", "bottom"] {
        let shell = m(
            json!([{"op":"shell_body","id":"x","profile":profile,"height":10,"wall":2,"floor":3,"open_face":open}]),
            "x",
        );
        close(volume(&shell), 3200. - 16. * 12. * 7.);
    }
    let lid = m(
        json!([{"op":"lid_for","id":"x","profile":profile,"wall":2,"thickness":1.5,"lip_height":3,"clearance":0.2}]),
        "x",
    );
    close(
        volume(&lid),
        20. * 16. * 1.5 + (15.6 * 11.6 - 11.6 * 7.6) * 3.,
    );
    let round = m(
        json!([{"op":"shell_body","id":"x","profile":{"kind":"rectangle","width":20,"depth":16,"radius":4},"height":10,"wall":2,"floor":3,"open_face":"top"}]),
        "x",
    );
    close(
        volume(&round),
        (320. - (4. - std::f64::consts::PI) * 16.) * 10.
            - (192. - (4. - std::f64::consts::PI) * 4.) * 7.,
    );
    let circle = m(
        json!([{"op":"shell_body","id":"x","profile":{"kind":"circle","radius":10},"height":10,"wall":2,"floor":3,"open_face":"top"}]),
        "x",
    );
    close(volume(&circle), std::f64::consts::PI * (1000. - 64. * 7.));
}
#[test]
fn oriented_openings_counterbores_and_split() {
    let fr = json!({"origin":[0,5,5],"x_axis":[0,1,0],"y_axis":[0,0,1]});
    let holes = m(
        json!([box_feature(),{"op":"holes","id":"x","input":"base","frame":fr,"positions":[[0,0]],"radius":1,"depth":10,"counterbore":{"radius":2,"depth":2}}]),
        "x",
    );
    close(volume(&holes), 1000. - std::f64::consts::PI * 16.);
    let window = m(
        json!([box_feature(),{"op":"cutout","id":"x","input":"base","frame":fr,"profile":{"kind":"rectangle","width":4,"depth":2},"depth":10}]),
        "x",
    );
    close(volume(&window), 920.);
    for (keep, v) in [(true, 600.), (false, 400.)] {
        let part = m(
            json!([box_feature(),{"op":"split","id":"x","input":"base","axis":"z","value":4,"keep_positive":keep}]),
            "x",
        );
        close(volume(&part), v);
    }
    let bad = m(
        json!([box_feature(),{"op":"cutout","id":"x","input":"base","frame":frame_z(20.),"profile":{"kind":"circle","radius":1},"depth":2}]),
        "x",
    );
    assert!(build(&bad).is_err());
}

#[test]
fn rigid_placement_and_mirror_preserve_volume() {
    let x = m(
        json!([{"op":"box","id":"base","origin":[0,0,0],"size":[2,4,6]},
 {"op":"place","id":"placed","input":"base","translation":[10,20,30],"rotation_deg":[0,0,90]},
 {"op":"mirror","id":"mirror","input":"placed","origin":[0,0,0],"normal":[1,0,0]}]),
        "mirror",
    );
    let b = build(&x).unwrap();
    close(b.report["geometry"]["volume_mm3"].as_f64().unwrap(), 48.);
    for (i, expect) in [[-10., 20., 30.], [-6., 22., 36.]].iter().enumerate() {
        for (j, n) in expect.iter().enumerate() {
            close(b.report["geometry"]["bbox_mm"][i][j].as_f64().unwrap(), *n);
        }
    }
}
#[test]
fn polygon_and_rounded_profiles_extrude_exactly() {
    let x = m(
        json!([{"op":"extrude","id":"x","profile":{"kind":"polygon","points":[[0,0],[8,0],[8,2],[2,2],[2,6],[0,6]]},"height":5}]),
        "x",
    );
    close(volume(&x), 120.);
    let r = m(
        json!([{"op":"extrude","id":"x","profile":{"kind":"rectangle","width":10,"depth":8,"radius":1},"height":5}]),
        "x",
    );
    close(volume(&r), (80. - 4. + std::f64::consts::PI) * 5.);
    let hole = m(
        json!([{"op":"extrude","id":"x","profile":{"kind":"circle","radius":10},"holes":[{"kind":"circle","radius":2}],"height":5}]),
        "x",
    );
    close(volume(&hole), std::f64::consts::PI * 96. * 5.);
}
#[test]
fn exact_arc_outline_is_closed_and_editable() {
    let x = m(
        json!([{"op":"extrude","id":"x","profile":{"kind":"outline","start":[-3,-2],"segments":[{"kind":"line","to":[3,-2]},{"kind":"arc","via":[5,0],"to":[3,2]},{"kind":"line","to":[-3,2]},{"kind":"arc","via":[-5,0],"to":[-3,-2]}]},"height":3}]),
        "x",
    );
    close(volume(&x), (24. + 4. * std::f64::consts::PI) * 3.);
}
#[test]
fn revolve_uses_explicit_frame_and_angle() {
    let x = m(
        json!([{"op":"revolve","id":"x","profile":{"kind":"polygon","points":[[2,0],[4,0],[4,5],[2,5]]},"frame":{"origin":[0,0,0],"x_axis":[1,0,0],"y_axis":[0,0,1]},"axis_origin":[0,0,0],"axis":[0,0,1],"angle_deg":360}]),
        "x",
    );
    close(volume(&x), std::f64::consts::PI * 60.);
}
#[test]
fn edge_selection_fillet_and_chamfer_match_analytic_volumes() {
    let select = json!({"kind":"parallel","axis":[0,0,1],"expected_count":4});
    let f = m(
        json!([box_feature(),{"op":"fillet","id":"x","input":"base","select":select,"radius":1}]),
        "x",
    );
    close(volume(&f), (100. - 4. + std::f64::consts::PI) * 10.);
    let c = m(
        json!([box_feature(),{"op":"chamfer","id":"x","input":"base","select":select,"length":1}]),
        "x",
    );
    close(volume(&c), 980.);
    let mismatch = m(
        json!([box_feature(),{"op":"fillet","id":"x","input":"base","select":{"kind":"parallel","axis":[0,0,1],"expected_count":3},"radius":1}]),
        "x",
    );
    assert_eq!(build(&mismatch).err().unwrap().code, "SELECTION_AMBIGUOUS");
}
#[test]
fn shell_open_face_keeps_external_envelope() {
    let x = m(
        json!([{"op":"box","id":"base","origin":[0,0,0],"size":[20,16,10]},
 {"op":"shell","id":"x","input":"base","thickness":2,"open_faces":[{"axis":"z","value":10,"tolerance":0.001,"expected_count":1}]}]),
        "x",
    );
    close(volume(&x), 1664.);
}
#[test]
fn boss_blind_hole_and_disconnected_pattern() {
    let x = m(
        json!([{"op":"boss","id":"base","origin":[0,0,0],"outer_radius":4,"height":10,"hole_radius":1,"hole_depth":6},
 {"op":"pattern","id":"x","input":"base","positions":[[0,0,0],[20,0,0]]}]),
        "x",
    );
    close(volume(&x), 2. * std::f64::consts::PI * 154.);
}
#[test]
fn pipe_and_profile_sweep_are_capped_solids() {
    let x = m(
        json!([{"op":"pipe","id":"x","path":{"kind":"line","start":[0,0,0],"end":[0,0,10]},"radius":2}]),
        "x",
    );
    close(volume(&x), 40. * std::f64::consts::PI);
    let y = m(
        json!([{"op":"sweep","id":"x","profile":{"kind":"rectangle","width":2,"depth":2},"frame":frame_z(0.),"path":[{"kind":"line","start":[0,0,0],"end":[0,0,10]}]}]),
        "x",
    );
    close(volume(&y), 40.);
}
#[test]
fn arbitrary_profile_sections_loft() {
    let x = m(
        json!([{"op":"profile_loft","id":"x","degree":1,"sections":[{"profile":{"kind":"rectangle","width":10,"depth":8},"frame":frame_z(0.)},{"profile":{"kind":"rectangle","width":10,"depth":8},"frame":frame_z(10.)}]}]),
        "x",
    );
    close(volume(&x), 800.);
}
#[test]
fn step_assets_roundtrip_and_reject_changed_content() {
    let step = build(&m(json!([box_feature()]), "base")).unwrap().step;
    let mut x = m(json!([{"op":"import_step","id":"x","asset":"part"}]), "x");
    x.assets.insert(
        "part".into(),
        agent_cad::StepAsset {
            sha256: format!("{:x}", Sha256::digest(step.as_bytes())),
            step,
        },
    );
    close(volume(&x), 1000.);
    x.assets.get_mut("part").unwrap().step.push(' ');
    assert_eq!(validate(&x).unwrap_err().code, "ASSET_DIGEST_MISMATCH");
}
#[test]
fn assembly_preserves_parts_and_refuses_collision() {
    let mut v = json!({"version":1,"units":"mm","parameters":{},"features":[box_feature()],"assembly":[{"id":"left","input":"base"},{"id":"right","input":"base","translation":[20,0,0]}],"checks":[{"kind":"collision","parts":["left","right"],"max_overlap_mm3":0}]});
    let x: Model = serde_json::from_value(v.clone()).unwrap();
    let b = build(&x).unwrap();
    close(b.report["geometry"]["volume_mm3"].as_f64().unwrap(), 2000.);
    assert_eq!(
        b.report["assembly"]["instances"].as_array().unwrap().len(),
        2
    );
    let imported = step_import::import(b.step.as_bytes(), &Default::default()).unwrap();
    assert_eq!(imported.bodies.len(), 1); // Shared definition, two native product instances.
    assert_eq!(imported.assembly_instances, 2);
    assert_eq!(b.report["geometry"]["solids"], 2);
    v["assembly"][1]["translation"] = json!([5, 0, 0]);
    let x: Model = serde_json::from_value(v).unwrap();
    assert_eq!(build(&x).err().unwrap().code, "ASSEMBLY_COLLISION");
}
#[test]
fn construction_volume_containment_is_checked_but_not_exported() {
    let v = json!({"version":1,"units":"mm","parameters":{},"features":[box_feature(),{"op":"box","id":"small","origin":[2,2,2],"size":[2,2,2]}],"assembly":[{"id":"space","input":"base","export":false},{"id":"part","input":"small"}],"checks":[{"kind":"containment","container":"space","targets":["part"],"max_outside_mm3":0}]});
    let x: Model = serde_json::from_value(v).unwrap();
    close(volume(&x), 8.);
}

#[test]
fn spatial_sweep_and_curved_pipe() {
    let x = m(
        json!([{"op":"curve_sweep","id":"x","profile":{"kind":"rectangle","width":2,"depth":2},"frame":frame_z(0.),"path":{"kind":"bezier","control_points":[[0,0,0],[0,0,5],[5,0,10],[5,5,15]]},"translational":true}]),
        "x",
    );
    close(volume(&x), 60.);
    let pipe = m(
        json!([{"op":"pipe","id":"x","path":{"kind":"arc","start":[10,0,0],"via":[0,10,0],"end":[-10,0,0]},"radius":1}]),
        "x",
    );
    close(volume(&pipe), 10. * std::f64::consts::PI.powi(2));
}

#[test]
fn opening_distance_and_overlap_are_enforced() {
    let mut v = json!({"version":1,"units":"mm","parameters":{},"features":[box_feature()],"assembly":[{"id":"a","input":"base","export":false},{"id":"b","input":"base","translation":[12,0,0]}],"checks":[{"kind":"opening","opening":"a","target":"b","max_distance_mm":2.01,"min_overlap_mm3":0}]});
    let b = build(&serde_json::from_value(v.clone()).unwrap()).unwrap();
    close(
        b.report["assembly"]["checks"][0]["distance_mm"]
            .as_f64()
            .unwrap(),
        2.,
    );
    v["checks"][0]["max_distance_mm"] = json!(1.9);
    assert_eq!(
        build(&serde_json::from_value(v.clone()).unwrap())
            .err()
            .unwrap()
            .code,
        "ASSEMBLY_OPENING"
    );
    v["assembly"][1]["translation"] = json!([9, 0, 0]);
    v["checks"][0]["max_distance_mm"] = json!(0);
    v["checks"][0]["min_overlap_mm3"] = json!(99.9);
    assert!(build(&serde_json::from_value(v).unwrap()).is_ok());
}

#[test]
fn partial_revolution_exports_canonical_cones() {
    let source: Model =
        serde_json::from_str(include_str!("../../../examples/v0.2/revolve.json")).unwrap();
    let built = build(&source).unwrap();
    close(
        built.report["geometry"]["volume_mm3"].as_f64().unwrap(),
        1156. * std::f64::consts::PI * 0.75,
    );
    let imported = step_import::import(built.step.as_bytes(), &Default::default()).unwrap();
    let angles = imported.bodies[0]
        .faces
        .iter()
        .filter_map(|f| {
            if let step_import::model::Surface::Cone { semi_angle_deg, .. } = &f.surface {
                Some(*semi_angle_deg)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(angles.len(), 2);
    assert!(angles.iter().all(|a| *a > 0.));
}
#[test]
fn opening_distance_on_curved_surfaces_is_geometric() {
    let source:Model=serde_json::from_value(json!({"version":1,"units":"mm","parameters":{},"features":[{"op":"sphere","id":"ball","center":[0,0,0],"radius":2}],"assembly":[{"id":"a","input":"ball"},{"id":"b","input":"ball","translation":[6,0,0]}],"checks":[{"kind":"opening","opening":"a","target":"b","max_distance_mm":2.0001,"min_overlap_mm3":0}]})).unwrap();
    let b = build(&source).unwrap();
    close(
        b.report["assembly"]["checks"][0]["distance_mm"]
            .as_f64()
            .unwrap(),
        2.,
    );
}
