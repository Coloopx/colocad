//! [INPUT]: SolidCore保守二次曲线证明、曲面毫米度量与原通用求解器。
//! [OUTPUT]: 未映射/映射曲线的证明复用及相同负例报告回归。
//! [POS]: 原生校验性能与拒绝语义的对照，不更改阈值或几何。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use solidcore::geom::assert_array::{AssertArray, ObjectKey};
use solidcore::nurbs::b_spline_curve::BSplineCurve;
use solidcore::nurbs::b_spline_surface::BSplineSurface;
use solidcore::nurbs::math::{CPoint, NOW};
use solidcore::nurbs::nurbs_crv::allocate_nurb_curve;
use solidcore::surface::surface::Surface;
use solidcore::Kernel;
use solidcore::Vec3;

fn bezier(coords: &[[f64; 3]]) -> BSplineCurve {
    let degree = coords.len() - 1;
    let mut nurb = allocate_nurb_curve(degree, degree, 2 * degree + 1).unwrap();
    for (pole, p) in nurb.cpts_mut().iter_mut().zip(coords) {
        *pole = CPoint::new(p[0], p[1], p[2], NOW);
    }
    for (i, knot) in nurb.knots_mut().iter_mut().enumerate() {
        *knot = if i <= degree { 0. } else { 1. };
    }
    BSplineCurve::from_nurb(nurb, 3, false)
}
fn check(k: &Kernel, c: &BSplineCurve) -> (bool, String) {
    let mut reports = AssertArray::new();
    let valid = c
        .assert_valid(k, Some(&mut reports), false, ObjectKey(17))
        .unwrap();
    (valid, format!("{:?}", reports.reports()))
}
#[test]
fn certificate_cache_and_stricter_policy_agree_with_reference() {
    let mut fast = Kernel::default();
    fast.caches.enable_validation_property_memo();
    let mut reference = Kernel::default();
    let w = 0.5_f64.sqrt();
    let mut arc = allocate_nurb_curve(2, 2, 5).unwrap();
    arc.knots_mut().copy_from_slice(&[0., 0., 0., 1., 1., 1.]);
    arc.cpts_mut().copy_from_slice(&[
        CPoint::new(1., 0., 0., 1.),
        CPoint::new(w, w, 0., w),
        CPoint::new(0., 1., 0., 1.),
    ]);
    let circle = BSplineCurve::from_nurb(arc, 3, false);
    let expected = check(&reference, &circle);
    assert!(expected.0);
    assert_eq!(check(&fast, &circle), expected);
    assert_eq!(fast.caches.validation_property_counts().2, 1);
    let linear = bezier(&[[0., 0., 0.], [3., 5., 7.]]);
    assert_eq!(check(&fast, &linear), check(&reference, &linear));
    let (_, misses, certified) = fast.caches.validation_property_counts();
    assert_eq!(check(&fast, &linear), check(&reference, &linear));
    assert!(
        fast.caches.validation_property_counts().0 > 0,
        "at least one pure solve was reused"
    );
    let changed = bezier(&[[0., 0., 0.], [3., 5., f64::from_bits(7f64.to_bits() + 1)]]);
    assert_eq!(check(&fast, &changed), check(&reference, &changed));
    assert!(fast.caches.validation_property_counts().1 > misses);
    let curved = bezier(&[[-0.0005, 0.25, 0.], [0., -0.25, 0.], [0.0005, 0.25, 0.]]);
    let first = check(&reference, &curved);
    assert_eq!(check(&fast, &curved), first);
    fast.fixes.bspline_check_min_radius = true;
    reference.fixes.bspline_check_min_radius = true;
    let rejected = check(&reference, &curved);
    assert!(!rejected.0);
    assert_eq!(check(&fast, &curved), rejected);
    assert_eq!(fast.caches.validation_property_counts().2, certified);
    // Positive weights alone do not certify a quadratic. This narrow bend
    // reaches the stricter radius check, with the same native failure report.
    let mut sharp = allocate_nurb_curve(2, 2, 5).unwrap();
    sharp.knots_mut().copy_from_slice(&[0., 0., 0., 1., 1., 1.]);
    sharp.cpts_mut().copy_from_slice(&[
        CPoint::new(-0.0005, 0.25, 0., 1.),
        CPoint::new(0., -0.25 * w, 0., w),
        CPoint::new(0.0005, 0.25, 0., 1.),
    ]);
    let sharp = BSplineCurve::from_nurb(sharp, 3, false);
    let failed = check(&reference, &sharp);
    assert!(!failed.0);
    assert_eq!(check(&fast, &sharp), failed);
    assert_eq!(fast.caches.validation_property_counts().2, certified);
    let folded = bezier(&[[0., 0., 0.], [0., 0., 2.], [0., 0., -2.], [0., 0., 1.]]);
    assert_eq!(check(&fast, &folded), check(&reference, &folded));
    assert_eq!(fast.caches.validation_property_counts().2, certified);
}

fn uv_arc(scale: f64) -> BSplineCurve {
    let w = 0.5_f64.sqrt();
    let mut n = allocate_nurb_curve(2, 2, 5).unwrap();
    n.knots_mut().copy_from_slice(&[0., 0., 0., 1., 1., 1.]);
    n.cpts_mut().copy_from_slice(&[
        CPoint::new(scale, 0., 0., 1.),
        CPoint::new(scale * w, scale * w, 0., w),
        CPoint::new(0., scale, 0., 1.),
    ]);
    BSplineCurve::from_nurb(n, 2, false)
}

fn metric_plane(scale: f64, skew: f64) -> Surface {
    Surface::from(
        BSplineSurface::create_bilinear_surface(
            Vec3::ZERO,
            Vec3::new(1. / scale, 0., 0.),
            Vec3::new(skew / scale, 2. / scale, 0.),
            Vec3::new((1. + skew) / scale, 2. / scale, 0.),
        )
        .unwrap(),
    )
}

fn check_on(k: &Kernel, curve: &BSplineCurve, surface: &Surface) -> (bool, String) {
    let mut reports = AssertArray::new();
    let valid = curve
        .assert_valid_on(k, Some(&mut reports), false, ObjectKey(17), Some(surface))
        .unwrap();
    (valid, format!("{:?}", reports.reports()))
}

#[test]
fn mapped_quadratic_uses_its_own_millimetre_certificate() {
    for (scale, skew) in [(1., 0.), (1e-3, 0.), (1e-3, 0.4), (10., -0.3), (1e-6, 0.)] {
        let curve = uv_arc(scale);
        let surface = metric_plane(scale, skew);
        let mut fast = Kernel::default();
        fast.fixes.uv_trim_checks_on_surface = true;
        fast.caches.enable_validation_property_memo();
        let mut reference = Kernel::default();
        reference.fixes.uv_trim_checks_on_surface = true;
        let expected = check_on(&reference, &curve, &surface);
        assert!(expected.0, "scale={scale}, skew={skew}: {expected:?}");
        assert_eq!(check_on(&fast, &curve, &surface), expected);
        // 原来已被判为退化的微小UV弧不会进入曲率求解，保持该路径的原语义。
        let expected_certificates = usize::from(scale >= 1e-3);
        assert_eq!(fast.caches.validation_property_counts().2, expected_certificates,
            "mapped regular curve should avoid the generic radius solve: scale={scale}, skew={skew}");
    }
}

#[test]
fn mapped_certificate_retains_stricter_solver_and_radius_rejections() {
    let surface = metric_plane(1., 0.4);
    let curve = uv_arc(1.);
    for strict_status in [false, true] {
        let mut fast = Kernel::default();
        fast.fixes.uv_trim_checks_on_surface = true;
        fast.fixes.query_statuses_pass_on = strict_status;
        fast.fixes.curve_check_zero_speed_tangent_end = !strict_status;
        fast.caches.enable_validation_property_memo();
        let mut reference = Kernel::default();
        reference.fixes = fast.fixes;
        assert_eq!(
            check_on(&fast, &curve, &surface),
            check_on(&reference, &curve, &surface)
        );
        assert_eq!(fast.caches.validation_property_counts().2, 0);
    }
    let w = 0.5_f64.sqrt();
    let mut n = allocate_nurb_curve(2, 2, 5).unwrap();
    n.knots_mut().copy_from_slice(&[0., 0., 0., 1., 1., 1.]);
    n.cpts_mut().copy_from_slice(&[
        CPoint::new(-0.0005, 0.25, 0., 1.),
        CPoint::new(0., -0.25 * w, 0., w),
        CPoint::new(0.0005, 0.25, 0., 1.),
    ]);
    let sharp = BSplineCurve::from_nurb(n, 2, false);
    let mut fast = Kernel::default();
    fast.fixes.uv_trim_checks_on_surface = true;
    fast.fixes.bspline_check_min_radius = true;
    fast.caches.enable_validation_property_memo();
    let mut reference = Kernel::default();
    reference.fixes = fast.fixes;
    let rejected = check_on(&reference, &sharp, &surface);
    assert!(!rejected.0, "real sharp radius must remain rejected");
    assert_eq!(check_on(&fast, &sharp, &surface), rejected);
    assert_eq!(fast.caches.validation_property_counts().2, 0);
}

#[test]
fn regular_uv_curve_cannot_certify_a_physically_sharp_mapped_ellipse() {
    let curve = uv_arc(1.);
    let safe = metric_plane(1., 0.);
    let sharp = Surface::from(
        BSplineSurface::create_bilinear_surface(
            Vec3::ZERO,
            Vec3::new(1., 0., 0.),
            Vec3::new(0., 1e-3, 0.),
            Vec3::new(1., 1e-3, 0.),
        )
        .unwrap(),
    );
    let mut fast = Kernel::default();
    fast.fixes.uv_trim_checks_on_surface = true;
    fast.fixes.bspline_check_min_radius = true;
    fast.caches.enable_validation_property_memo();
    let mut reference = Kernel::default();
    reference.fixes = fast.fixes;
    // 同一UV曲线连续换L：安全、危险、再安全，不能复用前一个曲面的证书。
    for (surface, valid) in [(&safe, true), (&sharp, false), (&safe, true)] {
        let mut reports = AssertArray::new();
        let expected = curve
            .assert_valid_on(
                &reference,
                Some(&mut reports),
                false,
                ObjectKey(17),
                Some(surface),
            )
            .unwrap();
        assert_eq!(expected, valid);
        let before = fast.caches.validation_property_counts().2;
        assert_eq!(
            check_on(&fast, &curve, surface),
            (expected, format!("{:?}", reports.reports()))
        );
        if !valid {
            assert!(reports.reports().iter().any(|r| r.test_index == 13));
            assert_eq!(
                fast.caches.validation_property_counts().2,
                before,
                "raw UV circle must not certify its flattened physical ellipse"
            );
        }
    }
}

#[test]
fn mapped_certificate_never_suppresses_cancellation_or_knot_kinks() {
    use solidcore::{kernel::CancelToken, SolidError};
    let curve = uv_arc(1.);
    let surface = metric_plane(1., 0.);
    let mut k = Kernel::default();
    k.fixes.uv_trim_checks_on_surface = true;
    k.caches.enable_validation_property_memo();
    let token = CancelToken::new();
    k.set_cancel_token(token.clone());
    assert!(check_on(&k, &curve, &surface).0);
    assert_eq!(k.caches.validation_property_counts().2, 0);
    token.cancel();
    let cancelled = curve.assert_valid_on(&k, None, false, ObjectKey(17), Some(&surface));
    assert!(
        matches!(cancelled, Err(SolidError::Cancelled { .. })),
        "{cancelled:?}"
    );

    use solidcore::nurbs::b_spline_curve::p1::{BSplineCurveForm, CanonicalData, KnotType};
    let points = [
        Vec3::ZERO,
        Vec3::new(1., 0., 0.),
        Vec3::new(1., 1., 0.),
        Vec3::new(2., 1., 0.),
        Vec3::new(2., 2., 0.),
    ];
    let w = 0.5_f64.sqrt();
    let kink = BSplineCurve::create_canonical(&CanonicalData {
        dim: 2,
        degree: 2,
        control_points: &points,
        form: BSplineCurveForm::Unspecified,
        knot_multiplicities: &[3, 2, 3],
        knots: &[0., 0.5, 1.],
        knot_type: KnotType::Unspecified,
        weights: Some(&[1., w, 1., w, 1.]),
        trim: None,
    })
    .unwrap();
    let mut fast = Kernel::default();
    fast.fixes.uv_trim_checks_on_surface = true;
    fast.caches.enable_validation_property_memo();
    let mut reference = Kernel::default();
    reference.fixes = fast.fixes;
    let mut reports = AssertArray::new();
    let valid = kink
        .assert_valid_on(
            &reference,
            Some(&mut reports),
            false,
            ObjectKey(17),
            Some(&surface),
        )
        .unwrap();
    assert!(!valid);
    assert!(reports.reports().iter().any(|r| r.test_index == 7));
    assert_eq!(
        check_on(&fast, &kink, &surface),
        (valid, format!("{:?}", reports.reports()))
    );
}
