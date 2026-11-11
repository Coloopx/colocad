// Compare the original solver and the scoped assembly optimization on the same inputs.
use solidcore::geom::assert_array::{AssertArray, ObjectKey};
use solidcore::nurbs::b_spline_curve::BSplineCurve;
use solidcore::nurbs::math::{CPoint, NOW};
use solidcore::nurbs::nurbs_crv::allocate_nurb_curve;
use solidcore::Kernel;

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
