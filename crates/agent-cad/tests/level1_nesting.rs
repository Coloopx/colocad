// Exercise the native predicate on corrupted planar trims, not a feature-level rejection.
use solidcore::curve::curve::p2::CurveKind;
use solidcore::geom::axis2_placement::Axis2Placement;
use solidcore::topology::{
    edgeuse::p1,
    face::{p2::loops, p4::assert_test_face37},
};
use solidcore::{Kernel, Vec3};
#[test]
fn planar_broad_phase_rejects_nested_holes_and_wrong_outer_loop() {
    use solidcore::boolean::api_brep::boolean_difference;
    use solidcore::primitive::api_primitives::{create_box, create_cylinder};
    let mut k = Kernel::default();
    let mut b = create_box(&mut k, Vec3::ZERO, 20., 20., 6.).unwrap();
    for (x, y) in [(4., 4.), (10., 4.), (4., 10.), (10., 10.)] {
        let bore = create_cylinder(&mut k, Vec3::new(x, y, -1.), 1., 8.).unwrap();
        b = boolean_difference(&mut k, b, bore).unwrap();
    }
    let original = solidcore::topology::brep::validation::check_brep(
        &mut k,
        &mut b,
        solidcore::topology::brep::validation::CheckLevel::Level1,
    )
    .unwrap();
    assert!(original.is_valid(), "{:?}", original.lines());
    let face = b
        .faces()
        .ids()
        .find(|&f| loops(&b, f, false, None).unwrap().len() == 5)
        .unwrap();
    let ls = loops(&b, face, false, None).unwrap();
    let edge = |l| {
        let es = p1::loopuse_edgeuses(&b, b.loop_(l).unwrap().loopuse.unwrap()).unwrap();
        assert_eq!(es.len(), 1);
        es[0]
    };
    let (a, z) = (edge(ls[1]), edge(ls[2]));
    let (original, _) = p1::uv_trim_curve(&b, a).unwrap();
    let mut tiny = original.unwrap().clone();
    let CurveKind::BSpline(sp) = tiny.kind() else {
        panic!("fixture must exercise the hull path")
    };
    let (p, _) = sp.control_polygon().unwrap();
    let center = p.iter().copied().fold(Vec3::ZERO, |a, b| a + b) / (p.len() as f64);
    let shift = |v| {
        let mut t = Axis2Placement::IDENTITY;
        t.set_origin(v);
        t
    };
    tiny.transform(&k, &shift(-center), None).unwrap();
    tiny.transform(&k, &Axis2Placement::IDENTITY, Some(Vec3::new(0.1, 0.1, 1.)))
        .unwrap();
    tiny.transform(&k, &shift(center), None).unwrap();
    let holder = p1::uv_trim_curve(&b, z).unwrap().1.unwrap();
    assert!(assert_test_face37(&k, &mut b, face).unwrap());
    let saved = b.edgeuse_mut(holder).unwrap().uv_trim_curve.replace(tiny);
    assert!(
        !assert_test_face37(&k, &mut b, face).unwrap(),
        "a nested trim must still fail"
    );
    b.edgeuse_mut(holder).unwrap().uv_trim_curve = saved;
    let fu = b.face(face).unwrap().faceuse.unwrap();
    b.faceuse_mut(fu).unwrap().loopuses.swap(0, 1);
    assert!(
        !assert_test_face37(&k, &mut b, face).unwrap(),
        "the designated outer loop must contain every sample"
    );
}
