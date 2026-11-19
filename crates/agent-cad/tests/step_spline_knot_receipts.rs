//! [INPUT]: 合成的原始 B 样条节点向量与原生曲线校验。
//! [OUTPUT]: 合法曲线行为不变，失效端点保留原始和分组重数。
//! [POS]: STEP 曲线重建失败的诊断回归。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use solidcore::{
    geom::assert_array::{AssertArray, ObjectKey},
    nurbs::{
        b_spline_curve::BSplineCurve,
        math::{CPoint, NOW},
        nurbs_crv::allocate_nurb_curve,
    },
    Kernel,
};

fn validate(knots: [f64; 5]) -> (bool, AssertArray) {
    let mut raw = allocate_nurb_curve(2, 1, 4).unwrap();
    for (i, point) in raw.cpts_mut().iter_mut().enumerate() {
        *point = CPoint::new(i as f64, 0.0, 0.0, NOW);
    }
    raw.knots_mut().copy_from_slice(&knots);
    let curve = BSplineCurve::from_nurb(raw, 3, true);
    let mut reports = AssertArray::new();
    let valid = curve
        .assert_valid(&Kernel::default(), Some(&mut reports), true, ObjectKey(7))
        .unwrap();
    (valid, reports)
}

#[test]
fn clamped_spline_remains_valid() {
    let (ok, reports) = validate([0.0, 0.0, 0.5, 1.0, 1.0]);
    assert!(ok, "{reports:?}");
}

#[test]
fn endpoint_knot_failure_records_actual_multiplicities() {
    let (ok, reports) = validate([0.0, 0.5, 0.7, 1.0, 1.0]);
    assert!(!ok);
    let report = reports.get(0).unwrap();
    assert_eq!(report.test_index, 1);
    let receipt = report.format_log_message();
    assert!(receipt.contains("dimension=3, degree=1"));
    assert!(receipt.contains("grouped_start=1, grouped_end=2, raw_start=1, raw_end=2"));
}

#[test]
fn close_interior_knot_reports_grouped_and_raw_counts_separately() {
    let (ok, reports) = validate([0.0, 0.0, 1.0e-14, 1.0, 1.0]);
    assert!(!ok);
    let receipt = reports.get(0).unwrap().format_log_message();
    assert!(receipt.contains("grouped_start=3, grouped_end=2, raw_start=2, raw_end=2"));
}

#[test]
fn synthetic_step_edges_with_close_knots_keep_the_box() {
    use agent_cad::{build, Model};
    use serde_json::json;
    use solidcore::curve::curve::CurveKind;
    use solidcore::geom::attribute::{AttributeStore, ObjectKey};
    use solidcore::mass::integrator::properties::{brep_volume, PropertyAttributes};
    use step_import::model::{BSplineCurve, Curve};

    let model: Model = serde_json::from_value(json!({
        "version": 1, "units": "mm", "parameters": {},
        "features": [{"id":"box","op":"box","origin":[0,0,0],"size":[10,8,6]}], "output":"box"
    }))
    .unwrap();
    let baseline = build(&model).unwrap();
    for (degree, near_start) in [(1, false), (1, true), (3, false), (3, true)] {
        let mut neutral =
            step_import::import(baseline.step.as_bytes(), &Default::default()).unwrap();
        let body = &mut neutral.bodies[0];
        let edge = &mut body.edges[0];
        let a = body.vertices[edge.start].point;
        let b = body.vertices[edge.end].point;
        let (middle, near) = if near_start {
            (a, 1e-14)
        } else {
            (b, 1.0 - 1e-14)
        };
        let points = match degree {
            1 => vec![a, middle, b],
            3 => {
                let on_edge =
                    |t: f64| std::array::from_fn(|axis| a[axis] + t * (b[axis] - a[axis]));
                if near_start {
                    vec![a, a, on_edge(1.0 / 3.0), on_edge(2.0 / 3.0), b]
                } else {
                    vec![a, on_edge(1.0 / 3.0), on_edge(2.0 / 3.0), b, b]
                }
            }
            _ => unreachable!(),
        };
        edge.curve = Curve::BSpline(BSplineCurve {
            degree,
            points,
            weights: (degree == 3).then_some(vec![1.0, 1.0, 0.85, 1.0, 1.0]),
            knots: vec![0.0, near, 1.0],
            mults: vec![degree + 1, 1, degree + 1],
        });
        edge.same_sense = true;
        edge.t_start = 0.0;
        edge.t_end = 1.0;

        let step = step_export::write(&neutral).unwrap();
        let imported = step_import::import(step.as_bytes(), &Default::default()).unwrap();
        let mut candidate =
            step_brep::build_from_topology(&imported.bodies[0], Default::default()).unwrap();
        assert!(
            candidate.face_failures.is_empty(),
            "{:?}",
            candidate.face_failures
        );
        assert_eq!(candidate.brep.face_list().len(), 6);
        assert_eq!(candidate.brep.edge_list().len(), 12);
        let rebuilt = candidate
            .brep
            .edge_list()
            .iter()
            .filter_map(
                |&id| match candidate.brep.edge(id).unwrap().curve.as_ref()?.kind() {
                    CurveKind::BSpline(curve) => Some(curve),
                    _ => None,
                },
            )
            .collect::<Vec<_>>();
        assert_eq!(rebuilt.len(), 1);
        let multiplicities = rebuilt[0].knots(None).unwrap().1;
        assert_eq!(multiplicities.first(), Some(&((degree + 1) as usize)));
        assert_eq!(multiplicities.last(), Some(&((degree + 1) as usize)));

        let attrs = PropertyAttributes {
            store: &AttributeStore::default(),
            key_of: &|_| ObjectKey(0),
        };
        let volume = brep_volume(&Kernel::default(), &mut candidate.brep, 1e-5, &attrs).unwrap();
        assert!((volume.abs() - 480.0).abs() < 1e-8, "{volume}");

        let mut stitched = step_brep::build_body(
            &imported.bodies[0],
            step_brep::BuildOptions {
                mode: step_brep::BuildMode::StitchOnly,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(stitched.path, step_brep::BuildPath::Stitch);
        let checked = solidcore::topology::brep::validation::check_brep(
            &mut Kernel::default(),
            &mut stitched.brep,
            solidcore::topology::brep::validation::CheckLevel::Level1,
        )
        .unwrap();
        assert!(checked.is_valid(), "{}", checked.lines().join("; "));
        let volume = brep_volume(&Kernel::default(), &mut stitched.brep, 1e-5, &attrs).unwrap();
        assert!((volume.abs() - 480.0).abs() < 1e-8, "{volume}");

        for definition in [false, true] {
            let imported = import_model(&step, definition);
            assert_box_delivery(&imported);
            assert_box_delivery(&import_model(&imported.step, definition));
        }
        let delivered = import_template(&step);
        assert_box_delivery(&delivered);
        assert_box_delivery(&import_template(&delivered.step));
    }
}

fn step_asset(step: &str) -> agent_cad::StepAsset {
    use sha2::{Digest, Sha256};
    agent_cad::StepAsset {
        sha256: format!("{:x}", Sha256::digest(step.as_bytes())),
        step: step.to_owned(),
    }
}

fn import_model(step: &str, definition: bool) -> agent_cad::Build {
    let model = serde_json::from_value(serde_json::json!({
        "version": 1, "units": "mm", "parameters": {},
        "assets": {"source": step_asset(step)},
        "features": [{"id":"part","op":"import_step","asset":"source","definition":definition}],
        "output":"part"
    }))
    .unwrap();
    agent_cad::build(&model).unwrap_or_else(|error| {
        panic!(
            "definition={definition}, mapped items={}: {error:?}",
            step.matches("MAPPED_ITEM(").count()
        )
    })
}

fn import_template(step: &str) -> agent_cad::Build {
    let source = step_asset(step);
    let info = agent_cad::step_assembly::inspect(&source).unwrap();
    let mut template =
        serde_json::from_value::<agent_cad::Model>(info["model_template"].clone()).unwrap();
    template.assets.insert("source".into(), source);
    agent_cad::build(&template).unwrap()
}

fn assert_box_delivery(delivered: &agent_cad::Build) {
    let report = &delivered.report;
    assert_eq!(report["validation"]["valid"], true);
    assert_eq!(report["geometry"]["solids"], 1);
    assert!((report["geometry"]["volume_mm3"].as_f64().unwrap() - 480.0).abs() < 1e-5);
    for (side, expected) in [[0.0, 0.0, 0.0], [10.0, 8.0, 6.0]].iter().enumerate() {
        for (axis, value) in expected.iter().enumerate() {
            let actual = report["geometry"]["bbox_mm"][side][axis].as_f64().unwrap();
            assert!((actual - value).abs() < 1e-5, "{actual} != {value}");
        }
    }
}
