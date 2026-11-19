//! [INPUT]: SolidCore精确圆孔、同几何UV重参数化与公开Colocad build入口。
//! [OUTPUT]: UV/mm误判复现、单体/装配/STEP回读及真实坏几何拒绝回归。
//! [POS]: 原生校验单位一致性；不放宽任何几何容差。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use agent_cad::{build, Model};
use serde_json::json;
use sha2::{Digest, Sha256};
use solidcore::boolean::api_brep::boolean_difference;
use solidcore::geom::{axis2_placement::Axis2Placement, extent2d::Extent2d};
use solidcore::nurbs::b_spline_surface::BSplineSurface;
use solidcore::primitive::api_primitives::{create_box, create_cylinder};
use solidcore::surface::surface::Surface;
use solidcore::topology::{edgeuse::p1, face::p2::loops};
use solidcore::{Brep, Kernel, Vec3};

// ---- 同一物理平面映射到[0,1e-5]²，三维边曲线/顶点/容差保持原值 ----
fn plate() -> Brep {
    let mut k = Kernel::default();
    let stock = create_box(&mut k, Vec3::ZERO, 20., 20., 6.).unwrap();
    let tool = create_cylinder(&mut k, Vec3::new(10., 10., -1.), 1., 8.).unwrap();
    let mut b = boolean_difference(&mut k, stock, tool).unwrap();
    let faces = b.faces().ids().collect::<Vec<_>>();
    let mut changed = 0;
    for f in faces {
        let ls = loops(&b, f, false, None).unwrap();
        if ls.len() != 2 {
            continue;
        }
        let old = b.face(f).unwrap().surface.clone().unwrap();
        if !old.is_planar(&k, 1e-10).unwrap().is_planar {
            continue;
        }
        let domain = b.face(f).unwrap().uv_domain;
        let points = [(0., 0.), (1., 0.), (0., 1.), (1., 1.)]
            .map(|(u, v)| old.evaluate_point(&k, domain.evaluate(u, v)).unwrap());
        let mut next = Surface::from(
            BSplineSurface::create_bilinear_surface(points[0], points[1], points[2], points[3])
                .unwrap(),
        );
        next.reparameterize(&k, &Extent2d::from_bounds(0., 0., 1e-5, 1e-5))
            .unwrap();
        let origin = domain.min();
        let size = domain.size();
        let mut shift = Axis2Placement::IDENTITY;
        shift.set_origin(Vec3::new(-origin.x, -origin.y, 0.));
        for l in ls {
            let eu = p1::loopuse_edgeuses(&b, b.loop_(l).unwrap().loopuse.unwrap()).unwrap();
            for e in eu {
                let (trim, holder) = p1::uv_trim_curve(&b, e).unwrap();
                let mut trim = trim.unwrap().clone();
                trim.transform(&k, &shift, None).unwrap();
                trim.transform(
                    &k,
                    &Axis2Placement::IDENTITY,
                    Some(Vec3::new(1e-5 / size.x, 1e-5 / size.y, 1.)),
                )
                .unwrap();
                b.edgeuse_mut(holder.unwrap()).unwrap().uv_trim_curve = Some(trim);
            }
        }
        let face = b.face_mut(f).unwrap();
        face.surface = Some(next);
        face.uv_domain = Extent2d::from_bounds(0., 0., 1e-5, 1e-5);
        changed += 1;
    }
    assert_eq!(changed, 2);
    b
}
fn input(step: String, assembly: bool) -> Model {
    let digest = format!("{:x}", Sha256::digest(step.as_bytes()));
    let mut v = json!({"version":1,"units":"mm","parameters":{},
        "assets":{"fixture":{"step":step,"sha256":digest}},
        "features":[{"id":"plate","op":"import_step","asset":"fixture","assembly":true}],"output":"plate"});
    if assembly {
        v.as_object_mut().unwrap().remove("output");
        v["assembly"] = json!([{"id":"mounted","input":"plate"}]);
    }
    serde_json::from_value(v).unwrap()
}
fn step(b: &Brep) -> String {
    step_export::write(&step_brep::brep_to_model(b).unwrap()).unwrap()
}
fn check_hole(assembly: bool) {
    let raw = step(&plate());
    if let Ok(dir) = std::env::var("COLOCAD_UV_METRIC_EVIDENCE_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            std::path::Path::new(&dir).join(format!("input-{assembly}.step")),
            &raw,
        )
        .unwrap();
    }
    let result = build(&input(raw.clone(), assembly))
        .unwrap_or_else(|e| panic!("assembly={assembly}: {e:?}"));
    assert_eq!(result.report["geometry"]["solids"], 1);
    assert_eq!(result.report["validation"]["valid"], true);
    let volume = result.report["geometry"]["volume_mm3"].as_f64().unwrap();
    let expected = 20. * 20. * 6. - std::f64::consts::PI * 6.;
    assert!(
        (volume - expected).abs() < expected * 1e-8,
        "volume={volume}"
    );
    assert_eq!(result.report["kernel"]["tolerances"], "kernel_defaults");
    assert!(result.report["kernel"]["upstream_fixes"]
        .as_array()
        .unwrap()
        .contains(&json!("uv_trim_checks_on_surface")));
    if let Ok(dir) = std::env::var("COLOCAD_UV_METRIC_EVIDENCE_DIR") {
        std::fs::write(
            std::path::Path::new(&dir).join(format!("output-{assembly}.step")),
            &result.step,
        )
        .unwrap();
    }
    let reopened = build(&input(result.step, assembly)).unwrap();
    assert_eq!(reopened.report["geometry"]["solids"], 1);
}
#[test]
fn physically_valid_hole_survives_uv_rescaling_in_public_build() {
    check_hole(false);
}
#[test]
fn physically_valid_hole_survives_uv_rescaling_in_assembly() {
    check_hole(true);
}

// ---- 负例：保持原距离及拓扑门禁；不混入1%策略 ----
#[test]
fn vertex_gap_is_rejected_by_single_body_and_assembly() {
    let mut k = Kernel::default();
    let mut b = create_box(&mut k, Vec3::ZERO, 20., 20., 6.).unwrap();
    let vertex = b.vertices().ids().next().unwrap();
    b.vertex_mut(vertex).unwrap().point.x += 0.01;
    let raw = step(&b);
    for assembly in [false, true] {
        let error = build(&input(raw.clone(), assembly))
            .err()
            .expect("a real 0.01mm endpoint gap must fail");
        assert_eq!(error.code, "GEOMETRY_VALIDATION_FAILED", "{error:?}");
    }
}
#[test]
fn wrong_outer_loop_is_not_waived_by_surface_metric() {
    use solidcore::topology::brep::validation::{check_brep, CheckLevel};
    let mut k = Kernel::default();
    k.fixes.uv_trim_checks_on_surface = true;
    let stock = create_box(&mut k, Vec3::ZERO, 20., 20., 6.).unwrap();
    let tool = create_cylinder(&mut k, Vec3::new(10., 10., -1.), 1., 8.).unwrap();
    let mut b = boolean_difference(&mut k, stock, tool).unwrap();
    let original = check_brep(&mut k, &mut b, CheckLevel::Level1).unwrap();
    assert!(original.is_valid(), "{:?}", original.lines());
    let face = b
        .faces()
        .ids()
        .find(|&f| {
            loops(&b, f, false, None).unwrap().len() == 2
                && b.face(f)
                    .unwrap()
                    .surface
                    .as_ref()
                    .unwrap()
                    .is_planar(&k, 1e-10)
                    .unwrap()
                    .is_planar
        })
        .unwrap();
    let use_id = b.face(face).unwrap().faceuse.unwrap();
    b.faceuse_mut(use_id).unwrap().loopuses.swap(0, 1);
    assert!(!check_brep(&mut k, &mut b, CheckLevel::Level1)
        .unwrap()
        .is_valid());
}
#[test]
fn three_dimensional_curve_checks_are_unchanged() {
    use solidcore::geom::assert_array::{AssertArray, ObjectKey};
    use solidcore::nurbs::{
        b_spline_curve::BSplineCurve,
        math::{CPoint, NOW},
        nurbs_crv::allocate_nurb_curve,
    };
    let mut nurb = allocate_nurb_curve(2, 2, 5).unwrap();
    nurb.knots_mut().copy_from_slice(&[0., 0., 0., 1., 1., 1.]);
    nurb.cpts_mut().copy_from_slice(&[
        CPoint::new(0., 0., 0., NOW),
        CPoint::new(0., 0., 0., NOW),
        CPoint::new(1., 0., 0., NOW),
    ]);
    let curve = BSplineCurve::from_nurb(nurb, 3, false);
    let mut results = Vec::new();
    for enabled in [false, true] {
        let mut k = Kernel::default();
        k.fixes.uv_trim_checks_on_surface = enabled;
        let mut list = AssertArray::new();
        let valid = curve
            .assert_valid(&k, Some(&mut list), false, ObjectKey(7))
            .unwrap();
        assert!(!valid);
        results.push(format!("{:?}", list.reports()));
    }
    assert_eq!(results[0], results[1]);
}
#[test]
fn singular_surface_preserves_raw_uv_validation() {
    use solidcore::geom::assert_array::{AssertArray, ObjectKey};
    use solidcore::nurbs::b_spline_curve::{
        p1::{BSplineCurveForm, CanonicalData, KnotType},
        BSplineCurve,
    };
    let control = (0..9)
        .map(|i| {
            let angle = i as f64 * std::f64::consts::PI / 4.;
            let scale = if i % 2 == 1 { 2f64.sqrt() } else { 1. };
            Vec3::new(
                0.5 + scale * (0.003 / 4500.) * angle.cos(),
                0.5 + scale * (0.003 / 4500.) * angle.sin(),
                0.,
            )
        })
        .collect::<Vec<_>>();
    let w = 0.5f64.sqrt();
    let curve = BSplineCurve::create_canonical(&CanonicalData {
        dim: 2,
        degree: 2,
        control_points: &control,
        form: BSplineCurveForm::Unspecified,
        knot_multiplicities: &[3, 2, 2, 2, 3],
        knots: &[0., 0.25, 0.5, 0.75, 1.],
        knot_type: KnotType::Unspecified,
        weights: Some(&[1., w, 1., w, 1., w, 1., w, 1.]),
        trim: None,
    })
    .unwrap();
    let singular = Surface::from(
        BSplineSurface::create_bilinear_surface(Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO)
            .unwrap(),
    );
    let mut results = Vec::new();
    for enabled in [false, true] {
        let mut k = Kernel::default();
        k.fixes.uv_trim_checks_on_surface = enabled;
        let mut list = AssertArray::new();
        let result =
            curve.assert_valid_on(&k, Some(&mut list), true, ObjectKey(9), Some(&singular));
        results.push((result.unwrap_or(false), format!("{:?}", list.reports())));
    }
    assert!(
        !results[0].0,
        "raw small trim must reproduce the reference failure"
    );
    assert_eq!(
        results[0], results[1],
        "no metric must preserve the original rejection"
    );
}
#[test]
fn spherical_poles_and_periodic_seams_survive_public_step_readback() {
    for (op, fields, expected) in [
        (
            "sphere",
            json!({"center":[0,0,0],"radius":5}),
            4. / 3. * std::f64::consts::PI * 125.,
        ),
        (
            "cylinder",
            json!({"origin":[0,0,0],"radius":5,"height":10}),
            std::f64::consts::PI * 250.,
        ),
    ] {
        let mut feature = fields;
        feature["id"] = json!("body");
        feature["op"] = json!(op);
        let model: Model = serde_json::from_value(
            json!({"version":1,"units":"mm","parameters":{},"features":[feature],"output":"body"}),
        )
        .unwrap();
        let built = build(&model).unwrap();
        let reopened = build(&input(built.step, false)).unwrap();
        let volume = reopened.report["geometry"]["volume_mm3"].as_f64().unwrap();
        assert!((volume - expected).abs() < expected * 1e-8);
    }
}

#[test]
fn independent_uv_definitions_pass_early_assembly_validation() {
    let raw = step(&plate());
    let digest = format!("{:x}", Sha256::digest(raw.as_bytes()));
    let model:Model=serde_json::from_value(json!({"version":1,"units":"mm","parameters":{},
        "assets":{"fixture":{"step":raw,"sha256":digest}},
        "features":[{"id":"first","op":"import_step","asset":"fixture","assembly":true},
                    {"id":"second","op":"import_step","asset":"fixture","assembly":true}],
        "assembly":[{"id":"left","input":"first"},{"id":"right","input":"second","translation":[30,0,0]}]})).unwrap();
    let built = build(&model).unwrap();
    assert_eq!(built.report["geometry"]["solids"], 2);
    assert_eq!(built.report["early_released_definitions"], 2);
    assert_eq!(built.report["validation"]["measured_instances"], 2);
}

#[test]
fn genuine_collapsed_uv_span_is_still_rejected_with_a_regular_surface_metric() {
    use solidcore::geom::assert_array::{AssertArray, ObjectKey};
    use solidcore::nurbs::b_spline_curve::{
        p1::{BSplineCurveForm, CanonicalData, KnotType},
        BSplineCurve,
    };
    let controls = [
        Vec3::new(0.1, 0.1, 0.),
        Vec3::new(0.5, 0.5, 0.),
        Vec3::new(0.5, 0.5, 0.),
        Vec3::new(0.9, 0.1, 0.),
    ];
    let curve = BSplineCurve::create_canonical(&CanonicalData {
        dim: 2,
        degree: 1,
        control_points: &controls,
        form: BSplineCurveForm::Unspecified,
        knot_multiplicities: &[2, 1, 1, 2],
        knots: &[0., 1. / 3., 2. / 3., 1.],
        knot_type: KnotType::Unspecified,
        weights: None,
        trim: None,
    })
    .unwrap();
    let plane = Surface::from(
        BSplineSurface::create_bilinear_surface(
            Vec3::ZERO,
            Vec3::new(10., 0., 0.),
            Vec3::new(0., 20., 0.),
            Vec3::new(10., 20., 0.),
        )
        .unwrap(),
    );
    for enabled in [false, true] {
        let mut k = Kernel::default();
        k.fixes.uv_trim_checks_on_surface = enabled;
        let mut list = AssertArray::new();
        assert!(!curve
            .assert_valid_on(&k, Some(&mut list), true, ObjectKey(11), Some(&plane))
            .unwrap());
        assert!(
            (0..list.len()).any(|i| list.get(i).is_some_and(|r| r.test_index == 15)),
            "{:?}",
            list.reports()
        );
    }
}
