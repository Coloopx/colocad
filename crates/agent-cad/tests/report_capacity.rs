//! [INPUT]: 真实公开build/checkpoints、2×3×4mm盒体和有界旧报告文件。
//! [OUTPUT]: oversized旧manifest冷建和小报告完整性保守控制；不修改生产。
//! [POS]: Windows native RED测试；没有原生/HTTP替身。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use agent_cad::{build, build_revision, incremental::Checkpoints, Model};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
fn model() -> Model {
    serde_json::from_value(json!({"version":1,"units":"mm","parameters":{},
        "features":[{"id":"box","op":"box","origin":[0,0,0],"size":[2,3,4]}],"output":"box"})).unwrap()
}
struct Saved(PathBuf);
impl Saved {
    fn new() -> Self {
        let p=std::env::temp_dir().join(format!("colocad-report-red-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
        fs::create_dir(&p).unwrap();eprintln!("retained report capacity evidence: {}",p.display());Self(p)
    }
    fn report(&self,raw:&[u8]) {fs::write(self.0.join("report.json"),raw).unwrap();}
}
fn cold(raw: Vec<u8>) {
    let saved=Saved::new();saved.report(&raw);
    let before=format!("{:x}",Sha256::digest(&raw));
    let mut cache=Checkpoints::revision(Some(&saved.0))
        .expect("oversized optional checkpoint manifest must select safe cold build, not CHECKPOINT_IO");
    let built=build_revision(&model(),&mut cache).unwrap();
    assert_eq!(built.report["validation"]["valid"],true);
    assert!((built.report["geometry"]["volume_mm3"].as_f64().unwrap()-24.0).abs()<1e-6);
    assert!(built.report["features"].as_array().unwrap().iter().all(|f|f["cache_hit"]!=true));
    assert_eq!(built.report["checkpoint_recovery"],json!({"mode":"cold","reason":"oversized_legacy_manifest"}));
    assert_eq!(format!("{:x}",Sha256::digest(fs::read(saved.0.join("report.json")).unwrap())),before);
}
#[test]
fn oversized_valid_legacy_manifest_selects_cold_rebuild() {
    let mut report=build(&model()).unwrap().report;
    report["public_padding"]=Value::String("x".repeat(2*1024*1024+1));
    cold(serde_json::to_vec(&report).unwrap());
}
#[test]
fn oversized_unparseable_manifest_is_not_read_or_trusted() {
    cold(vec![b'x';2*1024*1024+1]);
}
#[test]
fn small_corrupt_manifest_still_reports_integrity_failure() {
    let saved=Saved::new();saved.report(b"not JSON");
    let error=Checkpoints::revision(Some(&saved.0)).err().expect("small corrupt manifest must not silently become trusted");
    assert_eq!(error.code,"CHECKPOINT_IO");
}
#[test]
fn corrupt_checkpoint_digest_is_never_restored_as_trusted_geometry() {
    let saved=Saved::new();
    let mut initial=Checkpoints::revision(None).unwrap();
    let first=build_revision(&model(),&mut initial).unwrap();
    assert!(first.report["features"][0]["checkpoint"].is_string(),"fixture must capture a real native checkpoint");
    let mut report=first.report.clone();let mut corrupted=false;
    report["artifact_sha256"]=json!({});
    for (name,bytes) in &first.artifacts {
        report["artifact_sha256"][name]=json!(format!("{:x}",Sha256::digest(bytes)));
        let mut altered=bytes.clone();
        if name.starts_with("geom-") {altered.push(0);corrupted=true;}
        fs::write(saved.0.join(name),altered).unwrap();
    }
    assert!(corrupted,"fixture needs an actual native geom payload");
    saved.report(&serde_json::to_vec(&report).unwrap());
    let mut cache=Checkpoints::revision(Some(&saved.0)).unwrap();
    let rebuilt=build_revision(&model(),&mut cache).unwrap();
    assert!(rebuilt.report["features"].as_array().unwrap().iter().all(|f|f["cache_hit"]!=true));
    assert!((rebuilt.report["geometry"]["volume_mm3"].as_f64().unwrap()-24.0).abs()<1e-6);
    assert_eq!(rebuilt.report["validation"]["valid"],true);
}
#[test]
fn report_counters_match_actual_compact_stdout_and_pretty_file_bytes() {
    let report=json!({"nested":{"unicode":"日".repeat(500),"items":[1,2,3]}});
    let wrapper=json!({"ok":true,"data":&report});
    let stdout=serde_json::to_vec(&wrapper).unwrap().len()+1;
    let file=serde_json::to_vec_pretty(&report).unwrap().len();
    agent_cad::resources::check_report(&wrapper,stdout).unwrap();
    agent_cad::resources::check_report_file(&report,file).unwrap();
    let error=agent_cad::resources::check_report(&wrapper,stdout-1).unwrap_err();
    let details=error.details.unwrap();
    assert_eq!(details["resource"],"reportBytes");
    assert_eq!(details["observed_bytes"],stdout);
    assert_eq!(details["observed_is_lower_bound"],false);
    assert!(agent_cad::resources::check_report_file(&report,file-1).is_err());
}
