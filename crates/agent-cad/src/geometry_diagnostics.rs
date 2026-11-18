//! [INPUT]: 已完成的原生 B-rep 校验与精确间隙/容差数值。
//! [OUTPUT]: 原有失败文本及有限的高精度测量值。
//! [POS]: 零件和装配实体校验的失败诊断，不参与接受判定。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use solidcore::geom::assert_array::{labels, AssertReport};
use solidcore::math::UNDEF_DOUBLE;
use solidcore::topology::brep::validation::BrepCheck;
use std::{collections::BTreeSet, fmt::Write};

pub(crate) fn failure_message(check: &BrepCheck) -> String {
    let mut message = check.lines().join("; ");
    let mut reported = BTreeSet::new();
    for measurement in check.failures().filter_map(measurement) {
        let (name, value, tolerance) = measurement;
        if reported.insert(name) {
            let _ = write!(
                message,
                "; precise {name} [mm]: value={value:.17e}, tolerance={tolerance:.17e}"
            );
        }
    }
    message
}

fn measurement(report: &AssertReport) -> Option<(&'static str, f64, f64)> {
    let name = match (report.reporting_type, report.list_index, report.test_index) {
        (labels::TRIMMING_TOOLS_TYPE, 3, 0) => "trim endpoint gap",
        (labels::BREP_TYPE, 1, 59 | 60) => "edge-to-BRep tolerance",
        (labels::BREP_TYPE, 2, 0 | 2) => "edge-to-vertex gap",
        (labels::BREP_TYPE, 2, 1 | 3) => "vertex-to-face gap",
        _ => return None,
    };
    let value = report.value;
    let tolerance = report.tol3d;
    [value, tolerance]
        .into_iter()
        .all(|n| n.is_finite() && n != UNDEF_DOUBLE)
        .then_some((name, value, tolerance))
}

#[cfg(test)]
mod tests {
    use super::*;
    use solidcore::geom::assert_array::{AssertArray, ReportSpec};

    fn report(class: i64, list: u64, test: u64, value: f64, tolerance: f64) -> AssertReport {
        AssertReport::new(ReportSpec {
            reporter: None,
            test_class: class,
            test_index: test,
            list_index: list,
            owner: None,
            owner_type: 0,
            owner_type_string: None,
            other: None,
            other_type: 0,
            other_type_string: None,
            tol3d: tolerance,
            value,
            message: None,
        })
        .unwrap()
    }

    #[test]
    fn near_threshold_gap_preserves_distinct_native_values() {
        let tol = 0.01_f64;
        let offset = f64::from_bits(tol.to_bits() + 1);
        let mut reports = AssertArray::new();
        reports.add_unique_delete_duplicate(report(labels::BREP_TYPE, 1, 59, offset, tol));
        let check = BrepCheck {
            pointers_status: 1000,
            pointer_reports: AssertArray::new(),
            assert_valid: Some(false),
            reports,
        };
        let result = failure_message(&check);
        assert!(!check.is_valid());
        assert!(result.contains("SmBrep::Test #59"));
        assert!(result.contains(&format!("value={offset:.17e}, tolerance={tol:.17e}")));
    }

    #[test]
    fn excludes_uv_and_missing_measurements() {
        assert!(measurement(&report(labels::TRIMMING_TOOLS_TYPE, 3, 1, 1., 0.1)).is_none());
        assert!(measurement(&report(labels::BREP_TYPE, 2, 3, UNDEF_DOUBLE, 0.1)).is_none());
    }

    #[test]
    fn distinguishes_3d_endpoint_and_vertex_face_checks() {
        assert_eq!(
            measurement(&report(labels::TRIMMING_TOOLS_TYPE, 3, 0, 2.6e-5, 2.0e-5)),
            Some(("trim endpoint gap", 2.6e-5, 2.0e-5))
        );
        assert_eq!(
            measurement(&report(labels::BREP_TYPE, 2, 3, 1.6e-5, 1.5e-5)),
            Some(("vertex-to-face gap", 1.6e-5, 1.5e-5))
        );
    }
}
