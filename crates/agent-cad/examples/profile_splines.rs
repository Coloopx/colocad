// Inspect the actual curves in a closed native snapshot; timings are outside benchmark cgroups.
use agent_cad::query::artifact::Manifest;
use solidcore::curve::curve::{p2::CurveKind, Curve};
use solidcore::geom::assert_array::ObjectKey;
use solidcore::topology::brep::validation::curve_assert_valid;
use solidcore::Kernel;
use std::{collections::BTreeMap, path::PathBuf, time::Instant};

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).unwrap());
    let rev: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("query-manifest.json")).unwrap()).unwrap();
    let manifest = Manifest::open(&root, rev["query_revision_id"].as_str().unwrap()).unwrap();
    let definition = std::env::args()
        .nth(2)
        .expect("supply the definition ID after the artifact directory");
    let mut k = Kernel::default();
    let body = manifest.load(&root, &definition, &mut k).unwrap().body;
    let mut stats = BTreeMap::<String, (usize, f64)>::new();
    let mut run = |curve: &Curve, role: &str| {
        let mut shape = curve.clone();
        let name = if let CurveKind::BSpline(s) = shape.kind() {
            let pts = s.control_points().unwrap();

            format!(
                "{role} spline degree {} poles {} rational {} first {:?} last {:?}",
                s.degree().unwrap(),
                pts.len(),
                s.is_rational().unwrap(),
                pts[0],
                pts[pts.len() - 1]
            )
        } else {
            format!("{role} {:?}", std::mem::discriminant(shape.kind()))
        };
        let t = Instant::now();
        let _r = curve_assert_valid(&k, &mut shape, None, 1, ObjectKey(5)).unwrap();
        let x = stats.entry(name).or_default();
        x.0 += 1;
        x.1 += t.elapsed().as_secs_f64() * 1000.;
    };
    for id in body.edges().ids() {
        if let Some(c) = &body.edge(id).unwrap().curve {
            run(c, "edge")
        }
    }
    for id in body.edgeuses().ids() {
        if let Some(c) = &body.edgeuse(id).unwrap().uv_trim_curve {
            run(c, "trim")
        }
    }
    let mut vals: Vec<_> = stats.into_iter().collect();
    vals.sort_by(|a, b| b.1 .1.total_cmp(&a.1 .1));
    for (tag, (count, ms)) in vals {
        println!("{ms:.3} ms {count} {tag}");
    }
}
