//! [INPUT]: 已完成的原生 B-rep 校验与精确间隙/容差数值。
//! [OUTPUT]: 保留原有错误码和失败文本，附有限的高精度测量值。
//! [POS]: 零件和装配实体校验的失败诊断，不参与接受判定。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use crate::CadError;
use solidcore::geom::assert_array::{labels, AssertReport};
use solidcore::math::UNDEF_DOUBLE;
use solidcore::topology::brep::validation::BrepCheck;
use std::fmt::Write;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GapKind {
    EdgeLowerTolerance,
    EdgeUpperTolerance,
    EdgeToVertex,
    TrimEndpoint,
    VertexToFace,
}

impl GapKind {
    const LABELS: [&str; 5] = [
        "edge-to-BRep lower tolerance",
        "edge-to-BRep upper tolerance",
        "edge-to-vertex gap",
        "trim endpoint gap",
        "vertex-to-face gap",
    ];
}

#[cold]
#[inline(never)]
pub(crate) fn validation_failure(check: &BrepCheck, instance: Option<&str>) -> CadError {
    let lines = check.lines().join("; ");
    let mut message = match instance {
        Some(id) => format!("instance {id}: {lines}"),
        None => lines,
    };
    let mut worst: [Option<(f64, f64)>; GapKind::LABELS.len()] = [None; GapKind::LABELS.len()];
    for (kind, value, tolerance) in check.failures().filter_map(measurement) {
        let slot = &mut worst[kind as usize];
        if slot.is_none_or(|(v, tol)| (value - tolerance).abs() > (v - tol).abs()) {
            *slot = Some((value, tolerance));
        }
    }
    for (name, sample) in GapKind::LABELS.into_iter().zip(worst) {
        if let Some((value, tolerance)) = sample {
            let _ = write!(
                message,
                "; precise {name} [mm]: value={value:.17e}, tolerance={tolerance:.17e}"
            );
        }
    }
    CadError::new("GEOMETRY_VALIDATION_FAILED", message)
}

fn measurement(report: &AssertReport) -> Option<(GapKind, f64, f64)> {
    let kind = match (report.reporting_type, report.list_index, report.test_index) {
        (labels::TRIMMING_TOOLS_TYPE, 3, 0) => GapKind::TrimEndpoint,
        (labels::BREP_TYPE, 1, 59) => GapKind::EdgeUpperTolerance,
        (labels::BREP_TYPE, 1, 60) => GapKind::EdgeLowerTolerance,
        (labels::BREP_TYPE, 2, 0 | 2) => GapKind::EdgeToVertex,
        (labels::BREP_TYPE, 2, 1 | 3) => GapKind::VertexToFace,
        _ => return None,
    };
    let value = report.value;
    let tolerance = report.tol3d;
    [value, tolerance]
        .into_iter()
        .all(|n| n.is_finite() && n != UNDEF_DOUBLE)
        .then_some((kind, value, tolerance))
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
        let result = validation_failure(&check, None).message;
        assert!(!check.is_valid());
        assert!(result.contains("SmBrep::Test #59"));
        assert!(result.contains(&format!("value={offset:.17e}, tolerance={tol:.17e}")));
    }

    #[test]
    fn reports_the_worst_residual_after_a_boundary_equality() {
        let tol = 0.01_f64;
        let excess = tol + 2.0e-5;
        let mut pointer_reports = AssertArray::new();
        pointer_reports.add_unique_delete_duplicate(report(labels::BREP_TYPE, 1, 59, tol, tol));
        let mut worst = report(labels::BREP_TYPE, 1, 59, excess, tol);
        worst.owner = Some(solidcore::geom::assert_array::ObjectKey(2));
        pointer_reports.add_unique_delete_duplicate(worst);
        let check = BrepCheck {
            pointers_status: 1001,
            pointer_reports,
            assert_valid: None,
            reports: AssertArray::new(),
        };
        let result = validation_failure(&check, None).message;
        assert!(result.contains(&format!("value={excess:.17e}, tolerance={tol:.17e}")));
        assert_eq!(
            result
                .matches("precise edge-to-BRep upper tolerance")
                .count(),
            1
        );
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
            Some((GapKind::TrimEndpoint, 2.6e-5, 2.0e-5))
        );
        assert_eq!(
            measurement(&report(labels::BREP_TYPE, 2, 3, 1.6e-5, 1.5e-5)),
            Some((GapKind::VertexToFace, 1.6e-5, 1.5e-5))
        );
    }
}
