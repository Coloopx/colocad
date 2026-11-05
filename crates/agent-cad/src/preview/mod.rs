//! Native, bounded display artifacts. Display geometry never drives exact measurements.
use crate::*;
mod analytic;
mod section;
pub use section::SectionPlane;
mod cache;
mod record;
mod render;
pub use cache::{observe, MeshScene, ObserveRequest};
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PreviewOptions {
    #[serde(default)]
    pub pose: placement::PoseMode,
    #[serde(default = "chord")]
    pub chord_mm: f64,
    #[serde(default = "width")]
    pub width: u32,
    #[serde(default = "height")]
    pub height: u32,
    #[serde(default)]
    pub view: View,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<SectionPlane>,
}
fn chord() -> f64 {
    0.2
}
fn width() -> u32 {
    800
}
fn height() -> u32 {
    600
}
impl Default for PreviewOptions {
    fn default() -> Self {
        Self {
            pose: Default::default(),
            chord_mm: chord(),
            width: width(),
            height: height(),
            view: View::default(),
            section: None,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum View {
    #[default]
    Iso,
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom,
}
impl PreviewOptions {
    pub fn validate(&self) -> Result<()> {
        if let Some(section) = self.section {
            section.unit()?;
        }
        if !self.chord_mm.is_finite()
            || !(0.005..=5.).contains(&self.chord_mm)
            || !(64..=1600).contains(&self.width)
            || !(64..=1200).contains(&self.height)
        {
            return Err(CadError::new(
                "PREVIEW_BUDGET",
                "chord 0.005..5 mm; width 64..1600; height 64..1200",
            ));
        }
        Ok(())
    }
}
fn error(e: impl std::fmt::Display) -> CadError {
    CadError::new("PREVIEW_FAILED", e.to_string())
}
pub(crate) struct InstanceMesh<'a> {
    pub id: String,
    pub definition: &'a str,
    pub body: &'a Brep,
    pub pose: placement::Rigid,
}
fn placed(mesh: &solidcore_tess::Mesh, pose: placement::Rigid) -> solidcore_tess::Mesh {
    use solidcore_record::Vec3 as P;
    let convert = |p: P, is_point: bool| {
        let v = Vec3::new(p.x, p.y, p.z);
        let v = if is_point {
            pose.point(v)
        } else {
            pose.vector(v)
        };
        P::new(v.x, v.y, v.z)
    };
    let mut out = mesh.clone();
    for p in &mut out.positions {
        *p = convert(*p, true);
    }
    for n in &mut out.normals {
        *n = convert(*n, false);
    }
    for p in out.edges.iter_mut().flatten() {
        *p = convert(*p, true);
    }
    out
}
pub(crate) fn finish(
    k: &mut Kernel,
    m: &Model,
    bodies: Vec<InstanceMesh<'_>>,
    cache: &mut incremental::Checkpoints,
) -> Result<(BTreeMap<String, Vec<u8>>, Value)> {
    let start = Instant::now();
    let Some(options) = &m.preview else {
        return Ok((BTreeMap::new(), Value::Null));
    };
    if bodies.is_empty() {
        return Err(CadError::new(
            "PREVIEW_EMPTY",
            "requested pose has no visible parts",
        ));
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .stack_size(8 << 20)
        .build()
        .map_err(error)?;
    let mut meshes = vec![];
    let mut convert_ms = 0.;
    let mut plan_ms = 0.;
    let mut evaluate_ms = 0.;
    let mut remaining = 200_000;
    let mut definitions = BTreeMap::new();
    let mut mesh_calls = 0usize;
    let mut analytic_calls = 0usize;
    let mut revision_hits = 0usize;
    for instance in bodies {
        let b = instance.body;
        let mesh_name = cache.mesh_name(instance.definition, options.chord_mm);
        if !definitions.contains_key(instance.definition) {
            if let Some(mesh) = mesh_name.as_ref().and_then(|n| cache.read_mesh(n)) {
                definitions.insert(instance.definition, mesh);
                revision_hits += 1;
            }
        }
        if !definitions.contains_key(instance.definition) {
            let tick = Instant::now();
            solidcore_tess::preview::admit(b.face_list().len(), remaining).map_err(error)?;
            let mesh = if let Some(mesh) = analytic::mesh(m, instance.definition, options.chord_mm)?
            {
                analytic_calls += 1;
                mesh
            } else {
                let record = record::convert(k, b)?;
                let bb = brep_bounding_box(k, b, true)?;
                let size = (bb.max() - bb.min()).length();
                convert_ms += tick.elapsed().as_secs_f64() * 1000.;
                let tick = Instant::now();
                let (plan, stats) = pool
                    .install(|| {
                        solidcore_tess::plan(
                            &record,
                            &solidcore_tess::MeshTolerance::display(options.chord_mm),
                            size,
                        )
                    })
                    .map_err(error)?;
                plan_ms += tick.elapsed().as_secs_f64() * 1000.;
                if !stats.unmeshed.is_empty()
                    || !stats.lenient.is_empty()
                    || !stats.over_tolerance.is_empty()
                    || !stats.edges_capped.is_empty()
                {
                    return Err(error(format!("incomplete mesh: {stats:?}")));
                }
                let tick = Instant::now();
                let (mesh, refused) = plan.evaluate();
                evaluate_ms += tick.elapsed().as_secs_f64() * 1000.;
                if !refused.is_empty() {
                    return Err(error(format!("unevaluable faces: {refused:?}")));
                }
                mesh
            };
            if let Some(name) = mesh_name {
                cache.save_mesh(name, &mesh);
            }
            definitions.insert(instance.definition, mesh);
            mesh_calls += 1;
        }
        let mesh = &definitions[instance.definition];
        solidcore_tess::preview::charge(mesh, &mut remaining).map_err(error)?;
        meshes.push(cache::SceneInstance::from_pose(
            instance.id,
            instance.definition,
            instance.pose,
        ));
    }
    let scene = MeshScene {
        schema: "agent-cad.mesh-scene.v2".into(),
        source_hash: model_hash(m),
        chord_mm: options.chord_mm,
        pose: options.pose,
        meshes: definitions
            .into_iter()
            .map(|(id, m)| (id.to_owned(), m))
            .collect(),
        instances: meshes,
    };
    let parts = scene.parts()?;
    let (mut artifacts, render_ms) = render::artifacts(&parts, options)?;
    let parts = parts
        .iter()
        .map(|part| json!({"id":part.id,"triangles":part.mesh.triangles.len()}))
        .collect::<Vec<_>>();
    let tick = Instant::now();
    let cache = serde_json::to_vec(&scene).map_err(error)?;
    let cache_bytes = cache.len();
    let cache_available = cache_bytes <= 24 * 1024 * 1024
        && cache_bytes + artifacts.values().map(Vec::len).sum::<usize>() <= 32 * 1024 * 1024;
    // A cache is optional: its storage budget must not reject an otherwise valid build.
    if cache_available {
        artifacts.insert("preview.mesh.json".into(), cache);
    }
    let cache_ms = tick.elapsed().as_secs_f64() * 1000.;
    let report = json!({"complete":true,"cache_hit":false,"cache_available":cache_available,"chord_mm":options.chord_mm,"pose":options.pose,"source_triangles":200_000-remaining,"triangles":render_ms["display_triangles"],"section":options.section,"revision_mesh_hits":revision_hits,"instance_mesh_hits":parts.len()-mesh_calls-revision_hits,"tessellation_calls":mesh_calls,"analytic_tessellations":analytic_calls,"width":options.width,"height":options.height,"view":options.view,"parts":parts,"display_only":true,"cache_bytes":if cache_available {cache_bytes} else {0},"timing_ms":{"record_conversion":convert_ms,"mesh_plan":plan_ms,"mesh_evaluate":evaluate_ms,"render":render_ms,"cache_encode":cache_ms,"total":start.elapsed().as_secs_f64()*1000.},"files":artifacts.keys().collect::<Vec<_>>()});
    Ok((artifacts, report))
}
