use agent_cad::{build, Model};
use serde_json::{json, Value};
fn model(origin: [f64; 3], size: [f64; 3]) -> Value {
    json!({"version":1,"units":"mm","parameters":{},"features":[{"id":"a","op":"box","origin":[0,0,0],"size":[10,10,10]},{"id":"b","op":"box","origin":[0,0,0],"size":size}],"assembly":[{"id":"base","input":"a"},{"id":"moving","input":"b","translation":origin}],"checks":[{"kind":"geometric_relationship","a":{"instance":"base"},"b":{"instance":"moving"},"tolerance":{"linear_mm":1e-5,"angular_rad":1e-7},"max_overlap_mm3":1000}]})
}
#[test]
fn exact_relationship_distinguishes_gap_contact_penetration_and_containment() {
    for (o, size, relation, gap, overlap) in [
        ([10.2, 0., 0.], [10., 10., 10.], "separated", 0.2, 0.),
        ([10., 0., 0.], [10., 10., 10.], "touching", 0., 0.),
        ([9., 0., 0.], [10., 10., 10.], "overlapping", 0., 100.),
        ([2., 2., 2.], [2., 2., 2.], "contained", 2., 8.),
    ] {
        let m: Model = serde_json::from_value(model(o, size)).unwrap();
        let b = build(&m).unwrap();
        let r = &b.report["assembly"]["checks"][0]["samples"][0];
        assert_eq!(r["relationship"], relation, "{r}");
        assert!(
            (r["boundary_distance_mm"].as_f64().unwrap() - gap).abs() < 1e-5,
            "{r}"
        );
        assert!(
            (r["overlap_mm3"].as_f64().unwrap() - overlap).abs() < 1e-5,
            "{r}"
        );
        assert!(r["witness"].is_object(), "{r}");
    }
}
#[test]
fn required_clearance_and_containment_cannot_pass_from_boundary_gap_alone() {
    let mut v = model([2., 2., 2.], [2., 2., 2.]);
    v["checks"][0]["min_clearance_mm"] = json!(0.2);
    let m: Model = serde_json::from_value(v).unwrap();
    let e = build(&m).err().unwrap();
    assert_eq!(e.code, "GEOMETRIC_RELATIONSHIP_FAILED");
    assert_eq!(
        e.details.unwrap()["samples"][0]["relationship"],
        "contained"
    );
    let mut v = model([10.1, 0., 0.], [10., 10., 10.]);
    v["checks"][0]["min_clearance_mm"] = json!(0.2);
    let m: Model = serde_json::from_value(v).unwrap();
    assert_eq!(
        build(&m).err().unwrap().code,
        "GEOMETRIC_RELATIONSHIP_FAILED"
    );
}
#[test]
fn explicit_pose_samples_are_checked_and_failed_receipts_retain_the_pose() {
    let mut v = model([10.2, 0., 0.], [10., 10., 10.]);
    v["checks"][0]["max_overlap_mm3"] = json!(0);
    v["checks"][0]["pose_samples"] = json!([{"id":"home"},{"id":"contact","b":{"translation":[-0.2,0,0]}},{"id":"penetration","b":{"translation":[-1,0,0]}}]);
    let m: Model = serde_json::from_value(v).unwrap();
    let e = build(&m).err().unwrap();
    assert_eq!(e.code, "GEOMETRIC_RELATIONSHIP_FAILED");
    let d = e.details.unwrap();
    assert_eq!(d["samples"].as_array().unwrap().len(), 3);
    assert_eq!(d["samples"][2]["pose_id"], "penetration");
    assert_eq!(d["samples"][2]["relationship"], "overlapping");
}
fn contact() -> Value {
    json!({"a":{"axis":"x","value":10,"tolerance":1e-5,"expected_count":1},"b":{"axis":"x","value":10,"tolerance":1e-5,"expected_count":1}})
}
#[test]
fn allowed_planar_contact_does_not_exempt_interference_elsewhere_on_the_pair() {
    let mut v = model([10., 0., 0.], [10., 10., 10.]);
    v["checks"][0]["max_overlap_mm3"] = json!(0);
    v["checks"][0]["min_clearance_mm"] = json!(0.2);
    v["checks"][0]["allowed_contacts"] = json!([contact()]);
    let m: Model = serde_json::from_value(v.clone()).unwrap();
    let b = build(&m).unwrap();
    assert_eq!(
        b.report["assembly"]["checks"][0]["samples"][0]["contact_evidence"]["status"],
        "certified_separating_plane"
    );
    // A second solid on the same moving occurrence collides elsewhere while the installation faces still meet.
    v["features"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"intruder","op":"box","origin":[-5,4,4],"size":[6,2,2]}));
    v["features"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"movingPart","op":"union","left":"b","right":"intruder"}));
    v["assembly"][1]["input"] = json!("movingPart");
    let m: Model = serde_json::from_value(v).unwrap();
    let e = build(&m).err().unwrap();
    assert_eq!(e.code, "GEOMETRIC_RELATIONSHIP_FAILED", "{e}");
    assert_eq!(
        e.details.unwrap()["samples"][0]["relationship"],
        "overlapping"
    );
}
