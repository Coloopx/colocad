//! [INPUT]: 自有Line、全域有理网、真实Circle/Ellipse、bounded域及Plane clamp，调用生产共用proof helper。
//! [OUTPUT]: 四个原red及十个全span/域/权重/conic正负控制，不依赖厂家CAD。
//! [POS]: G3-A independent review I1；仅Win执行，red须先观察缺陷。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use solidcore::curve::{curve::Curve, line::Line};
use solidcore::geom::axis2_placement::Axis2Placement;
use solidcore::nurbs::b_spline_curve::{BSplineCurve, BSplineCurveForm, CanonicalData, KnotType};
use solidcore::surface::{plane::Plane, surface::Surface};
use solidcore::{Kernel, Vec3};

fn plane() -> Surface {
    Plane::create_canonical(&Axis2Placement::IDENTITY)
        .unwrap()
        .into()
}
fn line(dim: u64, from: f64, to: f64) -> Curve {
    Line::create_line_segment(
        dim,
        Vec3::new(from, 0.0, 0.0),
        Vec3::new(to, 0.0, 0.0),
        None,
    )
    .unwrap()
    .into()
}
fn proof(source: &Curve, uv: &Curve) -> Result<(), step_brep::BuildError> {
    let mut k = Kernel::default();
    k.fixes.query_statuses_pass_on = true;
    step_brep::check_planar_uv_projection(
        &k,
        source,
        &source.natural_interval(),
        uv,
        &plane(),
        1e-5,
    )
}
#[test]
fn full_line_has_complete_projection() {
    assert!(proof(&line(3, 0.0, 10.0), &line(2, 0.0, 10.0)).is_ok());
}
#[test]
fn reversed_full_line_has_complete_projection() {
    assert!(proof(&line(3, 0.0, 10.0), &line(2, 10.0, 0.0)).is_ok());
}
#[test]
fn truncated_lift_must_not_cover_a_longer_source_edge() {
    let result = proof(&line(3, 0.0, 10.0), &line(2, 0.0, 5.0));
    assert!(
        result.is_err(),
        "incomplete [0,5] lift accepted for original [0,10]: {result:?}"
    );
}
fn narrow_span(dim: u64, bump: f64) -> Curve {
    let times = [0.0, 0.4, 0.5, 0.50000001, 0.50000002, 1.0];
    let mut poles = times.map(|t| Vec3::new(10.0 * t, 0.0, 0.0));
    poles[3].y = bump;
    BSplineCurve::create_canonical(&CanonicalData {
        dim,
        degree: 1,
        control_points: &poles,
        form: BSplineCurveForm::Unspecified,
        knot_multiplicities: &[2, 1, 1, 1, 1, 2],
        knots: &times,
        knot_type: KnotType::Unspecified,
        weights: None,
        trim: None,
    })
    .unwrap()
    .into()
}
#[test]
fn every_narrow_span_needs_a_conservative_projection_proof() {
    let result = proof(&narrow_span(3, 0.0), &narrow_span(2, 0.25));
    assert!(
        result.is_err(),
        "narrow-span 0.25mm deviation escaped full-domain proof: {result:?}"
    );
}

// ---- 全域证书控制：只追加，red四断言原字节保留 ----------------------
use solidcore::curve::ellipse::{Ellipse, EllipseOptions};
use solidcore::geom::{extent1d::Extent1d, extent2d::Extent2d};
use solidcore::nurbs::{
    crv_basic::Curve as Nurb,
    math::{CPoint, NOW, NOZ},
};

fn raw_curve(dim: u64, poles: &[[f64; 3]], weights: &[f64], knots: &[f64], degree: usize) -> Curve {
    assert_eq!(poles.len(), weights.len());
    let points = poles
        .iter()
        .zip(weights)
        .map(|(p, &w)| {
            let scale = if w == NOW { 1.0 } else { w };
            CPoint::new(
                p[0] * scale,
                p[1] * scale,
                if dim == 2 { NOZ } else { p[2] * scale },
                w,
            )
        })
        .collect();
    let data = Nurb::new(points, degree, knots.to_vec()).unwrap();
    BSplineCurve::from_gw_nurb(dim, &data).into()
}
const CUBIC_POLES: [[f64; 3]; 4] = [
    [0.0, 0.0, 0.0],
    [2.0, 3.0, 0.0],
    [7.0, -1.0, 0.0],
    [10.0, 0.0, 0.0],
];
const CUBIC_KNOTS: [f64; 8] = [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0];

#[test]
fn complete_non_linear_and_positive_rational_nets_cover_all_spans() {
    for weights in [[NOW; 4], [1.0, 2.0, 0.5, 1.0]] {
        let source = raw_curve(3, &CUBIC_POLES, &weights, &CUBIC_KNOTS, 3);
        let uv = raw_curve(2, &CUBIC_POLES, &weights, &CUBIC_KNOTS, 3);
        assert!(
            proof(&source, &uv).is_ok(),
            "matching complete rational basis"
        );
    }
    assert!(proof(&narrow_span(3, 0.25), &narrow_span(2, 0.25)).is_ok());
}

#[test]
fn matching_endpoints_do_not_prove_an_intermediate_control_point() {
    let source = raw_curve(3, &CUBIC_POLES, &[1.0, 2.0, 0.5, 1.0], &CUBIC_KNOTS, 3);
    let mut poles = CUBIC_POLES;
    poles[1][1] += 0.25;
    let uv = raw_curve(2, &poles, &[1.0, 2.0, 0.5, 1.0], &CUBIC_KNOTS, 3);
    assert!(proof(&source, &uv).is_err());
}

#[test]
fn different_knots_degree_weights_and_partial_source_domain_are_unknown() {
    let source = raw_curve(3, &CUBIC_POLES, &[1.0, 2.0, 0.5, 1.0], &CUBIC_KNOTS, 3);
    let uv = raw_curve(2, &CUBIC_POLES, &[1.0, 2.0, 0.5, 1.0], &CUBIC_KNOTS, 3);
    let partial = Extent1d::from_min_max(0.0, 0.5);
    assert!(step_brep::check_planar_uv_projection(
        &Kernel::default(),
        &source,
        &partial,
        &uv,
        &plane(),
        1e-5
    )
    .is_err());
    let other_knots = [0.0, 0.0, 0.0, 0.0, 2.0, 2.0, 2.0, 2.0];
    assert!(proof(
        &source,
        &raw_curve(2, &CUBIC_POLES, &[1.0, 2.0, 0.5, 1.0], &other_knots, 3)
    )
    .is_err());
    assert!(proof(
        &source,
        &raw_curve(2, &CUBIC_POLES, &[1.0, 1.0, 0.5, 1.0], &CUBIC_KNOTS, 3)
    )
    .is_err());
    let quadratic = raw_curve(
        2,
        &CUBIC_POLES[..3],
        &[1.0, 2.0, 0.5],
        &[0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        2,
    );
    assert!(proof(&source, &quadratic).is_err());
}

#[test]
fn non_positive_mixed_or_non_finite_homogeneous_data_cannot_certify() {
    for weights in [
        [1.0, 0.0, 1.0, 1.0],
        [1.0, -1.0, 1.0, 1.0],
        [NOW, 1.0, 1.0, 1.0],
        [1.0, f64::NAN, 1.0, 1.0],
        [1.0, f64::INFINITY, 1.0, 1.0],
    ] {
        assert!(proof(
            &raw_curve(3, &CUBIC_POLES, &weights, &CUBIC_KNOTS, 3),
            &raw_curve(2, &CUBIC_POLES, &weights, &CUBIC_KNOTS, 3)
        )
        .is_err());
    }
    let mut poles = CUBIC_POLES;
    poles[1][0] = f64::NAN;
    assert!(proof(
        &raw_curve(3, &poles, &[NOW; 4], &CUBIC_KNOTS, 3),
        &raw_curve(2, &poles, &[NOW; 4], &CUBIC_KNOTS, 3)
    )
    .is_err());
}

fn conic(dim: u64, rx: f64, ry: f64, start: f64, end: f64, inside_out: bool) -> Curve {
    let k = Kernel::default();
    let options = EllipseOptions {
        dim,
        inside_out,
        ..Default::default()
    };
    let interval = Extent1d::from_min_max(start, end);
    if rx == ry {
        Ellipse::new_circle(
            &k,
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            interval,
            rx,
            &options,
        )
        .unwrap()
        .into()
    } else {
        Ellipse::new(
            &k,
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            interval,
            rx,
            ry,
            &options,
        )
        .unwrap()
        .into()
    }
}

#[test]
fn actual_circle_and_ellipse_existing_nurbs_and_bounded_arcs_are_certified() {
    for (rx, ry, start, end, inside_out) in [
        (2.0, 2.0, 0.0, 360.0, false),
        (3.0, 1.0, 0.0, 360.0, false),
        (2.0, 2.0, 30.0, 210.0, false),
        (3.0, 1.0, 30.0, 210.0, true),
    ] {
        let source = conic(3, rx, ry, start, end, inside_out);
        let uv_conic = conic(2, rx, ry, start, end, inside_out);
        assert!(matches!(
            source.kind(),
            solidcore::curve::curve::CurveKind::Ellipse(_)
        ));
        assert!(
            proof(&source, &uv_conic).is_ok(),
            "real conic base, not BSpline-only branch"
        );
        let p = Plane::create_canonical(&Axis2Placement::IDENTITY).unwrap();
        let mut k = Kernel::default();
        k.fixes.query_statuses_pass_on = true;
        let mut dropped = p
            .drop_curve(
                &k,
                &p.step_uv_domain(),
                &source,
                &source.natural_interval(),
                1e-5,
                false,
                true,
                None,
            )
            .unwrap();
        assert_eq!(dropped.uv_curves.len(), 1);
        let uv = dropped.uv_curves.remove(0);
        assert!(
            proof(&source, &uv).is_ok(),
            "actual Plane.drop_curve output for bounded conic"
        );
    }
}

#[test]
fn a_whole_conic_does_not_certify_a_partial_edge_or_shorter_uv_arc() {
    let source = conic(3, 2.0, 2.0, 0.0, 360.0, false);
    let uv = conic(2, 2.0, 2.0, 0.0, 360.0, false);
    let natural = source.natural_interval();
    let partial = Extent1d::from_min_max(natural.min(), (natural.min() + natural.max()) * 0.5);
    assert!(step_brep::check_planar_uv_projection(
        &Kernel::default(),
        &source,
        &partial,
        &uv,
        &plane(),
        1e-5
    )
    .is_err());
    assert!(proof(&source, &conic(2, 2.0, 2.0, 0.0, 180.0, false)).is_err());
}

#[test]
fn plane_clamp_displacement_is_included_and_cannot_hide_missing_coverage() {
    let mut p = Plane::create_canonical(&Axis2Placement::IDENTITY).unwrap();
    p.adjust_step_uv_domain(&Extent2d::from_bounds(0.0, -1.0, 5.0, 1.0))
        .unwrap();
    let source = line(3, 0.0, 10.0);
    let uv = line(2, 0.0, 10.0);
    assert!(step_brep::check_planar_uv_projection(
        &Kernel::default(),
        &source,
        &source.natural_interval(),
        &uv,
        &p.clone().into(),
        1e-5
    )
    .is_err());
    p.set_out_of_bounds_enabled(true);
    assert!(step_brep::check_planar_uv_projection(
        &Kernel::default(),
        &source,
        &source.natural_interval(),
        &uv,
        &p.into(),
        1e-5
    )
    .is_ok());
}

#[test]
fn source_derived_roundoff_is_bounded_without_a_relative_gap_allowance() {
    let source = raw_curve(3, &CUBIC_POLES, &[NOW; 4], &CUBIC_KNOTS, 3);
    let mut close = CUBIC_POLES;
    close[1][2] = 1e-8;
    let mut far = CUBIC_POLES;
    far[1][2] = 0.001;
    let uv = raw_curve(2, &CUBIC_POLES, &[NOW; 4], &CUBIC_KNOTS, 3);
    assert!(proof(&raw_curve(3, &close, &[NOW; 4], &CUBIC_KNOTS, 3), &uv).is_ok());
    assert!(proof(&raw_curve(3, &far, &[NOW; 4], &CUBIC_KNOTS, 3), &uv).is_err());
    assert!(proof(&source, &uv).is_ok());
}

#[test]
fn translated_rotated_scaled_plane_uses_world_affine_control_net() {
    let plane = Plane::new(
        Vec3::new(100.0, 200.0, 300.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        solidcore::Vec2::new(2.0, 3.0),
        &Extent2d::from_bounds(-20.0, -20.0, 20.0, 20.0),
    );
    let world = CUBIC_POLES.map(|p| [100.0, 200.0 + 2.0 * p[0], 300.0 + 3.0 * p[1]]);
    let source = raw_curve(3, &world, &[1.0, 2.0, 0.5, 1.0], &CUBIC_KNOTS, 3);
    let uv = raw_curve(2, &CUBIC_POLES, &[1.0, 2.0, 0.5, 1.0], &CUBIC_KNOTS, 3);
    assert!(step_brep::check_planar_uv_projection(
        &Kernel::default(),
        &source,
        &source.natural_interval(),
        &uv,
        &plane.into(),
        1e-5
    )
    .is_ok());
}

#[test]
fn unsupported_wrapper_and_non_clamped_basis_are_unknown() {
    let source = Curve::from(
        solidcore::curve::crv_on_surf::CrvOnSurf::new(
            &Kernel::default(),
            line(2, 0.0, 10.0),
            plane(),
            None,
            0,
        )
        .unwrap(),
    );
    assert!(proof(&source, &line(2, 0.0, 10.0)).is_err());
    let knots = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    assert!(proof(
        &raw_curve(3, &CUBIC_POLES, &[NOW; 4], &knots, 2),
        &raw_curve(2, &CUBIC_POLES, &[NOW; 4], &knots, 2)
    )
    .is_err());
}
