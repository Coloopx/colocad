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
    for near_start in [false, true] {
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
        edge.curve = Curve::BSpline(BSplineCurve {
            degree: 1,
            points: vec![a, middle, b],
            weights: None,
            knots: vec![0.0, near, 1.0],
            mults: vec![2, 1, 2],
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
        assert_eq!(rebuilt[0].knots(None).unwrap().1, [2, 2]);

        let attrs = PropertyAttributes {
            store: &AttributeStore::default(),
            key_of: &|_| ObjectKey(0),
        };
        let volume = brep_volume(&Kernel::default(), &mut candidate.brep, 1e-5, &attrs).unwrap();
        assert!((volume.abs() - 480.0).abs() < 1e-8, "{volume}");
    }
}
