//! [INPUT]: 公开STEP、原生build/inspect及完整来源准入。
//! [OUTPUT]: 解析次数红绿、全部实体/位姿、不同body选择和来源/模式拒绝控制。
//! [POS]: 同次构建最近来源复用的公开行为测试；仅Windows执行。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use agent_cad::{build, Model, StepAsset};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::Write;

const BLOCK: &str = include_str!("fixtures/occt-drilled-block.step");
const NESTED: &str = include_str!("fixtures/nested-assembly.step");

fn source(text: &str) -> StepAsset {
    StepAsset { sha256: format!("{:x}", Sha256::digest(text.as_bytes())), step: text.into() }
}
// 每行分别指定来源ID、definition模式和body索引；几何仍走真实原生入口。
fn scene(assets: &[(&str, &str)], rows: &[(&str, bool, usize)]) -> Model {
    let assets = assets.iter().map(|(id, text)| ((*id).into(), serde_json::to_value(source(text)).unwrap())).collect::<serde_json::Map<_, _>>();
    let features = rows.iter().enumerate().map(|(i, (asset, definition, body))| json!({
        "id":format!("part-{i}"),"op":"import_step","asset":asset,"definition":definition,"body_index":body})).collect::<Vec<_>>();
    let assembly = rows.iter().enumerate().map(|(i, _)| json!({
        "id":format!("logical-{i}"),"input":format!("part-{i}"),"translation":[i as f64*32.,0,0]})).collect::<Vec<_>>();
    serde_json::from_value(json!({"version":1,"units":"mm","parameters":{},"assets":assets,"features":features,"assembly":assembly})).unwrap()
}
fn cache(report: &Value, parses: usize, hits: usize) {
    let c = &report["step_source_cache"];
    eprintln!("STEP_SOURCE_CACHE expected_parses={parses} expected_hits={hits} actual={c}");
    assert_eq!(c["scope"], "native_build_adjacent_part_definition_imports");
    assert_eq!(c["parse_calls"], parses);
    assert_eq!(c["hits"], hits);
    assert_eq!(c["capacity"], 1);
    assert_eq!(c["retained_models"], 0, "release source data before final queries/STEP/display");
}
fn properties(report: &Value, solids: usize, volume: f64, max: [f64; 3]) {
    assert_eq!(report["validation"]["valid"], true);
    assert_eq!(report["validation"]["manifold"], true);
    assert_eq!(report["geometry"]["solids"], solids);
    assert!((report["geometry"]["volume_mm3"].as_f64().unwrap()-volume).abs()<1e-4);
    for (i, corner) in [[0.; 3], max].iter().enumerate() { for (j, value) in corner.iter().enumerate() {
        assert!((report["geometry"]["bbox_mm"][i][j].as_f64().unwrap()-value).abs()<1e-5);
    }}
}
fn evidence(name: &str, bytes: &[u8]) {
    if let Some(path) = std::env::var_os("COLOCAD_IMPORT_CACHE_EVIDENCE") {
        let root = std::path::PathBuf::from(path); std::fs::create_dir_all(&root).unwrap();
        let mut file = std::fs::OpenOptions::new().create_new(true).write(true).open(root.join(name)).unwrap();
        file.write_all(bytes).unwrap();
    }
}

#[test]
fn adjacent_source_definitions_parse_once_and_keep_every_solid() {
    let text = BLOCK.replacen("ISO-10303-21;", &("ISO-10303-21;\n/*".to_string()+&"x".repeat(1024*1024)+"*/\n"), 1);
    let rows = vec![("source", true, 0); 32];
    let model = scene(&[("source", &text)], &rows);
    let before = model.assets["source"].sha256.clone();
    let result = build(&model).unwrap();
    properties(&result.report, 32, 32.*(3200.-40.*std::f64::consts::PI), [32.*31.+20.,16.,10.]);
    let instances = result.report["assembly"]["instances"].as_array().unwrap();
    assert_eq!(instances.len(), 32);
    for (i, instance) in instances.iter().enumerate() {
        assert_eq!(instance["id"], json!(format!("logical-{i}")));
        assert_eq!(instance["source_feature"], json!(format!("part-{i}")));
        assert_eq!(instance["world_placement"]["origin_mm"], json!([32.*i as f64,0.,0.]));
        assert_eq!(instance["world_placement"]["axes"], json!([[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]]));
        assert_eq!(instance["solids"], 1);
        for (corner, values) in [[32.*i as f64,0.,0.],[32.*i as f64+20.,16.,10.]].iter().enumerate() {
            for (axis, value) in values.iter().enumerate() {
                assert!((instance["bbox_mm"][corner][axis].as_f64().unwrap()-value).abs()<1e-5);
            }
        }
    }
    assert_eq!(source(&model.assets["source"].step).sha256, before);
    // 计数红例前保存完整公开几何回执，不能把前置几何/API失败误记为性能红例。
    evidence("definitions32-source.step", text.as_bytes());
    evidence("definitions32-output.step", result.step.as_bytes());
    evidence("definitions32-report.json", &serde_json::to_vec_pretty(&result.report).unwrap());
    cache(&result.report, 1, 31);
}

#[test]
fn exact_source_aliases_share_parsing_without_merging_feature_ids() {
    let model = scene(&[("a", BLOCK), ("b", BLOCK)], &[("a", true, 0), ("b", true, 0)]);
    let result = build(&model).unwrap();
    properties(&result.report, 2, 2.*(3200.-40.*std::f64::consts::PI), [52.,16.,10.]);
    let instances = result.report["assembly"]["instances"].as_array().unwrap();
    assert_eq!(instances.len(), 2);
    assert_ne!(instances[0]["id"], instances[1]["id"]);
    cache(&result.report, 1, 1);
    evidence("reference.step", BLOCK.as_bytes());
    evidence("aliases-output.step", result.step.as_bytes());
    evidence("aliases-report.json", &serde_json::to_vec_pretty(&result.report).unwrap());
}

#[test]
fn switching_sources_evicts_the_only_slot_and_reparses_on_return() {
    let other = BLOCK.replacen("ISO-10303-21;", "ISO-10303-21;\n/* another exact source */", 1);
    let result = build(&scene(&[("a", BLOCK), ("b", &other)], &[("a", true, 0), ("b", true, 0), ("a", true, 0)])).unwrap();
    properties(&result.report, 3, 3.*(3200.-40.*std::f64::consts::PI), [84.,16.,10.]);
    cache(&result.report, 3, 0);
}

#[test]
fn another_geometry_operation_releases_the_source_before_constructing_material() {
    let mut model = scene(&[("source", BLOCK)], &[("source", true, 0), ("source", true, 0)]);
    model.features.insert(1, serde_json::from_value(json!({
        "id":"bracket","op":"box","origin":[0,0,0],"size":[1,1,1]})).unwrap());
    model.assembly.push(serde_json::from_value(json!({
        "id":"new-material","input":"bracket","translation":[64,0,0]})).unwrap());
    let result = build(&model).unwrap();
    properties(&result.report, 3, 2.*(3200.-40.*std::f64::consts::PI)+1., [65.,16.,10.]);
    cache(&result.report, 2, 0);
}

#[test]
fn an_accepted_definition_does_not_authorize_an_unapplied_part_import() {
    let model = scene(&[("source", NESTED)], &[("source", true, 0), ("source", false, 0)]);
    let error = build(&model).err().expect("ordinary part mode must retain its stricter assembly gate");
    assert_eq!(error.code, "IMPORT_INCOMPLETE");
    assert_eq!(error.feature_id.as_deref(), Some("part-1"));
}

#[test]
fn invalid_later_body_index_never_reuses_the_previous_selected_body() {
    let model = scene(&[("source", BLOCK)], &[("source", true, 0), ("source", true, 1)]);
    let error = build(&model).err().expect("full source table bounds remain checked per selection");
    assert_eq!(error.code, "INVALID_REFERENCE");
    assert_eq!(error.feature_id.as_deref(), Some("part-1"));
}

#[test]
fn changed_source_bytes_with_an_old_digest_are_rejected_before_reuse() {
    let other = BLOCK.replacen("ISO-10303-21;", "ISO-10303-21;\n/* changed */", 1);
    let mut model = scene(&[("a", BLOCK), ("b", &other)], &[("a", true, 0), ("b", true, 0)]);
    let old_digest = model.assets["a"].sha256.clone();
    model.assets.get_mut("b").unwrap().sha256 = old_digest;
    assert_eq!(build(&model).err().unwrap().code, "ASSET_DIGEST_MISMATCH");
}

#[test]
fn a_new_build_has_no_permission_or_parsed_data_from_the_previous_invocation() {
    let model = scene(&[("source", BLOCK)], &[("source", true, 0), ("source", true, 0)]);
    for _ in 0..2 { cache(&build(&model).unwrap().report, 1, 1); }
}

#[test]
fn cached_source_still_selects_each_distinct_body_and_its_geometry() {
    let original: Model = serde_json::from_value(json!({"version":1,"units":"mm","parameters":{},
        "features":[{"id":"a","op":"box","origin":[0,0,0],"size":[1,2,3]},
            {"id":"b","op":"box","origin":[0,0,0],"size":[2,3,4]}],
        "assembly":[{"id":"a","input":"a"},{"id":"b","input":"b","translation":[20,0,0]}]})).unwrap();
    let seed = build(&original).unwrap();
    let mut model = scene(&[("source", &seed.step)], &[("source", true, 0), ("source", true, 1)]);
    // 共同原点的属性不依赖来源body表排序，也不能合并重叠的两个实体。
    let origin = model.assembly[0].translation.clone(); model.assembly[1].translation = origin;
    let result = build(&model).unwrap();
    properties(&result.report, 2, 30., [2.,3.,4.]);
    cache(&result.report, 1, 1);
}
