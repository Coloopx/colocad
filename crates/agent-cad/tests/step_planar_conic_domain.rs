//! [INPUT]: step-import中性Circle/Ellipse/Plane与原生BRep检查器。
//! [OUTPUT]: G2连续极值、自有反例及严格几何拒绝回归。
//! [POS]: 导入参数域的独立测试，不含厂家CAD或G1算法。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use step_import::model::{Body, BodyKind, Bound, Curve, Edge, Face, Frame, Loop, OrientedEdge, Shell, Surface, Vertex};

fn frame() -> Frame {
    Frame { origin: [0.; 3], x: [1., 0., 0.], y: [0., 1., 0.], z: [0., 0., 1.] }
}
fn arc(curve: Curve, start: f64, end: f64, plane: Frame) -> Body {
    Body {
        step_id: 1, name: None, kind: BodyKind::ShellModel, unit_mm: 1., uncertainty_mm: Some(1e-5),
        shells: vec![Shell { step_id: 2, closed: false, orientation: true, faces: vec![0] }],
        faces: vec![Face { step_id: 3, surface: Surface::Plane { frame: plane }, same_sense: true,
            loops: vec![Loop { outer: true, orientation: true, bound: Bound::Edges(vec![OrientedEdge { edge: 0, forward: true }]) }] }],
        edges: vec![Edge { step_id: 4, start: 0, end: 1, curve, same_sense: end > start, t_start: start, t_end: end }],
        vertices: vec![],
    }
}
fn circle(start: f64, end: f64) -> Body {
    arc(Curve::Circle { frame: frame(), radius: 2. }, start, end, frame())
}
fn domain(body: &Body) -> [f64; 4] {
    step_import::script::step_uv_domain(body, &body.faces[0]).unwrap()
}
fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() <= 1e-12, "actual={actual} expected={expected}");
}

#[test]
fn circle_contains_the_continuous_extremum_between_sample_parameters() {
    let b = circle(17., 139.);
    near(domain(&b)[3], 2.);
    let sampled = (0..=32).map(|i| step_import::script::edge_point(&b.edges[0], f64::from(i) / 32.).unwrap()[1]).fold(f64::NEG_INFINITY, f64::max);
    assert!(sampled < 2. - 1e-5, "fixture must expose the old sampled bound");
}
#[test]
fn reversed_arc_and_reversed_loop_keep_the_same_domain() {
    let mut b = circle(139., 17.);
    let expected = domain(&circle(17., 139.));
    assert_eq!(domain(&b), expected);
    b.faces[0].loops[0].orientation = false;
    if let Bound::Edges(es) = &mut b.faces[0].loops[0].bound { es[0].forward = false; }
    assert_eq!(domain(&b), expected);
}
#[test]
fn crossing_360_and_full_turn_include_only_their_interval_extrema() {
    near(domain(&circle(317., 401.))[2], 2.);
    near(domain(&circle(401., 317.))[2], 2.);
    for (a, b) in [(17., 377.), (377., 17.), (-343., 17.)] {
        let d = domain(&circle(a, b));
        for (actual, expected) in d.into_iter().zip([-2., -2., 2., 2.]) { near(actual, expected); }
    }
    let short = domain(&circle(17., 41.));
    assert!(short[0] > 1. && short[2] < 2., "exclude extrema outside the arc");
}
#[test]
fn tilted_plane_and_ellipse_use_the_projected_axes() {
    // 相互倾斜的正交坐标，U与V均不是圆锥曲线的原始X/Y轴。
    let q = 0.5_f64.sqrt();
    let plane = Frame { origin: [3., -4., 5.], x: [q, q, 0.], y: [-0.5, 0.5, q], z: [0.5, -0.5, q] };
    let b = arc(Curve::Ellipse { frame: frame(), semi1: 3., semi2: 1. }, 17., 377., plane);
    let d = domain(&b);
    near(d[0], q - 5_f64.sqrt()); near(d[2], q + 5_f64.sqrt());
    let v_center = 3.5 - 5. * q;
    near(d[1], v_center - 2.5_f64.sqrt()); near(d[3], v_center + 2.5_f64.sqrt());
}
#[test]
fn a_constant_projection_is_finite_without_an_invented_span() {
    let plane = Frame { x: [0., 0., 1.], y: [1., 0., 0.], z: [0., 1., 0.], ..frame() };
    let d = domain(&arc(Curve::Circle { frame: frame(), radius: 2. }, 17., 377., plane));
    assert!(d.into_iter().all(f64::is_finite)); near(d[0], 0.); near(d[2], 0.);
    near(d[1], -2.); near(d[3], 2.);
}
#[test]
fn nonplanar_surface_keeps_its_existing_domain_path() {
    let mut b = circle(17., 139.);
    b.faces[0].surface = Surface::Cylinder { frame: frame(), radius: 2. };
    let d = domain(&b);
    near(d[0], 17.); near(d[2], 139.); near(d[1], 0.); near(d[3], 0.);
}

fn circular_segment() -> Body {
    let mut b = circle(17., 139.);
    let a = step_import::script::edge_point(&b.edges[0], 0.).unwrap();
    let z = step_import::script::edge_point(&b.edges[0], 1.).unwrap();
    b.vertices = vec![Vertex { step_id: 5, point: a }, Vertex { step_id: 6, point: z }];
    let vector = [a[0] - z[0], a[1] - z[1], 0.];
    let length = (vector[0] * vector[0] + vector[1] * vector[1]).sqrt();
    b.edges.push(Edge { step_id: 7, start: 1, end: 0, curve: Curve::Line { origin: z, dir: vector.map(|v| v / length), magnitude: length }, same_sense: true, t_start: 0., t_end: 1. });
    if let Bound::Edges(es) = &mut b.faces[0].loops[0].bound { es.push(OrientedEdge { edge: 1, forward: true }); }
    b
}
#[test]
fn exact_segment_passes_but_shifted_uv_keeps_the_gap_rejection() {
    use solidcore::{Kernel, Vec3};
    use solidcore::topology::{brep::validation::{check_brep, CheckLevel}, edgeuse::p1};
    use solidcore::geom::axis2_placement::Axis2Placement;
    let mut b = step_brep::build_from_topology(&circular_segment(), Default::default()).unwrap().brep;
    let mut k = Kernel::default();
    let checked = check_brep(&mut k, &mut b, CheckLevel::Level1).unwrap();
    assert!(checked.is_valid(), "{:?}", checked.lines());
    let eu = b.edgeuses().ids().find(|&eu| b.edgeuse(eu).unwrap().uv_trim_curve.is_some()).unwrap();
    let holder = p1::uv_trim_curve(&b, eu).unwrap().1.unwrap();
    b.edgeuse_mut(holder).unwrap().uv_trim_curve.as_mut().unwrap().transform(&k, &Axis2Placement::new(Vec3::new(0.01, 0., 0.), Vec3::X, Vec3::Y), None).unwrap();
    let rejected = check_brep(&mut k, &mut b, CheckLevel::Level1).unwrap();
    assert!(!rejected.is_valid());
    assert!(rejected.lines().iter().any(|line| line.contains("SmTrimmingTools::Test #0")), "{:?}", rejected.lines());
}
