use agent_cad::{build, Model};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
fn m(v: Value) -> Model {
    serde_json::from_value(v).unwrap()
}
#[test]
fn nested_shared_instances_roundtrip_with_native_placements() {
    let source = m(
        json!({"version":1,"units":"mm","parameters":{},"features":[{"id":"box","op":"box","origin":[0,0,0],"size":[10,20,5]}],"groups":[{"id":"group","pose":{"translation":[100,0,0],"rotation_deg":[0,0,90]}}],"assembly":[{"id":"one","input":"box","parent":"group"},{"id":"two","input":"box","parent":"group","translation":[30,0,0],"rotation_deg":[0,0,90]}]}),
    );
    let b = build(&source).unwrap();
    assert_eq!(b.step.matches("MANIFOLD_SOLID_BREP(").count(), 1);
    assert_eq!(b.step.matches("NEXT_ASSEMBLY_USAGE_OCCURRENCE(").count(), 3);
    assert_eq!(b.report["geometry"]["solids"], 2);
    let sha = format!("{:x}", Sha256::digest(b.step.as_bytes()));
    let imported = m(
        json!({"version":1,"units":"mm","parameters":{},"assets":{"source":{"sha256":sha,"step":b.step}},"features":[{"id":"result","op":"import_step","asset":"source","assembly":true}],"output":"result"}),
    );
    let read = build(&imported).unwrap();
    assert!((read.report["geometry"]["volume_mm3"].as_f64().unwrap() - 2000.).abs() < 1e-5);
    assert_eq!(read.report["geometry"]["solids"], 2);
    for i in 0..2 {
        for j in 0..3 {
            assert!(
                (read.report["geometry"]["bbox_mm"][i][j].as_f64().unwrap()
                    - b.report["geometry"]["bbox_mm"][i][j].as_f64().unwrap())
                .abs()
                    < 1e-5
            );
        }
    }
}
#[test]
fn independent_occt_nested_asset_applies_both_transforms() {
    let step = include_str!("fixtures/nested-assembly.step");
    let sha = format!("{:x}", Sha256::digest(step));
    let source = m(
        json!({"version":1,"units":"mm","parameters":{},"assets":{"source":{"sha256":sha,"step":step}},"features":[{"id":"result","op":"import_step","asset":"source","assembly":true}],"output":"result"}),
    );
    let b = build(&source).unwrap();
    assert_eq!(b.report["geometry"]["solids"], 2);
    let bb = &b.report["geometry"]["bbox_mm"];
    for (i, v) in [[80., 0., 0.], [100., 30., 5.]].iter().enumerate() {
        for (j, expected) in v.iter().enumerate() {
            assert!((bb[i][j].as_f64().unwrap() - expected).abs() < 1e-5, "{bb}");
        }
    }
}
#[test]
fn component_native_size_is_checked_and_never_rescaled() {
    let step = include_str!("fixtures/nested-assembly.step");
    let sha = format!("{:x}", Sha256::digest(step));
    let mut source = m(
        json!({"version":1,"units":"mm","parameters":{},"assets":{"source":{"sha256":sha,"step":step}},"features":[{"id":"result","op":"component_cad","asset":"source","size":[20,30,5]}],"output":"result"}),
    );
    let b = build(&source).unwrap();
    assert!((b.report["geometry"]["bbox_mm"][0][0].as_f64().unwrap() + 10.).abs() < 1e-5);
    if let agent_cad::Feature::ComponentCad { size, .. } = &mut source.features[0] {
        size[0] = agent_cad::Scalar::Number(21.);
    }
    assert_eq!(build(&source).err().unwrap().code, "ASSET_MISMATCH");
}
#[test]
fn overlapping_pattern_preserves_independent_solids() {
    let source = m(
        json!({"version":1,"units":"mm","parameters":{},"features":[{"id":"box","op":"box","origin":[0,0,0],"size":[10,10,10]},{"id":"pattern","op":"pattern","input":"box","positions":[[0,0,0],[5,0,0]]}],"output":"pattern"}),
    );
    let b = build(&source).unwrap();
    assert_eq!(b.report["geometry"]["solids"], 2);
    assert!((b.report["geometry"]["volume_mm3"].as_f64().unwrap() - 2000.).abs() < 1e-6);
}
fn mapped_asset(scale: f64) -> String {
    let source = m(
        json!({"version":1,"units":"mm","parameters":{},"features":[{"id":"box","op":"box","origin":[0,0,0],"size":[10,10,10]}],"output":"box"}),
    );
    let step = build(&source).unwrap().step;
    let file =
        step_import::part21::parse(step.as_bytes(), &step_import::Limits::default()).unwrap();
    let rep = file
        .entities
        .iter()
        .find(|(_, r)| r.record("ADVANCED_BREP_SHAPE_REPRESENTATION").is_some())
        .unwrap();
    let rep_id = *rep.0;
    let context = rep
        .1
        .record("ADVANCED_BREP_SHAPE_REPRESENTATION")
        .unwrap()
        .params[2]
        .as_ref_id()
        .unwrap();
    let definition = file
        .entities
        .iter()
        .find_map(|(id, r)| {
            r.record("SHAPE_DEFINITION_REPRESENTATION")
                .map(|r| (*id, r.params[0].as_ref_id().unwrap()))
        })
        .unwrap();
    let mut text = step.replace(
        &format!(
            "#{}=SHAPE_DEFINITION_REPRESENTATION(#{},#{rep_id});",
            definition.0, definition.1
        ),
        &format!(
            "#{}=SHAPE_DEFINITION_REPRESENTATION(#{},#1000010);",
            definition.0, definition.1
        ),
    );
    let records=format!("#1000001=CARTESIAN_POINT('',(5.,0.,0.));\n#1000002=AXIS2_PLACEMENT_3D('',#1000001,$,$);\n#1000003=REPRESENTATION_MAP(#1000002,#{rep_id});\n#1000004=CARTESIAN_POINT('',(20.,0.,0.));\n#1000005=CARTESIAN_TRANSFORMATION_OPERATOR_3D('',$,$,#1000004,{scale},$);\n#1000006=MAPPED_ITEM('',#1000003,#1000005);\n#1000007=CARTESIAN_POINT('',(50.,0.,0.));\n#1000008=CARTESIAN_TRANSFORMATION_OPERATOR_3D('',$,$,#1000007,1.,$);\n#1000009=MAPPED_ITEM('',#1000003,#1000008);\n#1000010=SHAPE_REPRESENTATION('',(#1000006,#1000009),#{context});\n");
    text = text.replace(
        "ENDSEC;\nEND-ISO-10303-21;",
        &format!("{records}ENDSEC;\nEND-ISO-10303-21;"),
    );
    text
}
#[test]
fn mapped_items_apply_origin_and_target_and_replay_as_shared_instances() {
    let step = mapped_asset(1.);
    let asset = agent_cad::StepAsset {
        sha256: format!("{:x}", Sha256::digest(&step)),
        step,
    };
    let info = agent_cad::step_assembly::inspect(&asset).unwrap();
    assert_eq!(info["definitions"], 1);
    assert_eq!(info["instances"].as_array().unwrap().len(), 2);
    let mut model: Model = serde_json::from_value(info["model_template"].clone()).unwrap();
    model.assets.insert("source".into(), asset);
    let result = build(&model).unwrap();
    assert!((result.report["geometry"]["volume_mm3"].as_f64().unwrap() - 2000.).abs() < 1e-5);
    assert!((result.report["geometry"]["bbox_mm"][0][0].as_f64().unwrap() - 15.).abs() < 1e-5);
    assert!((result.report["geometry"]["bbox_mm"][1][0].as_f64().unwrap() - 55.).abs() < 1e-5);
    assert_eq!(result.step.matches("MANIFOLD_SOLID_BREP(").count(), 1);
}
#[test]
fn scaled_mapped_items_are_refused_instead_of_ignoring_scale() {
    let step = mapped_asset(2.);
    let asset = agent_cad::StepAsset {
        sha256: format!("{:x}", Sha256::digest(&step)),
        step,
    };
    assert_eq!(
        agent_cad::step_assembly::inspect(&asset)
            .err()
            .unwrap()
            .code,
        "STEP_ASSEMBLY"
    );
}
#[test]
fn conversion_based_inch_units_apply_to_geometry_and_placements() {
    let original = include_str!("fixtures/nested-assembly.step");
    let unit = original
        .lines()
        .find(|line| line.contains("SI_UNIT(.MILLI.,.METRE.)"))
        .unwrap();
    // OCCT writes this simple unit as a complex entity, potentially across lines.
    let file = step_import::part21::parse(original.as_bytes(), &Default::default()).unwrap();
    let ids = file
        .entities
        .iter()
        .filter(|(_, v)| v.record("LENGTH_UNIT").is_some() && v.record("SI_UNIT").is_some())
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();
    let mut step = original.to_string();
    for id in ids {
        let start = step
            .find(&format!("#{id} ="))
            .or_else(|| step.find(&format!("#{id}=")))
            .unwrap();
        let end = start + step[start..].find(';').unwrap() + 1;
        step.replace_range(
            start..end,
            &format!(
                "#{id}=(CONVERSION_BASED_UNIT('INCH',#999991) LENGTH_UNIT() NAMED_UNIT(#999993));"
            ),
        );
    }
    let records="#999990=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT($,.METRE.));\n#999991=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.0254),#999990);\n#999993=DIMENSIONAL_EXPONENTS(1.,0.,0.,0.,0.,0.,0.);\n";
    step = step.replace(
        "ENDSEC;\nEND-ISO-10303-21;",
        &format!("{records}ENDSEC;\nEND-ISO-10303-21;"),
    );
    assert!(!unit.is_empty());
    let sha = format!("{:x}", Sha256::digest(&step));
    let result=build(&m(json!({"version":1,"units":"mm","parameters":{},"assets":{"source":{"sha256":sha,"step":step}},"features":[{"id":"part","op":"import_step","asset":"source","assembly":true}],"output":"part"}))).unwrap();
    assert!(
        (result.report["geometry"]["volume_mm3"].as_f64().unwrap() - 2000. * 25.4_f64.powi(3))
            .abs()
            < 1e-3
    );
    assert!(
        (result.report["geometry"]["bbox_mm"][0][0].as_f64().unwrap() - 80. * 25.4).abs() < 1e-4
    );
}
