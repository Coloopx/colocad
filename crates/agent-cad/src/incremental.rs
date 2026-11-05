//! Revision-local cache. Keys cover the operation, its transitive inputs, the referenced
//! parameters, asset digests and kernel version. Only the previous trusted revision is read.
use crate::*;
use solidcore::geom::attribute::{AttributeStore, ObjectKey};
use solidcore::topology::{
    api_files, brep::p10::TopologyEnv, brep_data_write::FromBrepEnv, topology::NotifyRecords,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

const BUDGET: usize = 12 * 1024 * 1024;

#[derive(Default)]
pub struct Checkpoints {
    enabled: bool,
    previous: Option<(PathBuf, Value)>,
    pub(crate) keys: BTreeMap<String, String>,
    pub(crate) files: BTreeMap<String, Vec<u8>>,
    used: usize,
    terminal: BTreeSet<String>,
}
impl Checkpoints {
    pub fn revision(previous: Option<&Path>) -> Result<Self> {
        if previous.is_some_and(|p| !p.join("report.json").exists()) {
            return Ok(Self {
                enabled: true,
                ..Default::default()
            });
        }
        let previous = previous
            .map(|dir| {
                let path = dir.join("report.json");
                if fs::metadata(&path).map_err(io)?.len() > 2 * 1024 * 1024 {
                    return Err(io("oversized checkpoint manifest"));
                }
                let report: Value =
                    serde_json::from_slice(&fs::read(path).map_err(io)?).map_err(io)?;
                Ok((dir.to_owned(), report))
            })
            .transpose()?;
        Ok(Self {
            enabled: true,
            previous,
            ..Default::default()
        })
    }
    pub(crate) fn prepare(&mut self, model: &Model) {
        if !self.enabled {
            return;
        }
        self.keys.clear();
        self.files.clear();
        self.used = 0;
        self.terminal = model.features.iter().map(|f| f.id().to_string()).collect();
        for f in &model.features {
            for id in f.inputs() {
                self.terminal.remove(id);
            }
        }
        let assets = model
            .assets
            .iter()
            .map(|(id, a)| (id, &a.sha256))
            .collect::<BTreeMap<_, _>>();
        for f in &model.features {
            let operation = serde_json::to_value(f).expect("serialized feature");
            let mut params = BTreeMap::new();
            collect_params(&operation, &model.parameters, &mut params);
            let parents = f
                .inputs()
                .into_iter()
                .map(|id| &self.keys[id])
                .collect::<Vec<_>>();
            let spec = json!([
                EXECUTOR_VERSION,
                KERNEL_REV,
                operation,
                params,
                parents,
                assets
            ]);
            let key = format!("{:x}", Sha256::digest(spec.to_string().as_bytes()));
            self.keys.insert(f.id().into(), key);
        }
    }
    fn read(&self, name: &str) -> Option<Vec<u8>> {
        let (dir, report) = self.previous.as_ref()?;
        let expected = report["artifact_sha256"][name].as_str()?;
        if report["executor_version"] != EXECUTOR_VERSION {
            return None;
        }
        let path = dir.join(name);
        if fs::symlink_metadata(&path).ok()?.file_type().is_symlink()
            || fs::metadata(&path).ok()?.len() > BUDGET as u64
        {
            return None;
        }
        let data = fs::read(path).ok()?;
        (format!("{:x}", Sha256::digest(&data)) == expected).then_some(data)
    }
    fn store(&mut self, name: String, bytes: Vec<u8>) {
        if bytes.len() <= BUDGET - self.used && !self.files.contains_key(&name) {
            self.used += bytes.len();
            self.files.insert(name, bytes);
        }
    }
    pub(crate) fn restore(&mut self, k: &mut Kernel, id: &str) -> Option<(Brep, Value)> {
        let key = self.keys.get(id)?;

        let trace = self.previous.as_ref()?.1["features"]
            .as_array()?
            .iter()
            .find(|v| v["id"] == id && v["checkpoint"] == key.as_str())?
            .clone();
        let use_step = trace["checkpoint_format"] == "step-terminal";
        if use_step && !self.terminal.contains(id) {
            return None;
        }
        let name = format!("geom-{key}.{}", if use_step { "step" } else { "smb" });
        let bytes = self.read(&name)?;
        let mut b = if use_step {
            let model = step_import::import(&bytes, &step_import::Limits::default()).ok()?;
            if model.bodies.len() != 1 || !model.skipped.is_empty() || !model.diagnostics.is_empty()
            {
                return None;
            }
            exchange::build_body(k, &model.bodies[0]).ok()?
        } else {
            let mut attrs = AttributeStore::new();
            let mut records = NotifyRecords::new();
            let owner = |e| solidcore::topology::brep::p3::topology_ref(1, e);
            let mut env = TopologyEnv {
                all_attributes: &[],
                attributes: &mut attrs,
                key_of: &owner,
                records: &mut records,
                next_key: 1 << 50,
            };

            api_files::read_brep_from_file(k, Some(bytes.clone()), false, false, &mut env).ok()?
        };
        b.refresh_cached_topology();
        if b.face_list().len() as u64 != trace["faces"].as_u64()?
            || (!self.terminal.contains(id)
                && !check_brep(k, &mut b, CheckLevel::Level1).ok()?.is_valid())
        {
            return None;
        }
        if !self.terminal.contains(id)
            && signature(&b).as_deref() != Some(trace["intermediate_sha256"].as_str()?)
        {
            return None;
        }
        self.store(name, bytes);
        Some((b, trace))
    }
    pub(crate) fn capture(&mut self, k: &Kernel, id: &str, b: &Brep, trace: &mut Value) {
        let Some(key) = self.keys.get(id).cloned() else {
            return;
        };
        if b.face_list().len() > 512 || self.used >= BUDGET {
            trace["checkpoint_skip"] = json!("budget");
            return;
        }
        if !b.attributed_entities().is_empty() {
            trace["checkpoint_skip"] = json!("attributed_topology");
            return;
        }
        if !self.terminal.contains(id) {
            let Some(digest) = signature(b) else {
                return;
            };
            trace["intermediate_sha256"] = json!(digest);
        }
        let attrs = AttributeStore::new();
        let records = NotifyRecords::new();
        let owner = |e| b.owner_identity(e);
        let env = FromBrepEnv {
            attributes: &attrs,
            records: &records,
            key_of: &owner,
            curve_key: &|_| ObjectKey(0),
            uv_curve_key: &|_| ObjectKey(0),
            surface_key: &|_| ObjectKey(0),
        };
        // Some analytic trims carry lazy line geometry. Materialize only the UV
        // record on a disposable snapshot, preserving the exact surface in the B-rep.
        let serialized = (|| -> solidcore::Result<Vec<u8>> {
            use solidcore::curve::curve::{p2::CurveKind, Curve};
            use solidcore::topology::{brep_data::BrepData, database_io::FileType};
            let mut collected = Vec::new();
            let mut records = BrepData::from_brep(b, &env, &mut collected)?;
            for trim in &mut records.uv_curves {
                if let CurveKind::Line(line) = trim.object.kind() {
                    trim.object = Curve::from(line.base_with_nurb()?.into_owned());
                }
            }
            records.write_to_file_with_nurbs(k, FileType::Binary)
        })();
        match serialized {
            Ok(data) => {
                let name = format!("geom-{key}.smb");
                self.store(name.clone(), data);
                if self.files.contains_key(&name) {
                    trace["checkpoint"] = json!(key);
                }
            }
            Err(e) => {
                trace["checkpoint_skip"] = json!(e.to_string());
                // A terminal definition can be restored from exact STEP. Never feed a
                // STEP-reimported checkpoint into later feature selections or booleans.
                if self.terminal.contains(id) {
                    if let Ok(neutral) = exchange::to_model(b) {
                        if neutral.bodies.len() == 1 && brep_is_manifold_solid(b).unwrap_or(false) {
                            if let Ok(step) = step_assembly::write_solids(&neutral) {
                                let name = format!("geom-{key}.step");
                                self.store(name.clone(), step.into_bytes());
                                if self.files.contains_key(&name) {
                                    trace["checkpoint_format"] = json!("step-terminal");
                                    trace["checkpoint"] = json!(key);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    pub(crate) fn mesh_name(&self, id: &str, chord: f64) -> Option<String> {
        Some(format!(
            "mesh-{}-{:016x}.json",
            self.keys.get(id)?,
            chord.to_bits()
        ))
    }
    pub(crate) fn read_mesh(&mut self, name: &str) -> Option<solidcore_tess::Mesh> {
        let bytes = self.read(name)?;
        let mesh = serde_json::from_slice(&bytes).ok()?;
        self.store(name.to_string(), bytes);
        Some(mesh)
    }
    pub(crate) fn save_mesh(&mut self, name: String, mesh: &solidcore_tess::Mesh) {
        if self.used < BUDGET && mesh.triangles.len() < 50_000 {
            if let Ok(data) = serde_json::to_vec(mesh) {
                self.store(name, data);
            }
        }
    }
}
// Later feature selections can depend on analytic surface/curve types as well as
// topology order. Reject any intermediate serialization that changes its exact exchange form.
fn signature(b: &Brep) -> Option<String> {
    let model = exchange::to_model(b).ok()?;
    let text = if brep_is_manifold_solid(b).ok()? {
        step_assembly::write_solids(&model).ok()?
    } else {
        step_assembly::write_surface(&model).ok()?
    };
    Some(format!("{:x}", Sha256::digest(text.as_bytes())))
}
fn io(e: impl std::fmt::Display) -> CadError {
    CadError::new("CHECKPOINT_IO", e.to_string())
}
fn collect_params(v: &Value, source: &BTreeMap<String, f64>, out: &mut BTreeMap<String, f64>) {
    match v {
        Value::Object(o) => {
            if let Some(name) = o.get("param").and_then(Value::as_str) {
                if let Some(val) = source.get(name) {
                    out.insert(name.to_owned(), *val);
                }
            }
            for x in o.values() {
                collect_params(x, source, out);
            }
        }
        Value::Array(a) => {
            for x in a {
                collect_params(x, source, out);
            }
        }
        _ => {}
    }
}
