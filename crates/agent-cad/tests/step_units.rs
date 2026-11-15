//! [INPUT]: 明确上下文与混合mm/cm的合成盒体、真实公开inspect/build入口。
//! [OUTPUT]: 单位归一、CDSR/mapped世界位姿及拒绝缺失/冲突上下文回归。
//! [POS]: B4公开行为验收；期望数值手算，不读取私有硬件。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
#[path = "support/step_units.rs"]
mod fixture;
use agent_cad::{build, step_assembly, Model, StepAsset};
use fixture::Fixture;
use serde_json::Value;
use sha2::{Digest, Sha256};
use step_import::part21::Param;
fn asset(text: String) -> StepAsset {
    StepAsset {
        sha256: format!("{:x}", Sha256::digest(text.as_bytes())),
        step: text,
    }
}
fn replay(text: String, name: &str) -> (Value, agent_cad::Build) {
    if let Ok(path) = std::env::var("COLOCAD_UNIT_EVIDENCE_DIR") {
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(
            std::path::Path::new(&path).join(format!("{name}.step")),
            &text,
        )
        .unwrap();
    }
    let source = asset(text);
    let info = step_assembly::inspect(&source).unwrap();
    let mut m: Model = serde_json::from_value(info["model_template"].clone()).unwrap();
    m.assets.insert("source".into(), source);
    let result = build(&m).unwrap();
    if let Ok(path) = std::env::var("COLOCAD_UNIT_EVIDENCE_DIR") {
        std::fs::write(
            std::path::Path::new(&path).join(format!("{name}-output.step")),
            &result.step,
        )
        .unwrap();
    }
    (info, result)
}
fn bounds(v: &Value, want: [[f64; 3]; 2]) {
    for i in 0..2 {
        for j in 0..3 {
            assert!((v[i][j].as_f64().unwrap() - want[i][j]).abs() < 1e-5, "{v}");
        }
    }
}
#[test]
fn same_length_different_angle_contexts_preserve_explicit_body_mm() {
    let mut f = Fixture::new();
    f.box_body(false, true);
    let (_, b) = replay(f.text(), "same-length-angle");
    bounds(
        &b.report["geometry"]["bbox_mm"],
        [[0., 0., 0.], [2., 3., 4.]],
    );
    assert!((b.report["geometry"]["volume_mm3"].as_f64().unwrap() - 24.).abs() < 1e-6);
}
#[test]
fn mixed_length_contexts_remain_explicitly_unsupported() {
    let mut f = Fixture::new();
    f.box_body(true, false);
    f.box_body(false, true);
    let source = asset(f.text());
    let error = step_assembly::inspect(&source).unwrap_err();
    assert_eq!(error.code, "IMPORT_INCOMPLETE");
    assert!(error.message.contains("mixed length"), "{error:?}");
    let m:Model=serde_json::from_value(serde_json::json!({"version":1,"units":"mm","parameters":{},"assets":{"source":source},"features":[{"id":"body","op":"import_step","asset":"source","definition":true}],"output":"body"})).unwrap();
    assert_eq!(build(&m).err().unwrap().code, "IMPORT_INCOMPLETE");
}
#[test]
fn same_length_different_uncertainties_normalize_all_bodies() {
    let mut f = Fixture::new();
    let a = f.box_body(false, false);
    let b = f.box_body(false, false);
    f.uncertainty(a.rep, 1e-7);
    f.uncertainty(b.rep, 1e-6);
    let (info, result) = replay(f.text(), "same-length-uncertainty");
    assert_eq!(info["definitions"], 2);
    bounds(
        &result.report["geometry"]["bbox_mm"],
        [[0., 0., 0.], [2., 3., 4.]],
    );
    assert!((result.report["geometry"]["volume_mm3"].as_f64().unwrap() - 48.).abs() < 1e-6);
}
#[test]
fn uniformly_centimetre_body_is_normalized_once() {
    let mut f = Fixture::new();
    f.box_body(true, false);
    let (_, b) = replay(f.text(), "uniform-cm");
    bounds(
        &b.report["geometry"]["bbox_mm"],
        [[0., 0., 0.], [20., 30., 40.]],
    );
    assert!((b.report["geometry"]["volume_mm3"].as_f64().unwrap() - 24000.).abs() < 1e-5);
}
#[test]
fn same_length_contexts_preserve_nested_mapping_cdsr_and_shared_template() {
    let mut f = Fixture::new();
    let body = f.box_body(false, false);
    let source = f.frame([1., 2., 0.], 1);
    let one = f.mapped(body.rep, source, [30., 40., 0.], 2);
    let two = f.mapped(body.rep, source, [80., 40., 0.], 2);
    let midctx = f.context(false, true);
    let from = f.frame([5., 0., 0.], 1);
    let mid = f.representation(&[one, two, from], midctx);
    f.replace_product_rep(body.product, mid);
    let rootctx = f.context(false, false);
    let to = f.frame([10., 20., 0.], 2);
    let rep = f.representation(&[to], rootctx);
    let root = f.product(rep);
    f.occurrence(root, body.product, mid, rep, from, to);
    let (info, b) = replay(f.text(), "nested-same-length");
    assert_eq!(info["definitions"], 1);
    let instances = info["instances"].as_array().unwrap();
    assert_eq!(instances.len(), 2);
    let mut origins = instances
        .iter()
        .map(|i| {
            i["world_placement"]["origin_mm"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    origins.sort_by(|a, b| a[1].total_cmp(&b[1]));
    assert_eq!(origins, vec![vec![-29., 47., 0.], vec![-29., 97., 0.]]);
    for i in instances {
        assert_eq!(
            i["world_placement"]["axes"],
            serde_json::json!([[-1., 0., 0.], [0., -1., 0.], [0., 0., 1.]])
        );
    }
    bounds(
        &b.report["geometry"]["bbox_mm"],
        [[-31., 44., 0.], [-29., 97., 4.]],
    );
    assert!((b.report["geometry"]["volume_mm3"].as_f64().unwrap() - 48.).abs() < 1e-5);
    assert_eq!(b.step.matches("MANIFOLD_SOLID_BREP(").count(), 1);
}
#[test]
fn body_without_explicit_context_is_still_rejected() {
    let mut f = Fixture::new();
    let a = f.box_body(false, false);
    f.record_mut(a.rep, "ADVANCED_BREP_SHAPE_REPRESENTATION")
        .params[1] = Param::List(vec![]);
    assert_eq!(
        step_assembly::inspect(&asset(f.text())).unwrap_err().code,
        "IMPORT_INCOMPLETE"
    );
}
#[test]
fn one_body_in_conflicting_contexts_is_rejected() {
    let mut f = Fixture::new();
    let a = f.box_body(false, false);
    let ctx = f.context(true, false);
    f.representation(&[a.body], ctx);
    assert_eq!(
        step_assembly::inspect(&asset(f.text())).unwrap_err().code,
        "IMPORT_INCOMPLETE"
    );
}

#[test]
fn unrelated_dangling_reference_is_not_treated_as_context_information() {
    let mut f = Fixture::new();
    f.box_body(false, true);
    f.add("REPRESENTATION_MAP(#999999,#999998)");
    assert_eq!(
        step_assembly::inspect(&asset(f.text())).unwrap_err().code,
        "IMPORT_INCOMPLETE"
    );
}
#[test]
fn plain_part_import_does_not_discard_assembly_transforms() {
    let mut f = Fixture::new();
    let body = f.box_body(false, true);
    let origin = f.frame([0., 0., 0.], 0);
    let mapped = f.mapped(body.rep, origin, [10., 0., 0.], 0);
    let ctx = f.context(false, false);
    let rep = f.representation(&[mapped], ctx);
    f.replace_product_rep(body.product, rep);
    let source = asset(f.text());
    let m:Model=serde_json::from_value(serde_json::json!({"version":1,"units":"mm","parameters":{},"assets":{"source":source},"features":[{"id":"body","op":"import_step","asset":"source"}],"output":"body"})).unwrap();
    assert_eq!(build(&m).err().unwrap().code, "IMPORT_INCOMPLETE");
}

#[test]
fn repeated_identical_context_membership_is_not_ambiguous() {
    let mut f = Fixture::new();
    let a = f.box_body(false, false);
    let identical = f.context(false, false);
    f.representation(&[a.body], identical);
    let (info, result) = replay(f.text(), "identical-context-membership");
    assert_eq!(info["definitions"], 1);
    assert_eq!(info["instances"].as_array().unwrap().len(), 1);
    bounds(
        &result.report["geometry"]["bbox_mm"],
        [[0., 0., 0.], [2., 3., 4.]],
    );
}

#[test]
fn conflicting_uncertainties_on_one_body_are_not_identical_contexts() {
    let mut f = Fixture::new();
    let a = f.box_body(false, false);
    f.uncertainty(a.rep, 1e-7);
    let other = f.context(false, false);
    let rep = f.representation(&[a.body], other);
    f.uncertainty(rep, 1e-6);
    let error = step_assembly::inspect(&asset(f.text())).unwrap_err();
    assert_eq!(error.code, "IMPORT_INCOMPLETE");
    assert!(error
        .message
        .contains("conflicting representation contexts"));
}

#[test]
fn context_without_length_does_not_inherit_an_unrelated_file_unit() {
    let mut f = Fixture::new();
    let a = f.box_body(false, false);
    let ctx = f
        .record_mut(a.rep, "ADVANCED_BREP_SHAPE_REPRESENTATION")
        .params[2]
        .as_ref_id()
        .unwrap();
    let units = &mut f.record_mut(ctx, "GLOBAL_UNIT_ASSIGNED_CONTEXT").params[0];
    let Param::List(items) = units else {
        panic!("unit list required");
    };
    items.remove(0);
    let error = step_assembly::inspect(&asset(f.text())).unwrap_err();
    assert_eq!(error.code, "IMPORT_INCOMPLETE");
    assert!(error.message.contains("explicit length unit"));
}
