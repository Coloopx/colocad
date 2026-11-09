use agent_cad::{
    query::{contract::Request, engine::execute},
    Model,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "colocad-foreign-{}-{}",
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
fn fixture(id: &str) -> (String, Value) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../specs/experiments/0004/fixtures/engineering");
    let manifest: Value =
        serde_json::from_slice(&fs::read(root.join("manifest.json")).unwrap()).unwrap();
    let row = manifest["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap()
        .clone();
    let step = fs::read_to_string(root.join(row["file"].as_str().unwrap())).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(step.as_bytes())),
        row["sha256"]
    );
    (step, row)
}
fn query(id: &str) -> Value {
    let (step, oracle) = fixture(id);
    let m:Model=serde_json::from_value(json!({"version":1,"units":"mm","parameters":{},"assets":{"foreign":{"sha256":oracle["sha256"],"step":step}},"features":[{"id":"part","op":"import_step","asset":"foreign","body_index":0}],"output":"part"})).unwrap();
    let b =
        agent_cad::build_revision_with_options(&m, &mut Default::default(), Some(true)).unwrap();
    assert!(
        (b.report["geometry"]["volume_mm3"].as_f64().unwrap()
            - oracle["volume_mm3"].as_f64().unwrap())
        .abs()
            < 1e-6_f64.max(oracle["volume_mm3"].as_f64().unwrap() * 1e-6),
        "{id}: measured {}, independent {}",
        b.report["geometry"]["volume_mm3"],
        oracle["volume_mm3"]
    );
    let tmp = Temp::new();
    for (file, bytes) in b.artifacts {
        fs::write(tmp.0.join(file), bytes).unwrap();
    }
    let q:Request=serde_json::from_value(json!({"schema":"colocad.query/1","query_revision_id":b.report["query_revision_id"],"target":{"definition_id":"part"},"space":"definition","tolerance":{"linear_mm":1e-5,"angular_rad":1e-7},"queries":[{"id":"bores","op":"holes"}]})).unwrap();
    execute(&tmp.0, q).unwrap()
}
#[test]
fn imported_axial_and_angular_cylinder_splits_form_one_physical_bore() {
    for id in ["split-axial-bore", "split-angular-bore"] {
        let r = query(id);
        let r = &r["results"][0];
        assert_eq!(r["status"], "complete", "{id}: {r}");
        assert_eq!(r["items"].as_array().unwrap().len(), 1, "{id}: {r}");
        let h = &r["items"][0];
        assert_eq!(h["termination"], "through");
        assert!((h["depth_mm"].as_f64().unwrap() - 12.).abs() < 1e-5);
        assert_eq!(h["source_refs"].as_array().unwrap().len(), 2);
    }
}
#[test]
fn imported_spline_conical_tip_and_intersecting_bores_do_not_claim_complete_recognition() {
    for id in ["spline-bore", "conical-tip", "intersecting-bores"] {
        let r = query(id);
        assert_eq!(r["results"][0]["status"], "partial", "{id}: {r}");
    }
}
