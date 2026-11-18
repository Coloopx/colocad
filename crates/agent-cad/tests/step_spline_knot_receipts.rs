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
