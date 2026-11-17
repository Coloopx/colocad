/*
 * [INPUT]: 公开box构造器生成的共享产品STEP及宿主计数策略。
 * [OUTPUT]: 单定义本地坐标、同原件整装配/单槽缓存合同与原安全门禁回归。
 * [POS]: 原生导入模式边界；无私有资产或CadQuery依赖。
 * [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
 */
use agent_cad::{build, build_revision_with_import_limits, incremental::Checkpoints, Model, StepImportLimits};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

fn model(v: Value) -> Model { serde_json::from_value(v).unwrap() }
fn import(step: &str, mode: Value) -> Model {
    let mut feature = json!({"id":"part","op":"import_step","asset":"source","body_index":0});
    feature.as_object_mut().unwrap().extend(mode.as_object().unwrap().clone());
    model(json!({"version":1,"units":"mm","parameters":{},
        "assets":{"source":{"step":step,"sha256":format!("{:x}",Sha256::digest(step))}},
        "features":[feature],"output":"part"}))
}
fn run(m: &Model, limits: StepImportLimits) -> Result<agent_cad::Build, agent_cad::CadError> {
    build_revision_with_import_limits(m, &mut Checkpoints::default(), Some(false), limits)
}
fn remap(row: &str, ids: &BTreeSet<u64>, shift: u64) -> String {
    let mut out = String::new();
    let mut chars = row.chars().peekable();
    while let Some(c) = chars.next() {
        out.push(c);
        if c != '#' { continue; }
        let digits: String = std::iter::from_fn(|| chars.next_if(|c| c.is_ascii_digit())).collect();
        let id: u64 = digits.parse().unwrap();
        out.push_str(&(if ids.contains(&id) { id + shift } else { id }).to_string());
    }
    out
}
fn repeated_source() -> String {
    let source = model(json!({"version":1,"units":"mm","parameters":{},
        "features":[{"id":"box","op":"box","origin":[0,0,0],"size":[2,2,2]}],
        "assembly":[{"id":"part","input":"box","translation":[17,23,31]}]}));
    let step = build(&source).unwrap().step;
    let records = step.lines().filter(|s| s.starts_with('#')).collect::<Vec<_>>();
    let tail = &records[records.len()-9..];
    assert!(tail.iter().any(|s| s.contains("NEXT_ASSEMBLY_USAGE_OCCURRENCE(")));
    let ids = tail.iter().map(|s| s[1..s.find('=').unwrap()].parse().unwrap()).collect();
    let rows = (1..129).flat_map(|i| tail.iter().map(move |s| (*s,i)))
        .map(|(s,i)| remap(s,&ids,i*9)).collect::<Vec<_>>().join("\n");
    step.replace("ENDSEC;\nEND-ISO-10303-21;", &(rows+"\nENDSEC;\nEND-ISO-10303-21;"))
}
fn geometry(b: &agent_cad::Build, count: usize, bbox: Value) {
    assert_eq!(b.report["geometry"]["solids"],count);
    assert!((b.report["geometry"]["volume_mm3"].as_f64().unwrap()-8.0*count as f64).abs()<1e-5);
    for i in 0..2 { for j in 0..3 {
        let actual = b.report["geometry"]["bbox_mm"][i][j].as_f64().unwrap();
        let expected = bbox[i][j].as_f64().unwrap();
        assert!((actual-expected).abs()<1e-5,"{actual} != {expected}");
    }}
}

#[test]
fn default_single_definition_ignores_unrequested_product_placements() {
    let step = repeated_source();
    let m = import(&step,json!({"definition":true}));
    for limits in [StepImportLimits::default(),StepImportLimits { max_occurrences:1,max_groups:1,..Default::default() }] {
        geometry(&run(&m,limits).unwrap(),1,json!([[0,0,0],[2,2,2]]));
    }
}
#[test]
fn same_source_default_whole_refuses_and_explicit_129_preserves_all_solids() {
    let step = repeated_source();
    let m = import(&step,json!({"assembly":true}));
    let error = run(&m,Default::default()).err().unwrap();
    assert_eq!(error.code,"RESOURCE_LIMIT");
    assert_eq!(error.details.unwrap()["resource_limit"],json!({
        "resource":"step_occurrences","actual":129,"allowed":128,"stage":"product_walk"}));
    let limits = StepImportLimits { max_occurrences:129,..Default::default() };
    let inspected = agent_cad::step_assembly::inspect_with_import_limits(&m.assets["source"],limits).unwrap();
    assert_eq!(inspected["definitions"],1);
    assert_eq!(inspected["instances"].as_array().unwrap().len(),129);
    let mut replay = model(inspected["model_template"].clone());replay.assets = m.assets;
    geometry(&run(&replay,limits).unwrap(),129,json!([[17,23,31],[19,25,33]]));
}
#[test]
fn single_definition_retains_entity_bytes_digest_index_and_diagnostic_gates() {
    let step = repeated_source();
    let m = import(&step,json!({"definition":true}));
    let limits = StepImportLimits { max_entities:1,..Default::default() };
    let error = run(&m,limits).err().unwrap();
    assert_eq!(error.code,"RESOURCE_LIMIT");
    assert_eq!(error.details.unwrap()["resource_limit"]["resource"],"step_entities");
    let mut bad = m.clone();bad.assets.get_mut("source").unwrap().step.push(' ');
    assert_eq!(run(&bad,Default::default()).err().unwrap().code,"ASSET_DIGEST_MISMATCH");
    let oversized = " ".repeat(24*1024*1024+1);
    let byte_error = run(&import(&oversized,json!({"definition":true})),Default::default()).err().unwrap();
    assert_eq!(byte_error.code,"RESOURCE_LIMIT");
    assert_eq!(byte_error.details.unwrap()["resource"],"stepBytes");
    let bad_index = import(&step,json!({"definition":true,"body_index":1}));
    assert_eq!(run(&bad_index,Default::default()).err().unwrap().code,"INVALID_REFERENCE");
    let incompatible = import(&step,json!({"definition":true,"assembly":true}));
    assert_eq!(run(&incompatible,Default::default()).err().unwrap().code,"INVALID_IMPORT_MODE");
    let diagnostic = step.replace("SI_UNIT(.MILLI.,.METRE.)","SI_UNIT(.MILLI.,.SECOND.)");
    assert_ne!(diagnostic,step);
    assert!(run(&import(&diagnostic,json!({"definition":true})),Default::default()).is_err());
}
#[test]
fn single_definition_still_bounds_all_source_definitions() {
    let source = model(json!({"version":1,"units":"mm","parameters":{},
        "features":[{"id":"a","op":"box","origin":[0,0,0],"size":[2,2,2]},
                    {"id":"b","op":"box","origin":[0,0,0],"size":[3,3,3]}],
        "assembly":[{"id":"a","input":"a"},{"id":"b","input":"b"}]}));
    let m = import(&build(&source).unwrap().step,json!({"definition":true}));
    let error = run(&m,StepImportLimits { max_definitions:1,..Default::default() }).err().unwrap();
    assert_eq!(error.code,"RESOURCE_LIMIT");
    assert_eq!(error.details.unwrap()["resource_limit"],json!({
        "resource":"step_definitions","actual":2,"allowed":1,"stage":"external_map"}));
}

// -- 单槽准入与每次body选择分离；不继承不同模式的权限。 --
fn cached_model(step: &str, second: Value) -> Model {
    let mut m = import(step,json!({"definition":true}));
    let asset = m.assets["source"].clone();m.assets.insert("alias".into(),asset);
    m.features.push(serde_json::from_value(json!({"id":"next","op":"import_step",
        "asset":"alias","definition":second["definition"],"body_index":second["body_index"]})).unwrap());
    m.output=String::new();
    m.assembly=serde_json::from_value(json!([{"id":"a","input":"part"},
        {"id":"b","input":"next","translation":[10,0,0]}])).unwrap();
    m
}
#[test]
fn cached_129_source_definitions_reuse_only_admitted_local_geometry() {
    let step=repeated_source();let m=cached_model(&step,json!({"definition":true,"body_index":0}));
    let b=run(&m,Default::default()).unwrap();geometry(&b,2,json!([[0,0,0],[12,2,2]]));
    assert_eq!(b.report["step_source_cache"]["parse_calls"],1);
    assert_eq!(b.report["step_source_cache"]["hits"],1);
    assert_eq!(b.report["step_source_cache"]["retained_models"],0);
    let bad=cached_model(&step,json!({"definition":true,"body_index":1}));
    let err=run(&bad,Default::default()).err().unwrap();
    assert_eq!(err.code,"INVALID_REFERENCE");assert_eq!(err.feature_id.as_deref(),Some("next"));
}
#[test]
fn cached_local_definition_never_authorizes_unapplied_whole_source() {
    let step=repeated_source();let m=cached_model(&step,json!({"definition":false,"body_index":0}));
    let err=run(&m,Default::default()).err().unwrap();
    assert_eq!(err.code,"RESOURCE_LIMIT");assert_eq!(err.feature_id.as_deref(),Some("next"));
    assert_eq!(err.details.unwrap()["resource_limit"]["resource"],"step_occurrences");
    let err=run(&m,StepImportLimits{max_occurrences:129,..Default::default()}).err().unwrap();
    assert_eq!(err.code,"IMPORT_INCOMPLETE");assert_eq!(err.feature_id.as_deref(),Some("next"));
}

#[test]
fn public_equivalent_washer129_preserves_local_cache_and_whole_controls() {
    let step=include_str!("fixtures/washer129-colocad.step");
    let volume=std::f64::consts::PI*(4.5_f64.powi(2)-2.2_f64.powi(2))*1.5;
    let single=run(&import(step,json!({"definition":true})),Default::default()).unwrap();
    assert_eq!(single.report["geometry"]["solids"],1);
    assert!((single.report["geometry"]["volume_mm3"].as_f64().unwrap()-volume).abs()<1e-5);
    let cached=run(&cached_model(step,json!({"definition":true,"body_index":0})),Default::default()).unwrap();
    assert_eq!(cached.report["geometry"]["solids"],2);
    assert!((cached.report["geometry"]["volume_mm3"].as_f64().unwrap()-2.*volume).abs()<1e-5);
    assert_eq!(cached.report["step_source_cache"]["parse_calls"],1);
    assert_eq!(cached.report["step_source_cache"]["hits"],1);
    let whole=import(step,json!({"assembly":true}));
    let err=run(&whole,Default::default()).err().unwrap();
    assert_eq!(err.code,"RESOURCE_LIMIT");
    assert_eq!(err.details.unwrap()["resource_limit"],json!({"resource":"step_occurrences","actual":129,"allowed":128,"stage":"product_walk"}));
    let limits=StepImportLimits{max_occurrences:129,..Default::default()};
    let inspected=agent_cad::step_assembly::inspect_with_import_limits(&whole.assets["source"],limits).unwrap();
    let mut replay=model(inspected["model_template"].clone());replay.assets=whole.assets;
    let all=run(&replay,limits).unwrap();
    assert_eq!(all.report["geometry"]["solids"],129);
    assert!((all.report["geometry"]["volume_mm3"].as_f64().unwrap()-129.*volume).abs()<1e-4);
}
