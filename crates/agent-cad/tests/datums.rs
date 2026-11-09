use agent_cad::{build, Model};
use serde_json::{json, Value};
fn face(normal: [i32; 3]) -> Value {
    json!({"kind":"filter","input":{"kind":"entities","entity":"face"},"where":{"planar_normal_aligned_with":normal}})
}
fn selected(id: &str, definition: &str, kind: &str, selection: Value) -> Value {
    json!({"id":id,"definition":definition,"tolerance":{"linear_mm":1e-5,"angular_rad":1e-7},"derive":{"kind":"selected","selection":selection,"derive":{"kind":kind}}})
}
fn fixture() -> Value {
    json!({"version":1,"units":"mm","parameters":{"height":8},"features":[
      {"id":"base","op":"box","origin":[-15,-15,0],"size":[30,30,{"param":"height"}]},
      {"id":"support","op":"holes","input":"base","positions":[[0,0]],"radius":5.2,"frame":{"origin":[0,0,-1],"x_axis":[1,0,0],"y_axis":[0,1,0]},"depth":30},
      {"id":"shaft","op":"cylinder","origin":[0,0,0],"radius":5,"height":20}],
      "assembly":[{"id":"support_occ","input":"support","translation":[25,15,2],"rotation_deg":[0,25,10]},{"id":"shaft_occ","input":"shaft","translation":[26,14,4],"rotation_deg":[0,20,15]}],
      "datums":[selected("bore","support","axis",json!({"kind":"filter","input":{"kind":"entities","entity":"face"},"where":{"surface_type":"cylinder"}})),selected("top","support","plane",face([0,0,1])),selected("axis","shaft","axis",json!({"kind":"filter","input":{"kind":"entities","entity":"face"},"where":{"surface_type":"cylinder"}})),selected("bottom","shaft","plane",face([0,0,-1]))],
      "mates":{"anchors":["support_occ"],"expected_degrees_of_freedom":1,"constraints":[
        {"id":"axis","kind":"concentric","alignment":"unoriented","a":{"instance":"support_occ","datum":"bore"},"b":{"instance":"shaft_occ","datum":"axis"}},
        {"id":"seat","kind":"plane","distance":0,"opposed":true,"a":{"instance":"support_occ","datum":"top"},"b":{"instance":"shaft_occ","datum":"bottom"}}]}})
}
#[test]
fn derived_axis_and_plane_drive_mates_and_recompute_after_parameter_edit() {
    for height in [8., 12.] {
        let mut v = fixture();
        v["parameters"]["height"] = json!(height);
        let m: Model = serde_json::from_value(v).unwrap();
        let b = build(&m).unwrap();
        assert!(b.artifacts.contains_key("query-manifest.json"));
        let r = &b.report["assembly"]["mate_solution"];
        assert_eq!(r["degrees_of_freedom"], 1);
        assert!(r["max_residual"].as_f64().unwrap() < 1e-6);
        let poses = &r["placements"];
        let a = &poses["support_occ"];
        let p = &poses["shaft_occ"]["origin_mm"];
        for i in 0..3 {
            assert!(
                (p[i].as_f64().unwrap()
                    - a["origin_mm"][i].as_f64().unwrap()
                    - height * a["axes"][2][i].as_f64().unwrap())
                .abs()
                    < 1e-5
            );
        }
        let datum = &b.report["datums"]["support"]["top"];
        assert_eq!(datum["kind"], "plane");
        assert_eq!(
            datum["source_refs"][0]["query_revision_id"],
            b.report["query_revision_id"]
        );
    }
}
#[test]
fn ambiguous_datum_does_not_pick_first_face() {
    let mut v = fixture();
    v["datums"][1]["derive"]["selection"] = json!({"kind":"entities","entity":"face"});
    let m: Model = serde_json::from_value(v).unwrap();
    let error = build(&m).err().unwrap();
    assert_eq!(error.code, "SELECTION_CARDINALITY");
    assert_eq!(error.details.unwrap()["datum_id"], "top");
}
#[test]
fn incomplete_axis_cannot_be_used_as_fixed_frame_and_frame_datum_is_mutually_exclusive() {
    let mut v = fixture();
    v["mates"]["constraints"] = json!([{"id":"bad","kind":"fixed","a":{"instance":"support_occ","datum":"bore"},"b":{"instance":"shaft_occ","datum":"axis"}}]);
    let m: Model = serde_json::from_value(v.clone()).unwrap();
    assert_eq!(build(&m).err().unwrap().code, "DATUM_KIND_MISMATCH");
    v["mates"]["constraints"][0]["a"]["frame"] =
        json!({"origin":[0,0,0],"x_axis":[1,0,0],"y_axis":[0,1,0]});
    let m: Model = serde_json::from_value(v).unwrap();
    assert_eq!(build(&m).err().unwrap().code, "MATE_ATTACHMENT");
}

#[test]
fn frame_from_named_points_and_directions_fixes_six_dof_and_rejects_degeneracy() {
    let mut v = fixture();
    let tol = json!({"linear_mm":1e-5,"angular_rad":1e-7});
    let mut datums = vec![];
    for (def, origin) in [("support", json!([0, 0, 8])), ("shaft", json!([0, 0, 0]))] {
        datums.push(json!({"id":"origin","definition":def,"tolerance":tol,"derive":{"kind":"design_point","point":origin}}));
        datums.push(json!({"id":"toward_x","definition":def,"tolerance":tol,"derive":{"kind":"design_point","point":[10,0,origin[2]]}}));
        datums.push(json!({"id":"x","definition":def,"tolerance":tol,"derive":{"kind":"point_difference","from":"origin","to":"toward_x"}}));
        datums.push(selected("z", def, "plane", face([0, 0, 1])));
        datums.push(json!({"id":"mount","definition":def,"tolerance":tol,"derive":{"kind":"frame","origin":"origin","x":{"datum":"x","sense":"positive"},"z":{"datum":"z","sense":"positive"}}}));
    }
    v["datums"] = json!(datums);
    v["mates"]["expected_degrees_of_freedom"] = json!(0);
    v["mates"]["constraints"] = json!([{"id":"mount","kind":"fixed","a":{"instance":"support_occ","datum":"mount"},"b":{"instance":"shaft_occ","datum":"mount"}}]);
    let m: Model = serde_json::from_value(v.clone()).unwrap();
    let b = build(&m).unwrap();
    assert_eq!(
        b.report["assembly"]["mate_solution"]["degrees_of_freedom"],
        0
    );
    let frame = &b.report["datums"]["support"]["mount"];
    assert_eq!(frame["geometry"]["handedness"], "right");
    assert_eq!(frame["remaining_rigid_freedoms"], json!([]));
    v["datums"][4]["derive"]["x"]["datum"] = json!("z");
    let m: Model = serde_json::from_value(v).unwrap();
    assert_eq!(build(&m).err().unwrap().code, "DATUM_DEGENERATE");
}

#[test]
fn datum_recipe_rejects_forward_dependencies_and_explicit_snapshot_disable() {
    let mut v = fixture();
    v["datums"][0]["derive"] = json!({"kind":"point_difference","from":"missing","to":"later"});
    let m: Model = serde_json::from_value(v).unwrap();
    assert_eq!(build(&m).err().unwrap().code, "DATUM_REFERENCE");
    let m: Model = serde_json::from_value(fixture()).unwrap();
    assert_eq!(
        agent_cad::build_revision_with_options(&m, &mut Default::default(), Some(false))
            .err()
            .unwrap()
            .code,
        "QUERY_SNAPSHOT_REQUIRED"
    );
}

#[test]
fn relative_datum_selection_follows_rotated_geometry_and_rejects_forward_reference() {
    let tol = json!({"linear_mm":1e-5,"angular_rad":1e-7});
    for angle in [0_f64, 37., 90.] {
        let mut v = json!({"version":1,"units":"mm","parameters":{},"features":[{"id":"raw","op":"cylinder","origin":[0,0,0],"radius":3,"height":12},{"id":"part","op":"rotate","input":"raw","origin":[0,0,0],"axis":[0,1,0],"angle_deg":angle}],"output":"part","datums":[
        selected("axis","part","axis",json!({"kind":"filter","input":{"kind":"entities","entity":"face"},"where":{"surface_type":"cylinder"}})),
        {"id":"end","definition":"part","tolerance":tol,"derive":{"kind":"selected","derive":{"kind":"plane"},"selection":{"kind":"rank","input":{"kind":"filter","input":{"kind":"entities","entity":"face"},"where":{"planar_normal_parallel_to":{"datum":"axis"}}},"by":{"property":"center_projection","direction":{"datum":"axis"}},"descending":true,"rank":0,"tie_tolerance":1e-6}}}]});
        let m: Model = serde_json::from_value(v.clone()).unwrap();
        let b = build(&m).unwrap();
        let n = &b.report["datums"]["part"]["end"]["geometry"]["normal"];
        assert!((n[0].as_f64().unwrap() - angle.to_radians().sin()).abs() < 1e-7);
        assert!((n[2].as_f64().unwrap() - angle.to_radians().cos()).abs() < 1e-7);
        v["datums"].as_array_mut().unwrap().swap(0, 1);
        let m: Model = serde_json::from_value(v).unwrap();
        assert_eq!(build(&m).err().unwrap().code, "DATUM_REFERENCE");
    }
}
