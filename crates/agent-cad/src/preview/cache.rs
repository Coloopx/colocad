//! Checked display cache; no B-rep or exact measurement authority.
use super::*;
/// Display-only cache. It is tied to a built revision, pose and tessellation precision.
/// Camera and image dimensions may change without invoking the geometry kernel.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MeshScene {
    pub schema: String,
    pub source_hash: String,
    pub chord_mm: f64,
    pub pose: placement::PoseMode,
    pub meshes: Vec<(String, solidcore_tess::Mesh)>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub instances: Vec<SceneInstance>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneInstance {
    pub id: String,
    pub definition: String,
    pub axes: [[f64; 3]; 3],
    pub origin: [f64; 3],
}
impl SceneInstance {
    pub(super) fn from_pose(id: String, definition: &str, pose: placement::Rigid) -> Self {
        let xyz = |v: Vec3| [v.x, v.y, v.z];
        Self {
            id,
            definition: definition.into(),
            axes: pose.axes.map(xyz),
            origin: xyz(pose.origin),
        }
    }
    fn rigid(&self) -> Result<placement::Rigid> {
        let v = |p: [f64; 3]| Vec3::new(p[0], p[1], p[2]);
        let [a, b, c] = self.axes.map(v);
        if self
            .axes
            .iter()
            .flatten()
            .chain(self.origin.iter())
            .any(|x| !x.is_finite())
            || self.origin.iter().any(|v| v.abs() > MAX_COORD * 10.)
            || [a, b, c].iter().any(|v| (v.length() - 1.).abs() > 1e-8)
            || a.dot(b).abs() > 1e-8
            || (a.cross(b) - c).length() > 1e-8
        {
            return Err(CadError::new(
                "INVALID_MESH_CACHE",
                "invalid rigid instance transform",
            ));
        }
        Ok(placement::Rigid {
            axes: [a, b, c],
            origin: v(self.origin),
        })
    }
}
pub(super) struct Part<'a> {
    pub id: &'a str,
    pub mesh: &'a solidcore_tess::Mesh,
    pub pose: placement::Rigid,
}
impl MeshScene {
    pub(super) fn parts(&self) -> Result<Vec<Part<'_>>> {
        if self.schema == "agent-cad.mesh-scene.v1" && self.instances.is_empty() {
            return Ok(self
                .meshes
                .iter()
                .map(|(id, m)| Part {
                    id,
                    mesh: m,
                    pose: Default::default(),
                })
                .collect());
        }
        if self.schema != "agent-cad.mesh-scene.v2"
            || self.instances.is_empty()
            || self.instances.len() > 128
        {
            return Err(CadError::new(
                "INVALID_MESH_CACHE",
                "invalid instance table",
            ));
        }
        let mut out = Vec::new();
        let mut total = 0;
        let mut ids = BTreeSet::new();
        let defs = self
            .meshes
            .iter()
            .map(|(id, m)| (id.as_str(), m))
            .collect::<BTreeMap<_, _>>();
        if defs.len() != self.meshes.len() {
            return Err(CadError::new("INVALID_MESH_CACHE", "duplicate definition"));
        }
        for i in &self.instances {
            let mesh = *defs
                .get(i.definition.as_str())
                .ok_or_else(|| CadError::new("INVALID_MESH_CACHE", "missing definition"))?;
            total += mesh.triangles.len();
            if !ids.insert(&i.id) || total > 200_000 {
                return Err(CadError::new(
                    "PREVIEW_BUDGET",
                    "invalid instance count or triangle sum",
                ));
            }
            out.push(Part {
                id: &i.id,
                mesh,
                pose: i.rigid()?,
            });
        }
        Ok(out)
    }
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserveRequest {
    pub scene: MeshScene,
    #[serde(default)]
    pub options: PreviewOptions,
}

pub fn observe(request: &ObserveRequest) -> Result<(BTreeMap<String, Vec<u8>>, Value)> {
    let start = Instant::now();
    let scene = &request.scene;
    let o = &request.options;
    o.validate()?;
    if !matches!(
        scene.schema.as_str(),
        "agent-cad.mesh-scene.v1" | "agent-cad.mesh-scene.v2"
    ) || scene.chord_mm != o.chord_mm
        || serde_json::to_value(scene.pose).map_err(error)?
            != serde_json::to_value(o.pose).map_err(error)?
    {
        return Err(CadError::new(
            "PREVIEW_CACHE_MISMATCH",
            "cache schema, precision or pose differs; rebuild required",
        ));
    }
    if scene.meshes.is_empty() || scene.meshes.len() > 128 {
        return Err(CadError::new(
            "PREVIEW_BUDGET",
            "cache requires 1..128 meshes",
        ));
    }
    let mut triangles = 0usize;
    let mut vertices = 0usize;
    for (_, m) in &scene.meshes {
        triangles += m.triangles.len();
        vertices += m.positions.len();
        if triangles > 200_000 || vertices > 600_000 {
            return Err(CadError::new("PREVIEW_BUDGET", "cache exceeds mesh budget"));
        }
        if m.positions.is_empty()
            || m.triangles.is_empty()
            || m.positions.len() != m.normals.len()
            || m.positions
                .iter()
                .chain(&m.normals)
                .any(|p| !p.x.is_finite() || !p.y.is_finite() || !p.z.is_finite())
            || m.positions
                .iter()
                .any(|p| p.x.abs().max(p.y.abs()).max(p.z.abs()) > MAX_COORD * 10.)
            || m.triangles
                .iter()
                .flatten()
                .any(|&i| i as usize >= m.positions.len())
        {
            return Err(CadError::new(
                "INVALID_MESH_CACHE",
                "invalid positions, normals or triangle indices",
            ));
        }
    }
    let parts = scene.parts()?;
    let triangles = parts.iter().map(|p| p.mesh.triangles.len()).sum::<usize>();
    let (artifacts, render_ms) = render::artifacts(&parts, o)?;
    let report = json!({"display_only":true,"source_hash":scene.source_hash,"cache_hit":true,"geometry_rebuilt":false,"tessellation_calls":0,"source_triangles":triangles,"triangles":render_ms["display_triangles"],"section":o.section,"view":o.view,"pose":o.pose,"width":o.width,"height":o.height,"timing_ms":{"render":render_ms,"total":start.elapsed().as_secs_f64()*1000.}});
    Ok((artifacts, report))
}
