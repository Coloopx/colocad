//! SDD 0004 E03: kernel feasibility assertions, not acceptance of a public query API.
use agent_cad::query::face::{classify_uv, TrimStatus};
use agent_cad::query::snapshot::DefinitionSnapshot;
use solidcore::boolean::api_brep::boolean_difference;
use solidcore::mass::integrator::properties::face_area;
use solidcore::primitive::api_primitives::{create_box, create_cylinder};
use solidcore::query::api_queries::{
    face_bounding_box, face_get_boundary_loops, surface_closest_point,
};
use solidcore::{Brep, Kernel, Vec3};

fn plate(k: &mut Kernel) -> Brep {
    let mut b = create_box(k, Vec3::new(-50., -30., 0.), 100., 60., 12.).unwrap();
    // Through Ø6, blind Ø10 depth 8, and Ø6 through + Ø10 counterbore depth 3.
    for (x, y, z, radius, height) in [
        (-30., -20., -1., 3., 14.),
        (30., -20., 4., 5., 9.),
        (0., 15., -1., 3., 14.),
        (0., 15., 9., 5., 4.),
    ] {
        let tool = create_cylinder(k, Vec3::new(x, y, z), radius, height).unwrap();
        b = boolean_difference(k, b, tool).unwrap();
    }
    b
}

fn check_trim(k: &Kernel, b: &mut Brep) {
    let tops = b
        .face_list()
        .iter()
        .copied()
        .filter(|f| {
            let box_ = face_bounding_box(k, b, *f, true).unwrap();
            (box_.min().z - 12.).abs() < 1e-6 && (box_.max().z - 12.).abs() < 1e-6
        })
        .collect::<Vec<_>>();
    assert_eq!(tops.len(), 1);
    let f = tops[0];
    let loops = face_get_boundary_loops(b, f).unwrap();
    assert_eq!(
        loops.edge_counts.len(),
        4,
        "one outer and three inner loops"
    );
    assert_eq!(loops.edge_counts.iter().sum::<usize>(), loops.edges.len());
    assert_eq!(loops.edges.len(), loops.orientations.len());
    let expected_area = 6000. - 59. * std::f64::consts::PI;
    assert!((face_area(k, b, f, 1e-8).unwrap() - expected_area).abs() < 1e-4);
    let surface = b.face(f).unwrap().surface.as_ref().unwrap().clone();
    assert!(matches!(
        surface.kind(),
        solidcore::surface::surface::p2::SurfaceKind::Plane(_)
    ));
    for (point, expected) in [
        (Vec3::new(0., 0., 12.), TrimStatus::Inside),
        (Vec3::new(-27., -20., 12.), TrimStatus::Boundary),
        (Vec3::new(-30., -20., 12.), TrimStatus::OutsideTrim),
        (Vec3::new(30., -20., 12.), TrimStatus::OutsideTrim),
        (Vec3::new(0., 15., 12.), TrimStatus::OutsideTrim),
    ] {
        let result = surface_closest_point(k, &surface, point).unwrap();
        assert!(
            result.closest.distance.abs() < 1e-6,
            "interior and hole points lie within the support surface domain"
        );
        assert!(!result.uvs.is_empty());
        for uv in result.uvs {
            assert_eq!(classify_uv(k, b, f, uv).unwrap(), expected);
        }
    }
    // SurfaceClosestPoint is bounded by the surface's natural domain. For an
    // analytic infinite-plane projection, derive UV explicitly; do not confuse
    // the clamped closest point with the requested point outside the plate.
    if let solidcore::surface::surface::p2::SurfaceKind::Plane(plane) = surface.kind() {
        let mut plane = plane.copy();
        plane.set_out_of_bounds_enabled(true);
        let point = Vec3::new(60., 0., 12.);
        let uv = plane.project_point_to_uv_domain(point);
        assert!((plane.evaluate_point(uv) - point).length() < 1e-6);
        assert_eq!(classify_uv(k, b, f, uv).unwrap(), TrimStatus::OutsideTrim);
    }
}

#[test]
fn invalid_uv_is_an_error_not_outside_trim() {
    let mut k = Kernel::default();
    let mut b = plate(&mut k);
    let face = b.face_list()[0];
    for u in [f64::NAN, f64::INFINITY] {
        assert_eq!(
            classify_uv(&k, &mut b, face, solidcore::Point2::new(u, 0.))
                .unwrap_err()
                .code,
            "QUERY_ARGUMENT"
        );
    }
}

#[test]
fn planar_area_and_classification_respect_through_blind_and_counterbore_trims() {
    let mut k = Kernel::default();
    let mut b = plate(&mut k);
    check_trim(&k, &mut b);
}

#[test]
fn exact_snapshot_preserves_trim_classification_in_a_fresh_kernel() {
    let mut k = Kernel::default();
    let b = plate(&mut k);
    let snapshot = DefinitionSnapshot::capture(&k, &b).unwrap();
    let mut fresh = Kernel::default();
    let mut loaded = snapshot.load(&mut fresh, &snapshot.digest()).unwrap();
    check_trim(&fresh, &mut loaded.body);
}

#[test]
fn analytic_cylinder_seam_retains_repeated_oriented_edge_uses() {
    let mut k = Kernel::default();
    let b = create_cylinder(&mut k, Vec3::ZERO, 3., 10.).unwrap();
    let check = |b: &Brep| {
        let cylinders = b.face_list().iter().copied().filter(|f| {
            matches!(b.face(*f).unwrap().surface.as_ref().unwrap().kind(),
                solidcore::surface::surface::p2::SurfaceKind::Cone(c) if c.is_cylinder() && (c.base_radius()-3.).abs()<1e-8)
        }).collect::<Vec<_>>();
        assert_eq!(cylinders.len(), 1);
        let loops = face_get_boundary_loops(b, cylinders[0]).unwrap();
        let mut repeated = 0;
        for i in 0..loops.edges.len() {
            for j in i + 1..loops.edges.len() {
                if loops.edges[i] == loops.edges[j] {
                    repeated += 1;
                    assert_ne!(loops.orientations[i], loops.orientations[j]);
                }
            }
        }
        assert_eq!(
            repeated, 1,
            "the periodic seam is one edge used twice, not two holes"
        );
    };
    check(&b);
    let snapshot = DefinitionSnapshot::capture(&k, &b).unwrap();
    let mut fresh = Kernel::default();
    check(&snapshot.load(&mut fresh, &snapshot.digest()).unwrap().body);
}
