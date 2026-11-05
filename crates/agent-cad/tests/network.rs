use agent_cad::{build, Model};
use serde_json::json;
#[test]
fn network_interpolates_interior_guides_and_rejects_inconsistent_crossings() {
    let mut model = json!({"version":1,"units":"mm","parameters":{},"output_kind":"surface","features":[{"op":"surface_network","id":"skin","u_stations":[0,0.5,1],"v_stations":[0,0.5,1],
    "u_curves":[[[0,0,0],[5,0,0],[10,0,0]],[[0,5,0],[5,5,4],[10,5,0]],[[0,10,0],[5,10,0],[10,10,0]]],
    "v_curves":[[[0,0,0],[0,5,0],[0,10,0]],[[5,0,0],[5,5,4],[5,10,0]],[[10,0,0],[10,5,0],[10,10,0]]]}],"output":"skin","preview":{}});
    let b = build(&serde_json::from_value::<Model>(model.clone()).unwrap()).unwrap();
    assert_eq!(b.report["geometry"]["faces"], 1);
    assert!(b.step.contains("B_SPLINE_SURFACE"));
    assert!(b.step.contains("SHELL_BASED_SURFACE_MODEL"));
    model["features"][0]["v_curves"][1][1][2] = json!(8);
    assert_eq!(
        build(&serde_json::from_value(model).unwrap())
            .err()
            .unwrap()
            .code,
        "NETWORK_MISMATCH"
    );
}
