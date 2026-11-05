use agent_cad::{build, Model};
use serde_json::json;
#[test]
fn native_mesh_and_views_cover_all_faces_and_keep_holes() {
    let mut m: Model = serde_json::from_str(include_str!("../../../examples/plate.json")).unwrap();
    m.preview = Some(Default::default());
    let b = build(&m).unwrap();
    assert!(b.report["preview"]["triangles"].as_u64().unwrap() > 12);
    assert_eq!(&b.artifacts["preview.png"][..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(&b.artifacts["model.glb"][..4], b"glTF");
    let stl = &b.artifacts["model.stl"];
    let count = u32::from_le_bytes(stl[80..84].try_into().unwrap()) as usize;
    assert_eq!(stl.len(), 84 + 50 * count);
    let mut volume = 0.;
    for tri in stl[84..].as_chunks::<50>().0 {
        let points = (0..3)
            .map(|i| {
                let j = 12 + i * 12;
                (0..3)
                    .map(|k| {
                        f32::from_le_bytes(tri[j + k * 4..j + k * 4 + 4].try_into().unwrap()) as f64
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let (a, b, c) = (&points[0], &points[1], &points[2]);
        volume += (a[0] * (b[1] * c[2] - b[2] * c[1])
            + a[1] * (b[2] * c[0] - b[0] * c[2])
            + a[2] * (b[0] * c[1] - b[1] * c[0]))
            / 6.;
    }
    let exact = b.report["geometry"]["volume_mm3"].as_f64().unwrap();
    assert!(
        (volume - exact).abs() / exact < 0.005,
        "mesh={volume}, exact={exact}"
    );
    let mut no = m.clone();
    no.preview = None;
    assert_eq!(build(&no).unwrap().step, b.step);
}
#[test]
fn preview_budget_is_validated() {
    let m:Model=serde_json::from_value(json!({"version":1,"units":"mm","parameters":{},"features":[{"op":"box","id":"a","origin":[0,0,0],"size":[1,1,1]}],"output":"a","preview":{"width":80000}})).unwrap();
    assert_eq!(build(&m).err().unwrap().code, "PREVIEW_BUDGET");
}

#[test]
fn cached_observation_matches_fresh_pixels_and_rejects_stale_precision() {
    use agent_cad::preview::{observe, MeshScene, ObserveRequest, View};
    let mut m: Model = serde_json::from_str(include_str!("../../../examples/plate.json")).unwrap();
    m.preview = Some(Default::default());
    let original = build(&m).unwrap();
    for view in [View::Iso, View::Top, View::Front] {
        let scene: MeshScene =
            serde_json::from_slice(&original.artifacts["preview.mesh.json"]).unwrap();
        let mut options = m.preview.clone().unwrap();
        options.view = view;
        let (cached, report) = observe(&ObserveRequest {
            scene,
            options: options.clone(),
        })
        .unwrap();
        m.preview = Some(options);
        let fresh = build(&m).unwrap();
        for name in ["preview.png", "model.glb", "model.stl"] {
            assert_eq!(cached[name], fresh.artifacts[name], "{name} {view:?}");
        }
        assert_eq!(report["tessellation_calls"], 0);
    }
    let scene: MeshScene =
        serde_json::from_slice(&original.artifacts["preview.mesh.json"]).unwrap();
    let mut request = ObserveRequest {
        scene,
        options: Default::default(),
    };
    request.options.chord_mm = 0.1;
    assert_eq!(
        observe(&request).unwrap_err().code,
        "PREVIEW_CACHE_MISMATCH"
    );
    request.options.chord_mm = 0.2;
    request.options.pose = agent_cad::placement::PoseMode::Exploded;
    assert_eq!(
        observe(&request).unwrap_err().code,
        "PREVIEW_CACHE_MISMATCH"
    );
    request.options.pose = agent_cad::placement::PoseMode::Normal;
    request.scene.meshes[0].1.triangles[0][0] = u32::MAX;
    assert_eq!(observe(&request).unwrap_err().code, "INVALID_MESH_CACHE");
}
#[test]
fn winding_is_outward_for_curved_shells_and_voids() {
    for input in [
        include_str!("../../../examples/v0.2/shell.json"),
        include_str!("../../../examples/v0.2/pipe.json"),
        include_str!("../../../examples/v0.2/revolve.json"),
        include_str!("../../../examples/v0.2/profile_loft.json"),
    ] {
        let mut m: Model = serde_json::from_str(input).unwrap();
        m.preview = Some(Default::default());
        let b = build(&m).unwrap();
        let stl = &b.artifacts["model.stl"];
        let mut volume = 0.;
        for tri in stl[84..].as_chunks::<50>().0 {
            let p = (0..3)
                .map(|i| {
                    let j = 12 + i * 12;
                    (0..3)
                        .map(|k| {
                            f32::from_le_bytes(tri[j + k * 4..j + k * 4 + 4].try_into().unwrap())
                                as f64
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            let (a, b, c) = (&p[0], &p[1], &p[2]);
            volume += (a[0] * (b[1] * c[2] - b[2] * c[1])
                + a[1] * (b[2] * c[0] - b[0] * c[2])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.;
        }
        let exact = b.report["geometry"]["volume_mm3"].as_f64().unwrap();
        assert!(volume > 0., "{} signed mesh {volume}", m.output);
        assert!(
            (volume - exact).abs() / exact < 0.015,
            "{} mesh {volume}, exact {exact}",
            m.output
        );
    }
}
