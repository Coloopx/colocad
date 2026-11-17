//! [INPUT]: 明确上下文合成盒体、公开OCCT嵌套夹具及真实inspect/build入口。
//! [OUTPUT]: 单位归一、来源几何不变的CDSR位姿及缺失/冲突上下文拒绝回归。
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
fn same_length_contexts_preserve_the_public_occt_nested_fixture() {
    let original = include_str!("fixtures/nested-assembly.step");
    assert_eq!(asset(original.into()).sha256,
        "a00e6700566f453153c2f193cfed8fa004c30c4aa1723901334c57d6240153ef");
    let before = "#394 = UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.E-07),#391,";
    let after = "#394 = UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.E-06),#391,";
    assert_eq!(original.matches(before).count(), 1);
    let changed = original.replacen(before, after, 1);
    assert_eq!(changed.replacen(after, before, 1), original,
        "only one representation uncertainty changes; coordinates, topology and transforms remain exact");
    let (info, result) = replay(changed, "occt-nested-uncertainty");
    assert_eq!(info["definitions"], 1);
    let instances = info["instances"].as_array().unwrap();
    assert_eq!(instances.len(), 2);
    // 两级CDSR：根Rz90/平移100，再分别复用原点及Rz90/平移30。
    for (origin, axes) in [
        ([100., 0., 0.], [[0., 1., 0.], [-1., 0., 0.], [0., 0., 1.]]),
        ([100., 30., 0.], [[-1., 0., 0.], [0., -1., 0.], [0., 0., 1.]]),
    ] {
        let matched = instances.iter().filter(|i| (0..3).all(|j|
            (i["world_placement"]["origin_mm"][j].as_f64().unwrap() - origin[j]).abs() < 1e-5)).collect::<Vec<_>>();
        assert_eq!(matched.len(), 1, "each original occurrence must retain its own world pose");
        for i in 0..3 { for j in 0..3 {
            assert!((matched[0]["world_placement"]["axes"][i][j].as_f64().unwrap() - axes[i][j]).abs() < 1e-9);
        }}
    }
    assert_eq!(result.report["geometry"]["solids"], 2);
    assert!((result.report["geometry"]["volume_mm3"].as_f64().unwrap() - 2000.).abs() < 1e-4);
    bounds(&result.report["geometry"]["bbox_mm"], [[80., 0., 0.], [100., 30., 5.]]);
    replay(result.step, "occt-nested-readback");
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

// ---- review8：角度元数据不能改变真实盒体的单位或几何 ----
#[test]
fn descriptive_metadata_context_preserves_public_drilled_block() {
    let metadata = include_str!("fixtures/metadata-2d.step");
    let control = metadata.lines().filter(|line| !line.starts_with("#900"))
        .collect::<Vec<_>>().join("\n") + "\n";
    for (name, text) in [("metadata-2d", metadata.to_string()), ("metadata-removed", control)] {
        let (info, result) = replay(text, name);
        assert_eq!(info["definitions"], 1);
        assert_eq!(result.report["geometry"]["solids"], 1);
        assert!((result.report["geometry"]["volume_mm3"].as_f64().unwrap()
            - (3200. - 40. * std::f64::consts::PI)).abs() < 1e-4);
        bounds(&result.report["geometry"]["bbox_mm"], [[0., 0., 0.], [20., 16., 10.]]);
    }
}
fn angle_metadata(f: &mut Fixture, dimension: u8, item: &str) -> u64 {
    let unit = f.add("(NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.))");
    let ctx = f.add(&format!("(GEOMETRIC_REPRESENTATION_CONTEXT({dimension}) GLOBAL_UNIT_ASSIGNED_CONTEXT((#{unit})) REPRESENTATION_CONTEXT('',''))"));
    let item = f.add(item);
    f.add(&format!("REPRESENTATION('settings',(#{item}),#{ctx})"))
}
#[test]
fn descriptive_metadata_is_not_excluded_by_dimension() {
    let mut f = Fixture::new();
    f.box_body(false, false);
    angle_metadata(&mut f, 3, "DESCRIPTIVE_REPRESENTATION_ITEM('angle','radian')");
    let (_, b) = replay(f.text(), "metadata-3d");
    bounds(&b.report["geometry"]["bbox_mm"], [[0., 0., 0.], [2., 3., 4.]]);
}
fn rejected(text: String, name: &str) -> agent_cad::CadError {
    let directory = std::env::var("COLOCAD_UNIT_EVIDENCE_DIR").ok();
    if let Some(path) = &directory {
        std::fs::create_dir_all(path).unwrap();
        std::fs::write(std::path::Path::new(path).join(format!("negative-{name}.step")), &text).unwrap();
    }
    let error = step_assembly::inspect(&asset(text)).unwrap_err();
    if let Some(path) = directory {
        std::fs::write(std::path::Path::new(&path).join(format!("negative-{name}.json")),
            serde_json::to_vec_pretty(&error).unwrap()).unwrap();
    }
    error
}
#[test]
fn geometry_and_unknown_items_in_2d_context_still_require_length() {
    for (index, item) in ["CARTESIAN_POINT('',(0.,0.))", "UNRECOGNIZED_REPRESENTATION_ITEM('unknown')"].into_iter().enumerate() {
        let mut f = Fixture::new();
        f.box_body(false, false);
        angle_metadata(&mut f, 2, item);
        let e = rejected(f.text(), &["geometry-2d", "unknown-item-2d"][index]);
        assert_eq!(e.code, "IMPORT_INCOMPLETE");
        assert!(e.message.contains("explicit length unit"), "{e:?}");
    }
}
#[test]
fn linked_metadata_context_still_requires_length() {
    let mut f = Fixture::new();
    let body = f.box_body(false, false);
    let metadata = angle_metadata(&mut f, 2, "DESCRIPTIVE_REPRESENTATION_ITEM('angle','radian')");
    f.add(&format!("REPRESENTATION_RELATIONSHIP('','',#{},#{metadata})", body.rep));
    let e = rejected(f.text(), "linked-metadata");
    assert_eq!(e.code, "IMPORT_INCOMPLETE");
    assert!(e.message.contains("explicit length unit"), "{e:?}");
}
#[test]
fn placement_only_mixed_length_context_is_still_rejected() {
    let mut f = Fixture::new();
    f.box_body(false, false);
    let frame = f.frame([1., 0., 0.], 0);
    let ctx = f.context(true, false);
    f.representation(&[frame], ctx);
    let e = rejected(f.text(), "placement-mixed-length");
    assert_eq!(e.code, "IMPORT_INCOMPLETE");
    assert!(e.message.contains("mixed length"), "{e:?}");
}
#[test]
fn non_rigid_mapping_is_still_rejected_with_metadata() {
    let mut f = Fixture::new();
    let body = f.box_body(false, false);
    let from = f.frame([0., 0., 0.], 0);
    let mapped = f.mapped(body.rep, from, [10., 0., 0.], 0);
    let transform = f.record_mut(mapped, "MAPPED_ITEM").params[2].as_ref_id().unwrap();
    f.record_mut(transform, "CARTESIAN_TRANSFORMATION_OPERATOR_3D").params[4] = Param::Real(2.);
    let ctx = f.context(false, false);
    let rep = f.representation(&[mapped], ctx);
    f.replace_product_rep(body.product, rep);
    angle_metadata(&mut f, 2, "DESCRIPTIVE_REPRESENTATION_ITEM('angle','radian')");
    let e = rejected(f.text(), "non-rigid-mapping");
    assert!(e.message.contains("rigid"), "{e:?}");
}

#[test]
fn unknown_associations_to_metadata_context_item_or_rep_require_length() {
    for target in 0..3 {
        let mut f = Fixture::new();
        f.box_body(false, false);
        let rep = angle_metadata(&mut f, 2, "DESCRIPTIVE_REPRESENTATION_ITEM('angle','radian')");
        let r = f.record_mut(rep, "REPRESENTATION");
        let ctx = r.params[2].as_ref_id().unwrap();
        let item = r.params[1].as_list().unwrap()[0].as_ref_id().unwrap();
        let id = [ctx, item, rep][target];
        f.add(&format!("UNRECOGNIZED_GEOMETRY(#{id})"));
        let e = rejected(f.text(), &format!("unknown-association-{target}"));
        assert_eq!(e.code, "IMPORT_INCOMPLETE");
        assert!(e.message.contains("explicit length unit"), "{e:?}");
    }
}
