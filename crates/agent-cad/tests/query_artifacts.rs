use serde_json::{json, Value};
use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "colocad-query-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn call(args: &[&str], value: &Value) -> Value {
    let mut p = Command::new(env!("CARGO_BIN_EXE_agent-cad"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    p.stdin
        .take()
        .unwrap()
        .write_all(value.to_string().as_bytes())
        .unwrap();
    let out = p.wait_with_output().unwrap();
    serde_json::from_slice(&out.stdout).unwrap()
}
fn model(size: f64) -> Value {
    json!({"version":1,"units":"mm","parameters":{},"features":[{"id":"stock","op":"box","origin":[0,0,0],"size":[size,20,30]}],"output":"stock"})
}
fn build(out: &std::path::Path, size: f64) -> Value {
    let r = call(
        &["build", "--out", out.to_str().unwrap(), "--queryable"],
        &model(size),
    );
    assert_eq!(r["ok"], true, "{r}");
    r
}
fn request(rev: &str) -> Value {
    json!({"schema":"colocad.query/1","query_revision_id":rev,"target":{"definition_id":"stock"},"space":"definition","tolerance":{"linear_mm":1e-5,"angular_rad":1e-7},"queries":[{"id":"planes","op":"faces","where":{"surface_type":"plane"},"limit":64}]})
}
#[test]
fn bundled_face_grid_has_queryable_exact_snapshot() {
    let temp = Temp::new();
    let path = temp.0.join("grid");
    let m: Value = serde_json::from_str(include_str!(
        "../../../skills/colocad/assets/plate-grid.json"
    ))
    .unwrap();
    let built = call(
        &["build", "--out", path.to_str().unwrap(), "--queryable"],
        &m,
    );
    assert_eq!(built["ok"], true, "{built}");
    let req = json!({"schema":"colocad.query/1",
        "query_revision_id":built["data"]["query_revision_id"],
        "target":{"definition_id":"mounts"},"space":"definition",
        "tolerance":{"linear_mm":1e-5,"angular_rad":1e-7},
        "queries":[{"id":"bores","op":"holes","where":{"termination":"through"}}]});
    let found = call(&["query", "--snapshot", path.to_str().unwrap()], &req);
    assert_eq!(found["ok"], true, "{found}");
    let result = &found["data"]["results"][0];
    assert_eq!(result["status"], "complete", "{found}");
    assert_eq!(result["items"].as_array().unwrap().len(), 4);
    for hole in result["items"].as_array().unwrap() {
        let segments = hole["segments"].as_array().unwrap();
        assert!(!segments.is_empty());
        for s in segments {
            assert!((s["diameter_mm"].as_f64().unwrap() - 6.).abs() < 1e-5);
        }
    }
    assert_eq!(found["data"]["execution"]["geometry_build_calls"], 0);
}
#[test]
fn direct_grids_on_all_faces_preserve_seams_and_queries_after_relocation() {
    let temp = Temp::new();
    for (i, face) in [
        "pos_x", "neg_x", "pos_y", "neg_y", "pos_z", "neg_z", "global",
    ]
    .iter()
    .enumerate()
    {
        let axis = if i < 6 { i / 2 } else { 2 };
        let center = match axis {
            0 => [41., 43.],
            1 => [37., 43.],
            _ => [37., 41.],
        };
        let feature = if *face == "global" {
            json!({"id":"mounts","op":"hole_grid","input":"stock","origin":[23,32,12],"count":[3,2],"spacing":[14,18],"radius":2,"depth":62})
        } else {
            json!({"id":"mounts","op":"face_hole_grid","input":"stock","face":face,"center":center,"count":[3,2],"spacing":[14,18],"radius":2,"depth":60})
        };
        let mut m = json!({"version":1,"units":"mm","parameters":{},"features":[
            {"id":"stock","op":"box","origin":[7,11,13],"size":[60,60,60]},feature
        ],"output":"mounts"});
        if i % 2 == 0 {
            m["preview"] = json!({"width":160,"height":120,"chord_mm":1});
        }
        let path = temp.0.join(format!("grid-{i}"));
        let b = call(
            &["build", "--out", path.to_str().unwrap(), "--queryable"],
            &m,
        );
        assert_eq!(b["ok"], true, "{face}: {b}");
        let expected = 60_f64.powi(3) - 6. * std::f64::consts::PI * 4. * 60.;
        assert!((b["data"]["geometry"]["volume_mm3"].as_f64().unwrap() - expected).abs() < 1e-5);
        let q = json!({"schema":"colocad.query/1","query_revision_id":b["data"]["query_revision_id"],
        "target":{"definition_id":"mounts"},"space":"definition",
        "tolerance":{"linear_mm":1e-5,"angular_rad":1e-7},"queries":[
            {"id":"bores","op":"holes","where":{"termination":"through"}},
            {"id":"walls","op":"faces","where":{"surface_type":"cylinder"}},
            {"id":"edges","op":"edges"}, {"id":"loops","op":"loops"}
        ]});
        let r = call(&["query", "--snapshot", path.to_str().unwrap()], &q);
        assert_eq!(r["ok"], true, "{face}: {r}");
        let results = r["data"]["results"].as_array().unwrap();
        assert_eq!(results[0]["status"], "complete", "{face}: {r}");
        assert_eq!(results[0]["items"].as_array().unwrap().len(), 6);
        for h in results[0]["items"].as_array().unwrap() {
            for s in h["segments"].as_array().unwrap() {
                assert!((s["diameter_mm"].as_f64().unwrap() - 4.).abs() < 1e-5);
            }
        }
        let walls = results[1]["items"].as_array().unwrap();
        assert_eq!(walls.len(), 6);
        for wall in walls {
            assert_eq!(wall["material_side"], "internal");
            assert!(
                (wall["angular_coverage_rad"].as_f64().unwrap() - std::f64::consts::TAU).abs()
                    < 1e-7
            );
            let a = wall["axial_interval_mm"].as_array().unwrap();
            assert!((a[1].as_f64().unwrap() - a[0].as_f64().unwrap() - 60.).abs() < 1e-5);
        }
        let edges = results[2]["items"].as_array().unwrap();
        assert_eq!(edges.iter().filter(|e| e["seam"] == true).count(), 6);
        assert_eq!(
            edges.iter().filter(|e| e["curve_type"] == "circle").count(),
            12
        );
        for l in results[3]["items"].as_array().unwrap() {
            for u in l["edge_uses"].as_array().unwrap() {
                assert!(u["uv_bounds"].is_array());
            }
        }
        let moved = temp.0.join(format!("moved-{i}"));
        fs::rename(path, &moved).unwrap();
        let reopened = call(&["query", "--snapshot", moved.to_str().unwrap()], &q);
        assert_eq!(r["data"]["results"], reopened["data"]["results"]);
        for key in [
            "geometry_build_calls",
            "step_export_calls",
            "tessellation_calls",
        ] {
            assert_eq!(reopened["data"]["execution"][key], 0);
        }
    }
}
#[test]
fn queryable_build_reopens_after_relocation_and_preserves_reference_identity() {
    let temp = Temp::new();
    let path = temp.0.join("original");
    let b = build(&path, 10.);
    let req = request(b["data"]["query_revision_id"].as_str().unwrap());
    let before = call(&["query", "--snapshot", path.to_str().unwrap()], &req);
    assert_eq!(before["ok"], true, "{before}");
    assert_eq!(
        before["data"]["results"][0]["items"]
            .as_array()
            .unwrap()
            .len(),
        6
    );
    let moved = temp.0.join("moved");
    fs::rename(path, &moved).unwrap();
    let after = call(&["query", "--snapshot", moved.to_str().unwrap()], &req);
    assert_eq!(before["data"]["results"], after["data"]["results"]);
    assert_eq!(after["data"]["execution"]["geometry_build_calls"], 0);
    assert_eq!(after["data"]["execution"]["step_export_calls"], 0);
}
#[test]
fn edited_snapshot_rejects_stale_request() {
    let temp = Temp::new();
    let a = build(&temp.0.join("a"), 10.);
    let b = build(&temp.0.join("b"), 11.);
    assert_ne!(
        a["data"]["query_revision_id"],
        b["data"]["query_revision_id"]
    );
    let r = call(
        &["query", "--snapshot", temp.0.join("b").to_str().unwrap()],
        &request(a["data"]["query_revision_id"].as_str().unwrap()),
    );
    assert_eq!(r["error"]["code"], "QUERY_REVISION_MISMATCH");
}
#[test]
fn corrupted_geometry_is_rejected_before_evaluation() {
    let temp = Temp::new();
    let path = temp.0.join("a");
    let a = build(&path, 10.);
    let file = fs::read_dir(&path)
        .unwrap()
        .map(|x| x.unwrap().path())
        .find(|p| p.extension().is_some_and(|e| e == "smb"))
        .unwrap();
    let mut bytes = fs::read(&file).unwrap();
    bytes[0] ^= 1;
    fs::write(file, bytes).unwrap();
    let r = call(
        &["query", "--snapshot", path.to_str().unwrap()],
        &request(a["data"]["query_revision_id"].as_str().unwrap()),
    );
    assert_eq!(r["error"]["code"], "ARTIFACT_INTEGRITY");
}
#[test]
fn legacy_build_has_no_snapshot_and_query_never_rebuilds() {
    let temp = Temp::new();
    let path = temp.0.join("a");
    let r = call(&["build", "--out", path.to_str().unwrap()], &model(10.));
    assert_eq!(r["ok"], true);
    assert!(!path.join("query-manifest.json").exists());
    let r = call(
        &["query", "--snapshot", path.to_str().unwrap()],
        &request("sha256:missing"),
    );
    assert_eq!(r["error"]["code"], "QUERY_SNAPSHOT_REQUIRED");
    assert!(!path.join("query-manifest.json").exists());
}

#[test]
fn composable_selection_retains_ties_and_derives_plane_without_holes() {
    let temp = Temp::new();
    let path = temp.0.join("part");
    let a = build(&path, 10.);
    let mut req = request(a["data"]["query_revision_id"].as_str().unwrap());
    let top = json!({"kind":"expect","count":1,"input":{"kind":"filter","input":{"kind":"entities","entity":"face"},"where":{"planar_normal_aligned_with":[0,0,1]}}});
    req["queries"] = json!([
        {"id":"datum","op":"datums","selection":top,"derive":{"kind":"plane"}},
        {"id":"edges","op":"select","selection":{"kind":"children","input":top,"entity":"edge"}}
    ]);
    let r = call(&["query", "--snapshot", path.to_str().unwrap()], &req);
    assert_eq!(r["ok"], true, "{r}");
    assert_eq!(
        r["data"]["results"][0]["items"][0]["geometry"]["normal"],
        json!([0., 0., 1.])
    );
    assert!(
        (r["data"]["results"][0]["items"][0]["geometry"]["point_mm"][2]
            .as_f64()
            .unwrap()
            - 30.)
            .abs()
            < 1e-6
    );
    assert_eq!(r["data"]["results"][1]["total_count"], 4);
    req["queries"] = json!([{"id":"tie","op":"select","limit":1,"selection":{"kind":"expect","count":1,"input":{"kind":"rank","input":{"kind":"entities","entity":"face"},"by":{"property":"area_mm2"},"descending":true,"rank":0,"tie_tolerance":1e-6}}}]);
    let r = call(&["query", "--snapshot", path.to_str().unwrap()], &req);
    assert_eq!(r["error"]["code"], "SELECTION_CARDINALITY", "{r}");
}

#[test]
fn face_loop_circle_center_chain_excludes_other_installation_plane() {
    let temp = Temp::new();
    let path = temp.0.join("bored");
    let m = json!({"version":1,"units":"mm","parameters":{},"features":[{"id":"stock","op":"box","origin":[-10,-10,0],"size":[20,20,10]},{"id":"tool","op":"cylinder","origin":[2,3,-1],"radius":2,"height":12},{"id":"part","op":"cut","left":"stock","right":"tool"}],"output":"part"});
    let built = call(
        &["build", "--out", path.to_str().unwrap(), "--queryable"],
        &m,
    );
    assert_eq!(built["ok"], true, "{built}");
    let mut req = request(built["data"]["query_revision_id"].as_str().unwrap());
    req["target"] = json!({"definition_id":"part"});
    let selection = json!({"kind":"children","entity":"edge","input":{"kind":"filter","where":{"loop_role":"inner"},"input":{"kind":"children","entity":"loop","input":{"kind":"filter","where":{"planar_normal_aligned_with":[0,0,1]},"input":{"kind":"entities","entity":"face"}}}}});
    req["queries"] =
        json!([{"id":"center","op":"datums","derive":{"kind":"point"},"selection":selection}]);
    let r = call(&["query", "--snapshot", path.to_str().unwrap()], &req);
    assert_eq!(r["ok"], true, "{r}");
    assert_eq!(r["data"]["results"][0]["total_count"], 1);
    assert_eq!(
        r["data"]["results"][0]["items"][0]["geometry"]["point_mm"],
        json!([2., 3., 10.])
    );
}

#[test]
fn pagination_is_complete_bound_to_query_and_unknown_filters_fail() {
    let temp = Temp::new();
    let path = temp.0.join("part");
    let a = build(&path, 10.);
    let mut req = request(a["data"]["query_revision_id"].as_str().unwrap());
    req["queries"][0]["limit"] = json!(2);
    let first = call(&["query", "--snapshot", path.to_str().unwrap()], &req);
    assert_eq!(first["ok"], true, "{first}");
    let cursor = first["data"]["results"][0]["next_cursor"].clone();
    assert!(cursor.is_string());
    req["queries"][0]["cursor"] = cursor;
    let next = call(&["query", "--snapshot", path.to_str().unwrap()], &req);
    assert_eq!(next["data"]["results"][0]["total_count"], 6);
    assert_ne!(
        first["data"]["results"][0]["items"],
        next["data"]["results"][0]["items"]
    );
    req["queries"][0]["where"]["area_mm2"] = json!({"min":0});
    let mismatch = call(&["query", "--snapshot", path.to_str().unwrap()], &req);
    assert_eq!(mismatch["error"]["code"], "QUERY_REVISION_MISMATCH");
    req["queries"][0]["where"]["invented"] = json!(true);
    let invalid = call(&["query", "--snapshot", path.to_str().unwrap()], &req);
    assert_eq!(invalid["error"]["code"], "SCHEMA_ERROR");
}

#[test]
fn assembly_occurrences_share_definition_but_transform_geometry_and_refs() {
    let temp = Temp::new();
    let path = temp.0.join("assembly");
    let mut m = model(10.);
    m.as_object_mut().unwrap().remove("output");
    m["assembly"] = json!([{"id":"a","input":"stock"},{"id":"b","input":"stock","rotation_deg":[0,0,90],"translation":[100,0,0]}]);
    let a = call(
        &["build", "--out", path.to_str().unwrap(), "--queryable"],
        &m,
    );
    assert_eq!(a["ok"], true, "{a}");
    let manifest: Value =
        serde_json::from_slice(&fs::read(path.join("query-manifest.json")).unwrap()).unwrap();
    assert_eq!(
        manifest["body"]["definitions"].as_object().unwrap().len(),
        1
    );
    assert_eq!(
        manifest["body"]["occurrences"].as_object().unwrap().len(),
        2
    );
    let mut req = request(a["data"]["query_revision_id"].as_str().unwrap());
    req["target"] = json!({"instance_path":["b"]});
    req["space"] = json!("assembly");
    req["queries"] = json!([{"id":"vertices","op":"vertices"}]);
    let r = call(&["query", "--snapshot", path.to_str().unwrap()], &req);
    assert_eq!(r["ok"], true, "{r}");
    let items = r["data"]["results"][0]["items"].as_array().unwrap();
    assert_eq!(items.len(), 8);
    assert!(items
        .iter()
        .all(|v| v["point_mm"][0].as_f64().unwrap() >= 80. - 1e-6
            && v["point_mm"][0].as_f64().unwrap() <= 100. + 1e-6));
    let reference = items[0]["ref"].clone();
    req["target"] = json!({"instance_path":["a"]});
    req["queries"] = json!([{"id":"wrong_owner","op":"describe","refs":[reference]}]);
    let r = call(&["query", "--snapshot", path.to_str().unwrap()], &req);
    assert_eq!(r["error"]["code"], "TOPOLOGY_REF_INVALID");
}

#[test]
fn evaluate_face_distinguishes_hole_interior_material_and_outside_parameter_domain() {
    let temp = Temp::new();
    let path = temp.0.join("bored");
    let m = json!({"version":1,"units":"mm","parameters":{},"features":[{"id":"stock","op":"box","origin":[-10,-10,0],"size":[20,20,10]},{"id":"tool","op":"cylinder","origin":[0,0,-1],"radius":2,"height":12},{"id":"part","op":"cut","left":"stock","right":"tool"}],"output":"part"});
    let built = call(
        &["build", "--out", path.to_str().unwrap(), "--queryable"],
        &m,
    );
    assert_eq!(built["ok"], true);
    let mut req = request(built["data"]["query_revision_id"].as_str().unwrap());
    req["target"] = json!({"definition_id":"part"});
    req["queries"][0]["where"] = json!({"planar_normal_aligned_with":[0,0,1]});
    let r = call(&["query", "--snapshot", path.to_str().unwrap()], &req);
    assert_eq!(r["ok"], true, "{r}");
    let f = &r["data"]["results"][0]["items"][0];
    let domain = &f["uv_domain"];
    let uv = |t: f64| {
        [0, 1]
            .map(|i| domain[0][i].as_f64().unwrap() * (1. - t) + domain[1][i].as_f64().unwrap() * t)
    };
    req["queries"] = json!([{"id":"hole","op":"evaluate_face","ref":f["ref"],"uv":uv(0.5)},{"id":"material","op":"evaluate_face","ref":f["ref"],"uv":uv(0.1)},{"id":"out","op":"evaluate_face","ref":f["ref"],"uv":uv(2.)}]);
    let r = call(&["query", "--snapshot", path.to_str().unwrap()], &req);
    assert_eq!(r["ok"], true, "{r}");
    let results = &r["data"]["results"];
    assert_eq!(results[0]["items"][0]["trim_status"], "outside_trim");
    assert_eq!(results[0]["items"][0]["point_on_material_face"], false);
    assert_eq!(results[1]["items"][0]["trim_status"], "inside");
    assert_eq!(results[1]["items"][0]["normal"], json!([0., 0., 1.]));
    assert_eq!(
        results[2]["items"][0]["evaluation_status"],
        "outside_parameter_domain"
    );
    assert!(results[2]["items"][0]["point_mm"].is_null());
}

#[test]
fn digest_bound_manifest_rejects_escaping_payload_even_with_recomputed_identity() {
    let temp = Temp::new();
    let path = temp.0.join("part");
    build(&path, 10.);
    let mut manifest: agent_cad::query::artifact::Manifest =
        serde_json::from_slice(&fs::read(path.join("query-manifest.json")).unwrap()).unwrap();
    manifest.body.definitions.get_mut("stock").unwrap().path = "../outside.smb".into();
    manifest.query_revision_id =
        agent_cad::query::artifact::Manifest::revision(&manifest.body).unwrap();
    fs::write(
        path.join("query-manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    let r = call(
        &["query", "--snapshot", path.to_str().unwrap()],
        &request(&manifest.query_revision_id),
    );
    assert_eq!(r["error"]["code"], "ARTIFACT_INTEGRITY");
}

#[cfg(unix)]
#[test]
fn symlinked_payload_is_rejected_and_expression_budget_is_enforced() {
    let temp = Temp::new();
    let path = temp.0.join("part");
    let built = build(&path, 10.);
    let mut req = request(built["data"]["query_revision_id"].as_str().unwrap());
    let mut selection = json!({"kind":"entities","entity":"face"});
    for _ in 0..9 {
        selection = json!({"kind":"expect","count":6,"input":selection});
    }
    req["queries"] = json!([{"id":"deep","op":"select","selection":selection}]);
    let r = call(&["query", "--snapshot", path.to_str().unwrap()], &req);
    assert_eq!(r["error"]["code"], "QUERY_LIMIT");
    let payload = path.join("query-definition-0.smb");
    let outside = temp.0.join("outside.smb");
    fs::rename(&payload, &outside).unwrap();
    std::os::unix::fs::symlink(&outside, &payload).unwrap();
    let r = call(
        &["query", "--snapshot", path.to_str().unwrap()],
        &request(built["data"]["query_revision_id"].as_str().unwrap()),
    );
    assert_eq!(r["error"]["code"], "ARTIFACT_INTEGRITY");
}

#[test]
fn topology_neighbors_share_explicit_edges_and_set_algebra_preserves_cardinality() {
    let temp = Temp::new();
    let b = build(&temp.0.join("a"), 10.);
    let mut r = request(b["data"]["query_revision_id"].as_str().unwrap());
    let top = json!({"kind":"filter","input":{"kind":"entities","entity":"face"},"where":{"planar_normal_aligned_with":[0,0,1]}});
    let siblings = json!({"kind":"siblings","input":top,"entity":"face","via":"edge"});
    let ancestors = json!({"kind":"ancestors","entity":"face","input":{"kind":"children","input":top,"entity":"edge"}});
    r["queries"] = json!([
      {"id":"neighbors","op":"select","selection":{"kind":"expect","count":4,"input":siblings}},
      {"id":"parents","op":"select","selection":{"kind":"expect","count":5,"input":ancestors}},
      {"id":"difference","op":"select","selection":{"kind":"expect","count":1,"input":{"kind":"difference","left":ancestors,"right":siblings}}},
      {"id":"intersection","op":"select","selection":{"kind":"expect","count":0,"input":{"kind":"intersection","inputs":[top,siblings]}}}
    ]);
    let q = call(
        &["query", "--snapshot", temp.0.join("a").to_str().unwrap()],
        &r,
    );
    assert_eq!(q["ok"], true, "{q}");
    for f in q["data"]["results"][0]["items"].as_array().unwrap() {
        assert!(f["normal"][2].as_f64().unwrap().abs() < 1e-8);
    }
    assert!(
        (q["data"]["results"][2]["items"][0]["center_mm"][2]
            .as_f64()
            .unwrap()
            - 30.)
            .abs()
            < 1e-8
    );
}

#[test]
fn versioned_manifest_canonical_bytes_match_independent_sha_vector() {
    let bytes =
        include_str!("../../../specs/experiments/0004/fixtures/manifest-vector.json").trim_end();
    let body: agent_cad::query::artifact::ManifestBody = serde_json::from_str(bytes).unwrap();
    assert_eq!(serde_json::to_string(&body).unwrap(), bytes);
    assert_eq!(
        agent_cad::query::artifact::Manifest::revision(&body).unwrap(),
        include_str!("../../../specs/experiments/0004/fixtures/manifest-vector.sha256").trim()
    );
}

#[test]
fn cylinder_descriptors_expose_trim_period_seam_adjacency_and_material_side() {
    let t = Temp::new();
    let p = t.0.join("cylinder");
    let m = json!({"version":1,"units":"mm","parameters":{},"features":[{"id":"stock","op":"cylinder","origin":[0,0,0],"radius":3,"height":12}],"output":"stock"});
    let b = call(&["build", "--out", p.to_str().unwrap(), "--queryable"], &m);
    assert_eq!(b["ok"], true, "{b}");
    let mut q = request(b["data"]["query_revision_id"].as_str().unwrap());
    q["queries"] = json!([{ "id":"cylinder","op":"faces","where":{"surface_type":"cylinder"}},{"id":"edges","op":"edges"},{"id":"vertices","op":"vertices"},{"id":"loops","op":"loops"}]);
    let r = call(&["query", "--snapshot", p.to_str().unwrap()], &q);
    assert_eq!(r["ok"], true, "{r}");
    let f = &r["data"]["results"][0]["items"][0];
    assert_eq!(f["material_side"], "external");
    assert_eq!(f["trim_coverage_status"], "rectangular");
    assert_eq!(f["parameterization"]["periodic"][0], true);
    assert!((f["angular_coverage_rad"].as_f64().unwrap() - std::f64::consts::TAU).abs() < 1e-7);
    let a = f["axial_interval_mm"].as_array().unwrap();
    assert!((a[1].as_f64().unwrap() - a[0].as_f64().unwrap() - 12.).abs() < 1e-7);
    assert_eq!(f["adjacent_faces"].as_array().unwrap().len(), 2);
    let edges = r["data"]["results"][1]["items"].as_array().unwrap();
    assert_eq!(edges.iter().filter(|e| e["seam"] == true).count(), 1);
    for e in edges.iter().filter(|e| e["curve_type"] == "circle") {
        assert!((e["arc_coverage_rad"].as_f64().unwrap() - std::f64::consts::TAU).abs() < 1e-7);
    }
    for l in r["data"]["results"][3]["items"].as_array().unwrap() {
        for u in l["edge_uses"].as_array().unwrap() {
            assert!(u["uv_bounds"].is_array(), "{u}");
        }
    }
}
fn hole_plate() -> Value {
    let mut features =
        vec![json!({"id":"stock0","op":"box","origin":[-50,-30,0],"size":[100,60,12]})];
    let mut prior = "stock0".to_string();
    for (i, (x, y, z, r, h)) in [
        (-30., -20., -1., 3., 14.),
        (30., -20., 4., 5., 9.),
        (0., 15., -1., 3., 14.),
        (0., 15., 9., 5., 4.),
    ]
    .into_iter()
    .enumerate()
    {
        let id = format!("hole{i}");
        features.push(json!({"id":id,"op":"holes","input":prior,"frame":{"origin":[x,y,z],"x_axis":[1,0,0],"y_axis":[0,1,0]},"positions":[[0,0]],"radius":r,"depth":h}));
        prior = id;
    }
    features.push(json!({"id":"stock","op":"translate","input":prior,"offset":[0,0,0]}));
    json!({"version":1,"units":"mm","parameters":{},"features":features,"output":"stock"})
}
#[test]
fn recognizes_through_blind_and_counterbore_with_exact_depths_and_no_duplicate_entries() {
    let t = Temp::new();
    let p = t.0.join("holes");
    let b = call(
        &["build", "--out", p.to_str().unwrap(), "--queryable"],
        &hole_plate(),
    );
    assert_eq!(b["ok"], true, "{b}");
    let mut q = request(b["data"]["query_revision_id"].as_str().unwrap());
    q["queries"] = json!([{"id":"holes","op":"holes"}]);
    let r = call(&["query", "--snapshot", p.to_str().unwrap()], &q);
    assert_eq!(r["ok"], true, "{r}");
    let found = &r["data"]["results"][0];
    assert_eq!(found["status"], "complete", "{found}");
    let items = found["items"].as_array().unwrap();
    assert_eq!(items.len(), 3, "{found}");
    let blind = items.iter().find(|h| h["termination"] == "blind").unwrap();
    assert!((blind["depth_mm"].as_f64().unwrap() - 8.).abs() < 1e-5);
    assert_eq!(blind["openings"].as_array().unwrap().len(), 1);
    let stepped = items
        .iter()
        .find(|h| h["segments"].as_array().unwrap().len() == 2)
        .unwrap();
    assert_eq!(stepped["termination"], "through");
    assert_eq!(stepped["openings"].as_array().unwrap().len(), 2);
    let mut depths = stepped["segments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["length_mm"].as_f64().unwrap())
        .collect::<Vec<_>>();
    depths.sort_by(f64::total_cmp);
    assert!((depths[0] - 3.).abs() < 1e-5);
    assert!((depths[1] - 9.).abs() < 1e-5);
    assert!(r["data"]["execution"]["material_probes"].as_u64().unwrap() > 0);
}
#[test]
fn hole_recognition_excludes_external_shaft_and_marks_side_open_recess_partial() {
    for side_open in [false, true] {
        let t = Temp::new();
        let p = t.0.join("part");
        let m = if side_open {
            json!({"version":1,"units":"mm","parameters":{},"features":[{"id":"raw","op":"box","origin":[0,0,0],"size":[20,20,10]},{"id":"stock","op":"holes","input":"raw","frame":{"origin":[0,10,-1],"x_axis":[1,0,0],"y_axis":[0,1,0]},"positions":[[0,0]],"radius":3,"depth":12}],"output":"stock"})
        } else {
            json!({"version":1,"units":"mm","parameters":{},"features":[{"id":"stock","op":"cylinder","origin":[0,0,0],"radius":3,"height":12}],"output":"stock"})
        };
        let b = call(&["build", "--out", p.to_str().unwrap(), "--queryable"], &m);
        assert_eq!(b["ok"], true, "{b}");
        let mut q = request(b["data"]["query_revision_id"].as_str().unwrap());
        q["queries"] = json!([{"id":"holes","op":"holes"}]);
        let r = call(&["query", "--snapshot", p.to_str().unwrap()], &q);
        assert_eq!(r["ok"], true, "{r}");
        let h = &r["data"]["results"][0];
        assert_eq!(h["items"].as_array().unwrap().len(), 0, "{h}");
        assert_eq!(
            h["status"],
            if side_open { "partial" } else { "complete" },
            "{h}"
        );
    }
}
#[test]
fn groups_report_rectangular_circular_linear_patterns_and_excluded_extra_holes() {
    for (pattern, points, count) in [
        (
            "rectangular",
            vec![[-10., -6.], [10., -6.], [-10., 6.], [10., 6.], [22., 18.]],
            4,
        ),
        (
            "circular",
            (0..6)
                .map(|i| {
                    let a = i as f64 * std::f64::consts::TAU / 6.;
                    [12. * a.cos(), 12. * a.sin()]
                })
                .collect::<Vec<_>>(),
            6,
        ),
        (
            "linear",
            vec![[-20., 0.], [-10., 0.], [0., 0.], [10., 0.], [20., 0.]],
            4,
        ),
    ] {
        let t = Temp::new();
        let p = t.0.join("pattern");
        let m = json!({"version":1,"units":"mm","parameters":{},"features":[{"id":"base","op":"box","origin":[-30,-25,0],"size":[60,50,8]},{"id":"stock","op":"holes","input":"base","frame":{"origin":[0,0,-1],"x_axis":[1,0,0],"y_axis":[0,1,0]},"positions":points,"radius":2,"depth":10}],"output":"stock"});
        let b = call(&["build", "--out", p.to_str().unwrap(), "--queryable"], &m);
        assert_eq!(b["ok"], true, "{b}");
        let mut q = request(b["data"]["query_revision_id"].as_str().unwrap());
        q["queries"] =
            json!([{"id":"top","op":"faces","where":{"planar_normal_aligned_with":[0,0,1]}}]);
        let top = call(&["query", "--snapshot", p.to_str().unwrap()], &q);
        assert_eq!(top["ok"], true, "{top}");
        q["queries"] = json!([{"id":"groups","op":"hole_groups","support_faces":[top["data"]["results"][0]["items"][0]["ref"]],"where":{"pattern":pattern,"member_count":count,"equal_diameter":true}}]);
        let r = call(&["query", "--snapshot", p.to_str().unwrap()], &q);
        assert_eq!(r["ok"], true, "{r}");
        let result = &r["data"]["results"][0];
        assert_eq!(result["status"], "complete", "{result}");
        let groups = result["items"].as_array().unwrap();
        assert_eq!(
            groups.len(),
            if pattern == "linear" { 2 } else { 1 },
            "{result}"
        );
        if pattern == "linear" {
            for group in groups {
                assert_eq!(group["excluded_members"].as_array().unwrap().len(), 1);
            }
        }
        let g = &groups[0];
        assert_eq!(g["members"].as_array().unwrap().len(), count);
        assert_eq!(g["orientation_unique"], false);
        if pattern == "rectangular" {
            assert_eq!(g["excluded_members"].as_array().unwrap().len(), 1);
            let mut pitches = g["parameters"]["pitch_mm"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect::<Vec<_>>();
            pitches.sort_by(f64::total_cmp);
            assert!((pitches[0] - 12.).abs() < 1e-5);
            assert!((pitches[1] - 20.).abs() < 1e-5);
        }
        if pattern == "circular" {
            assert!((g["parameters"]["radius_mm"].as_f64().unwrap() - 12.).abs() < 1e-5);
        }
        if pattern == "linear" {
            assert!((g["parameters"]["pitch_mm"].as_f64().unwrap() - 10.).abs() < 1e-5);
        }
    }
}
#[test]
fn exact_two_occurrence_measurement_loads_each_definition_once_without_rebuild() {
    let temp = Temp::new();
    let path = temp.0.join("assembly");
    let mut m = model(10.);
    m.as_object_mut().unwrap().remove("output");
    m["assembly"] =
        json!([{"id":"a","input":"stock"},{"id":"b","input":"stock","translation":[10.2,0,0]}]);
    let built = call(
        &["build", "--out", path.to_str().unwrap(), "--queryable"],
        &m,
    );
    assert_eq!(built["ok"], true, "{built}");
    let mut req = request(built["data"]["query_revision_id"].as_str().unwrap());
    req["target"] = json!({"instance_path":["a"]});
    req["space"] = json!("assembly");
    req["queries"] = json!([{"id":"gap","op":"measure","a":{"instance_path":["a"]},"b":{"instance_path":["b"]}},{"id":"again","op":"measure","a":{"instance_path":["b"]},"b":{"instance_path":["a"]}}]);
    let r = call(&["query", "--snapshot", path.to_str().unwrap()], &req);
    assert_eq!(r["ok"], true, "{r}");
    let data = &r["data"];
    assert_eq!(data["execution"]["definitions_loaded"], 1);
    assert_eq!(data["execution"]["geometry_build_calls"], 0);
    assert_eq!(data["execution"]["step_export_calls"], 0);
    for r in data["results"].as_array().unwrap() {
        assert_eq!(r["status"], "complete");
        assert_eq!(r["returned_count"], 1);
        assert_eq!(r["items"][0], r["measurement"]);
        assert_eq!(r["measurement"]["relationship"], "separated");
        assert!((r["measurement"]["boundary_distance_mm"].as_f64().unwrap() - 0.2).abs() < 1e-5);
    }
    req["queries"][0]["b"] = json!({"instance_path":["missing"]});
    let r = call(&["query", "--snapshot", path.to_str().unwrap()], &req);
    assert_eq!(r["error"]["code"], "TOPOLOGY_REF_INVALID");
}
#[test]
fn queryable_preview_persists_native_ref_bound_face_outlines() {
    let temp = Temp::new();
    let path = temp.0.join("display");
    let mut m = model(10.);
    m["preview"] = json!({"chord_mm":0.2});
    let b = call(
        &["build", "--out", path.to_str().unwrap(), "--queryable"],
        &m,
    );
    assert_eq!(b["ok"], true, "{b}");
    let guide: Value =
        serde_json::from_slice(&fs::read(path.join("query-display.json")).unwrap()).unwrap();
    assert_eq!(guide["query_revision_id"], b["data"]["query_revision_id"]);
    let r = call(
        &["query", "--snapshot", path.to_str().unwrap()],
        &request(b["data"]["query_revision_id"].as_str().unwrap()),
    );
    assert_eq!(r["ok"], true, "{r}");
    for face in r["data"]["results"][0]["items"].as_array().unwrap() {
        let token = face["ref"]["topology_id"].as_str().unwrap();
        let ids = guide["definitions"]["stock"]["faces"][token]["edges"]
            .as_array()
            .unwrap();
        assert_eq!(ids.len(), 4);
        for edge in ids {
            let line = guide["definitions"]["stock"]["edges"][edge.as_str().unwrap()]["points_mm"]
                .as_array()
                .unwrap();
            assert!(line.len() >= 2);
            for p in line {
                let point: Vec<f64> = serde_json::from_value(p.clone()).unwrap();
                let normal: Vec<f64> = serde_json::from_value(face["normal"].clone()).unwrap();
                let o: Vec<f64> = serde_json::from_value(face["plane_point_mm"].clone()).unwrap();
                assert!(
                    (0..3)
                        .map(|i| (point[i] - o[i]) * normal[i])
                        .sum::<f64>()
                        .abs()
                        < 1e-6
                );
            }
        }
    }
}
