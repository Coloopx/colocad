//! [INPUT]: 公开闭合钻孔块及仅cap环引用顺序变化的派生STEP。
//! [OUTPUT]: 完整native inspect/template build/export/reopen与几何恒等回归，可选create_new公开产物证据。
//! [POS]: G3-A闭合交付正例；全链运行仅由Win统一队列执行。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use agent_cad::{build_revision_with_options, Model, StepAsset};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use solidcore::topology::brep::validation::{check_brep, CheckLevel};
use solidcore::Kernel;
use std::io::Write;
use std::path::Path;

const ORIGINAL: &str = include_str!("fixtures/occt-drilled-block.step");
const INNER_FIRST: &str = include_str!("fixtures/step-bound-order/drilled-block-inner-first.step");
const PROVENANCE: &str =
    include_str!("fixtures/step-bound-order/drilled-block-inner-first.provenance.json");

fn asset(text: &str) -> StepAsset {
    StepAsset {
        sha256: format!("{:x}", Sha256::digest(text.as_bytes())),
        step: text.to_owned(),
    }
}

fn template_build(text: &str) -> agent_cad::Build {
    let source = asset(text);
    let info = agent_cad::step_assembly::inspect(&source).unwrap();
    assert_eq!(info["definitions"], 1);
    assert_eq!(info["instances"].as_array().unwrap().len(), 1);
    let mut model: Model = serde_json::from_value(info["model_template"].clone()).unwrap();
    model.assets.insert("source".into(), source);
    build_revision_with_options(&model, &mut Default::default(), Some(false)).unwrap()
}

fn write_new(directory: &Path, name: &str, bytes: &[u8]) {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join(name))
        .expect("evidence file must be new");
    file.write_all(bytes).expect("write complete evidence");
}

fn retain_public_evidence(
    original: &agent_cad::Build,
    derived: &agent_cad::Build,
    original_readback: &agent_cad::Build,
    derived_readback: &agent_cad::Build,
) {
    let Some(directory) = std::env::var_os("COLOCAD_BOUND_ORDER_EVIDENCE_DIR") else {
        return;
    };
    assert!(!directory.is_empty(), "explicit new evidence directory");
    let directory = std::path::PathBuf::from(directory);
    if let Some(parent) = directory.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::create_dir(&directory).expect("preserve any prior evidence directory");
    let mut files = Vec::new();
    let mut save = |name: &str, bytes: &[u8]| {
        write_new(&directory, name, bytes);
        files.push(
            json!({"path":name,"bytes":bytes.len(),"sha256":format!("{:x}",Sha256::digest(bytes))}),
        );
    };
    let inputs = [
        ("original", "original-input.step", ORIGINAL),
        ("inner-first", "inner-first-input.step", INNER_FIRST),
    ];
    for (_, name, text) in inputs {
        save(name, text.as_bytes());
    }
    save("fixture-provenance.json", PROVENANCE.as_bytes());
    let builds = [
        ("original-first", original),
        ("inner-first-first", derived),
        ("original-reopened", original_readback),
        ("inner-first-reopened", derived_readback),
    ];
    for (label, build) in builds {
        save(&format!("{label}.step"), build.step.as_bytes());
        save(
            &format!("{label}.report.json"),
            &serde_json::to_vec_pretty(&build.report).unwrap(),
        );
    }
    drop(save);
    let optional_identity = |name| {
        std::env::var(name).ok().map(|value| {
            assert!(
                value.len() == 64 && value.bytes().all(|c| c.is_ascii_hexdigit()),
                "manifest SHA must be hex"
            );
            value
        })
    };
    let manifest = json!({"schema":"colocad.g3a.public-closed-artifacts.v1",
        "execution_kind":"Rust_integration_test_native_API_not_CLI", "phase":"green",
        "status":"native_closed_test_delivery_passed", "models":0,
        "source_manifest_sha256":optional_identity("COLOCAD_BOUND_ORDER_SOURCE_MANIFEST_SHA256"),
        "vendor_manifest_sha256":optional_identity("COLOCAD_BOUND_ORDER_VENDOR_MANIFEST_SHA256"),
        "test_binary_sha256":format!("{:x}",Sha256::digest(std::fs::read(std::env::current_exe().unwrap()).unwrap())),
        "inputs":inputs.iter().map(|(label,name,text)|json!({"label":label,"step":name,"bytes":text.len(),"sha256":asset(text).sha256})).collect::<Vec<_>>(),
        "deliveries":builds.iter().map(|(label,build)|json!({"label":label,"step":format!("{label}.step"),
            "bytes":build.step.len(),"sha256":asset(&build.step).sha256,"native_report":build.report})).collect::<Vec<_>>(),
        "files":files,"independent_OCCT":"pending_separate_verifier","private_G3_acceptance":"not_claimed"});
    let bytes = serde_json::to_vec_pretty(&manifest).unwrap();
    // 最后写manifest：中断只留下partial目录，不能伪造完整证据。
    write_new(&directory, "manifest.json", &bytes);
    write_new(&directory, "receipt.json", &bytes);
}

fn geometry(report: &Value) {
    assert_eq!(report["validation"]["valid"], true);
    assert_eq!(report["validation"]["manifold"], true);
    assert_eq!(report["geometry"]["solids"], 1);
    let expected = 20.0 * 16.0 * 10.0 - std::f64::consts::PI * 2.0_f64.powi(2) * 10.0;
    assert!((report["geometry"]["volume_mm3"].as_f64().unwrap() - expected).abs() < 1e-4);
    for (i, corner) in [[0.0, 0.0, 0.0], [20.0, 16.0, 10.0]].iter().enumerate() {
        for (j, coordinate) in corner.iter().enumerate() {
            assert!(
                (report["geometry"]["bbox_mm"][i][j].as_f64().unwrap() - coordinate).abs() < 1e-5
            );
        }
    }
}

#[test]
fn public_closed_fixture_only_changes_two_cap_bound_lists() {
    let provenance: Value = serde_json::from_str(PROVENANCE).unwrap();
    assert_eq!(provenance["source"]["sha256"], asset(ORIGINAL).sha256);
    assert_eq!(provenance["derived"]["sha256"], asset(INNER_FIRST).sha256);
    let before = step_import::part21::parse(ORIGINAL.as_bytes(), &Default::default()).unwrap();
    let after = step_import::part21::parse(INNER_FIRST.as_bytes(), &Default::default()).unwrap();
    assert_eq!(before.entities.len(), after.entities.len());
    let mut changed = vec![];
    for (id, original) in &before.entities {
        let mut derived = after.entities.get(id).unwrap().clone();
        if [213, 342].contains(id) {
            let step_import::part21::Instance::Simple(record) = &mut derived else {
                panic!("simple cap face");
            };
            let step_import::part21::Param::List(bounds) = &mut record.params[1] else {
                panic!("bound list");
            };
            assert_eq!(bounds.len(), 2);
            bounds.reverse();
            changed.push(*id);
        }
        assert_eq!(
            &derived, original,
            "geometry, topology orientation and every other record must stay exact: #{id}"
        );
    }
    assert_eq!(changed, [213, 342]);
}

#[test]
fn public_closed_inner_first_is_a_complete_native_delivery() {
    let original = template_build(ORIGINAL);
    let derived = template_build(INNER_FIRST);
    geometry(&original.report);
    geometry(&derived.report);
    assert!(derived.step.contains("MANIFOLD_SOLID_BREP("));
    assert!(derived.step.contains("CLOSED_SHELL("));
    // 两次fresh执行均经历原生首轮检查，不重用被validator更新的BRep。
    let reopened = template_build(&derived.step);
    geometry(&reopened.report);
    let original_reopened = template_build(&original.step);
    geometry(&original_reopened.report);
    assert_eq!(
        original.report["geometry"]["faces"],
        derived.report["geometry"]["faces"]
    );
    retain_public_evidence(&original, &derived, &original_reopened, &reopened);
}

#[test]
fn public_closed_native_topology_has_no_missing_or_loose_entities() {
    let mut counts = vec![];
    for text in [ORIGINAL, INNER_FIRST] {
        let model = step_import::import(text.as_bytes(), &Default::default()).unwrap();
        assert_eq!(model.bodies.len(), 1);
        assert_eq!(model.bodies[0].faces.len(), 7);
        let frozen = model.bodies[0].clone();
        let mut made =
            step_brep::build_from_topology(&model.bodies[0], Default::default()).unwrap();
        assert!(made.face_failures.is_empty());
        assert!(solidcore::query::api_queries::brep_is_manifold_solid(&made.brep).unwrap());
        assert!(
            check_brep(&mut Kernel::default(), &mut made.brep, CheckLevel::Level1)
                .unwrap()
                .is_valid()
        );
        counts.push((
            made.brep.face_list().len(),
            made.brep.edge_list().len(),
            made.brep.vertex_list().len(),
        ));
        assert_eq!(model.bodies[0], frozen);
    }
    assert_eq!(
        counts[0], counts[1],
        "cap reorder must not leave loose topology from retained arrays"
    );
}
