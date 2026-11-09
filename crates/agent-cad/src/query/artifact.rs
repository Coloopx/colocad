//! Immutable exact query artifacts. Geometry is stored separately from the manifest.
use super::snapshot::{Binding, DefinitionSnapshot, LoadedDefinition};
use crate::*;
use std::{
    fs,
    io::Read,
    path::{Component, Path},
};

pub const MANIFEST: &str = "query-manifest.json";
pub const MAX_GEOMETRY: usize = 64 * 1024 * 1024;
fn digest<T: Serialize>(v: &T) -> Result<String> {
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(
            serde_json::to_vec(v).map_err(|e| CadError::new("QUERY_ARGUMENT", e.to_string()))?
        )
    ))
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub path: String,
    pub schema: String,
    pub kernel_revision: String,
    pub geometry_sha256: String,
    pub definition_sha256: String,
    pub bindings: BTreeMap<u64, Binding>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Occurrence {
    pub instance_path: Vec<String>,
    pub definition_id: String,
    pub origin_mm: [f64; 3],
    pub axes: [[f64; 3]; 3],
    pub representation: Value,
}
impl Occurrence {
    pub(crate) fn placement(&self) -> Result<placement::Rigid> {
        let axes = self.axes.map(|v| Vec3::new(v[0], v[1], v[2]));
        if self
            .origin_mm
            .iter()
            .chain(self.axes.iter().flatten())
            .any(|n| !n.is_finite())
            || axes.iter().any(|v| (v.length() - 1.).abs() > 1e-7)
            || (axes[0].cross(axes[1]) - axes[2]).length() > 1e-7
        {
            return Err(CadError::new(
                "ARTIFACT_INTEGRITY",
                "occurrence is not a proper rigid transform",
            ));
        }
        Ok(placement::Rigid {
            origin: Vec3::new(self.origin_mm[0], self.origin_mm[1], self.origin_mm[2]),
            axes,
        })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestBody {
    pub schema: String,
    pub executor_version: String,
    pub kernel_revision: String,
    pub model_sha256: String,
    pub definitions: BTreeMap<String, Definition>,
    pub occurrences: BTreeMap<String, Occurrence>,
    pub degradations: Vec<Value>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub datums: super::datums::Definitions,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub interfaces: super::interfaces::Definitions,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub query_revision_id: String,
    pub body: ManifestBody,
}
impl Manifest {
    pub fn revision(body: &ManifestBody) -> Result<String> {
        digest(body)
    }
    pub fn open(root: &Path, expected: &str) -> Result<Self> {
        if !root.join(MANIFEST).exists() {
            return Err(CadError::new(
                "QUERY_SNAPSHOT_REQUIRED",
                "artifact has no exact query snapshot; explicitly rebuild with queryable enabled",
            ));
        }
        let data = read_payload(root, MANIFEST, 8 * 1024 * 1024)?;
        let manifest: Self = serde_json::from_slice(&data)
            .map_err(|e| CadError::new("ARTIFACT_INTEGRITY", e.to_string()))?;
        if manifest.body.schema != "colocad.query-manifest/1"
            || manifest.body.kernel_revision != KERNEL_REV
            || manifest.body.executor_version != EXECUTOR_VERSION
        {
            return Err(CadError::new(
                "QUERY_SNAPSHOT_INCOMPATIBLE",
                "query serializer/kernel/executor version mismatch",
            ));
        }
        if Self::revision(&manifest.body)? != manifest.query_revision_id {
            return Err(CadError::new(
                "ARTIFACT_INTEGRITY",
                "query manifest digest mismatch",
            ));
        }
        if manifest.query_revision_id != expected {
            return Err(CadError::new(
                "QUERY_REVISION_MISMATCH",
                "expected query revision differs from artifact",
            ));
        }
        if manifest.body.definitions.is_empty()
            || manifest.body.definitions.len() > 128
            || manifest.body.occurrences.len() > 128
        {
            return Err(CadError::new(
                "QUERY_LIMIT",
                "definition/occurrence budget exceeded",
            ));
        }
        for occurrence in manifest.body.occurrences.values() {
            occurrence.placement()?;
            if !manifest
                .body
                .definitions
                .contains_key(&occurrence.definition_id)
            {
                return Err(CadError::new(
                    "ARTIFACT_INTEGRITY",
                    "occurrence references a missing definition",
                ));
            }
        }
        // Inspect the full payload inventory without loading unrelated definitions.
        let mut size = 0_u64;
        for definition in manifest.body.definitions.values() {
            size = size
                .checked_add(
                    payload_path(root, &definition.path)?
                        .metadata()
                        .map_err(io_error)?
                        .len(),
                )
                .ok_or_else(|| CadError::new("QUERY_LIMIT", "snapshot size overflow"))?;
        }
        if size > MAX_GEOMETRY as u64 {
            return Err(CadError::new(
                "QUERY_LIMIT",
                "total query geometry exceeds 64 MiB",
            ));
        }
        Ok(manifest)
    }
    pub fn load(&self, root: &Path, id: &str, k: &mut Kernel) -> Result<LoadedDefinition> {
        let d =
            self.body.definitions.get(id).ok_or_else(|| {
                CadError::new("TOPOLOGY_REF_INVALID", "definition not in snapshot")
            })?;
        let s = DefinitionSnapshot {
            schema: d.schema.clone(),
            kernel_revision: d.kernel_revision.clone(),
            geometry_sha256: d.geometry_sha256.clone(),
            bindings: d.bindings.clone(),
            geometry: read_payload(root, &d.path, MAX_GEOMETRY)?,
        };
        s.load(k, &d.definition_sha256)
    }
}
fn io_error(e: std::io::Error) -> CadError {
    CadError::new("ARTIFACT_INTEGRITY", e.to_string())
}
fn payload_path(root: &Path, name: &str) -> Result<std::path::PathBuf> {
    let path = Path::new(name);
    if path.components().count() != 1
        || !matches!(path.components().next(), Some(Component::Normal(_)))
    {
        return Err(CadError::new(
            "ARTIFACT_INTEGRITY",
            "query payload path must be a local filename",
        ));
    }
    let root = fs::canonicalize(root).map_err(io_error)?;
    let path = root.join(path);
    let metadata = fs::symlink_metadata(&path).map_err(io_error)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || !fs::canonicalize(&path)
            .map_err(io_error)?
            .starts_with(&root)
    {
        return Err(CadError::new(
            "ARTIFACT_INTEGRITY",
            "query payload cannot be a symlink or escape the artifact root",
        ));
    }
    Ok(path)
}
fn read_payload(root: &Path, name: &str, limit: usize) -> Result<Vec<u8>> {
    let path = payload_path(root, name)?;
    let file = fs::File::open(path).map_err(io_error)?;
    if file.metadata().map_err(io_error)?.len() > limit as u64 {
        return Err(CadError::new("QUERY_LIMIT", "query payload exceeds limit"));
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() > limit {
        return Err(CadError::new(
            "QUERY_LIMIT",
            "query payload grew beyond limit",
        ));
    }
    Ok(bytes)
}
pub(crate) struct Capture {
    definitions: BTreeMap<String, Definition>,
    files: BTreeMap<String, Vec<u8>>,
    pub(crate) datums: super::datums::Definitions,
    pub(crate) interfaces: super::interfaces::Definitions,
    display: BTreeMap<String, Value>,
}
impl Capture {
    pub(crate) fn new(k: &Kernel, m: &Model, bodies: &BTreeMap<String, Brep>) -> Result<Self> {
        let ids = if m.assembly.is_empty() {
            BTreeSet::from([m.output.clone()])
        } else {
            m.assembly.iter().map(|i| i.input.clone()).collect()
        };
        let mut definitions = BTreeMap::new();
        let mut files = BTreeMap::new();
        let mut bytes = 0;
        let mut display = BTreeMap::new();
        let mut display_budget = 200_000;
        for (index, id) in ids.into_iter().enumerate() {
            let b = bodies
                .get(&id)
                .ok_or_else(|| CadError::new("QUERY_ARGUMENT", "final definition missing"))?;
            let s = DefinitionSnapshot::capture(k, b)?;
            bytes += s.geometry.len();
            if bytes > MAX_GEOMETRY {
                return Err(CadError::new(
                    "QUERY_LIMIT",
                    "total query geometry exceeds 64 MiB",
                ));
            }
            if let Some(preview) = &m.preview {
                let mut kernel = Kernel::default();
                let loaded = s.load(&mut kernel, &s.digest())?;
                display.insert(
                    id.clone(),
                    super::display::definition(
                        &kernel,
                        &loaded,
                        preview.chord_mm,
                        &mut display_budget,
                    )?,
                );
            }
            let path = format!("query-definition-{index}.smb");
            let d = Definition {
                path: path.clone(),
                definition_sha256: s.digest(),
                schema: s.schema,
                kernel_revision: s.kernel_revision,
                geometry_sha256: s.geometry_sha256,
                bindings: s.bindings,
            };
            files.insert(path, s.geometry);
            definitions.insert(id, d);
        }
        Ok(Self {
            definitions,
            files,
            display,
            datums: BTreeMap::new(),
            interfaces: BTreeMap::new(),
        })
    }
    pub(crate) fn resolve_datums(&mut self, m: &Model) -> Result<()> {
        for (id, d) in &self.definitions {
            let declarations = m
                .datums
                .iter()
                .filter(|d| &d.definition == id)
                .collect::<Vec<_>>();
            let interfaces = m
                .interfaces
                .iter()
                .filter(|d| &d.definition == id)
                .collect::<Vec<_>>();
            if declarations.is_empty() && interfaces.is_empty() {
                continue;
            }
            let body = ManifestBody {
                schema: "colocad.query-manifest/1".into(),
                executor_version: EXECUTOR_VERSION.into(),
                kernel_revision: KERNEL_REV.into(),
                model_sha256: model_hash(m),
                definitions: self.definitions.clone(),
                occurrences: BTreeMap::new(),
                degradations: vec![],
                datums: BTreeMap::new(),
                interfaces: BTreeMap::new(),
            };
            let manifest = Manifest {
                query_revision_id: Manifest::revision(&body)?,
                body,
            };
            let mut kernel = Kernel::default();
            kernel.fixes.precise_properties_body_length_scale = true;
            kernel.fixes.precise_properties_split_at_surface_knots = true;
            kernel.fixes.precise_properties_span_end_from_below = true;
            kernel.fixes.precise_properties_trim_base_v = true;
            kernel.fixes.integrator_propagates_failure = true;
            let snapshot = DefinitionSnapshot {
                schema: d.schema.clone(),
                kernel_revision: d.kernel_revision.clone(),
                geometry_sha256: d.geometry_sha256.clone(),
                bindings: d.bindings.clone(),
                geometry: self.files[&d.path].clone(),
            };
            let loaded = snapshot.load(&mut kernel, &d.definition_sha256)?;
            let solid = brep_is_manifold_solid(&loaded.body)?;
            let reverse = loaded.entities.iter().map(|(t, e)| (*e, *t)).collect();
            let mut c = super::engine::Context {
                kernel,
                loaded,
                manifest,
                definition: id.clone(),
                occurrence: None,
                pose: Default::default(),
                tolerance: declarations
                    .first()
                    .map(|d| d.tolerance)
                    .unwrap_or_else(|| interfaces[0].requirements.verification_tolerance),
                cache: BTreeMap::new(),
                visited: 0,
                reverse,
                solid,
                probes: 0,
                hypotheses: 0,
                holes: None,
                incidents: None,
            };
            let mut resolved = BTreeMap::new();
            for declaration in declarations {
                c.manifest.body.datums.insert(id.clone(), resolved.clone());
                let datum = super::datums::resolve(&mut c, declaration, &resolved, &m.parameters)
                    .map_err(|mut e| {
                    e.details = Some(json!({"definition_id":id,"datum_id":declaration.id}));
                    e
                })?;
                resolved.insert(declaration.id.clone(), datum);
            }
            c.manifest.body.datums.insert(id.clone(), resolved.clone());
            self.datums.insert(id.clone(), resolved);
            let mut resolved = BTreeMap::new();
            for declaration in interfaces {
                let interface = super::interfaces::resolve(&mut c, declaration, &m.parameters)
                    .map_err(|mut e| {
                        e.details = Some(json!({"definition_id":id,"interface_id":declaration.id}));
                        e
                    })?;
                resolved.insert(declaration.id.clone(), interface);
            }
            if !resolved.is_empty() {
                self.interfaces.insert(id.clone(), resolved);
            }
        }
        Ok(())
    }
    pub(crate) fn finish(mut self, m: &Model, result: &mut Build) -> Result<()> {
        let mut occurrences = BTreeMap::new();
        if let Some(rows) = result.report["assembly"]["instances"].as_array() {
            for row in rows {
                let id = row["id"]
                    .as_str()
                    .ok_or_else(|| CadError::new("QUERY_ARGUMENT", "instance ID missing"))?;
                let instance = m.assembly.iter().find(|i| i.id == id).unwrap();
                let mut path = vec![id.to_string()];
                let mut parent = instance.parent.clone();
                while let Some(id) = parent {
                    path.push(id.clone());
                    parent = m
                        .groups
                        .iter()
                        .find(|g| g.id == id)
                        .and_then(|g| g.parent.clone());
                    if path.len() > 513 {
                        return Err(CadError::new("QUERY_LIMIT", "instance path too deep"));
                    }
                }
                path.reverse();
                let o = Occurrence {
                    instance_path: path,
                    definition_id: instance.input.clone(),
                    origin_mm: serde_json::from_value(row["world_placement"]["origin_mm"].clone())
                        .map_err(|e| CadError::new("QUERY_ARGUMENT", e.to_string()))?,
                    axes: serde_json::from_value(row["world_placement"]["axes"].clone())
                        .map_err(|e| CadError::new("QUERY_ARGUMENT", e.to_string()))?,
                    representation: row["representation"].clone(),
                };
                o.placement()?;
                occurrences.insert(id.into(), o);
            }
        }
        let degradations = result.report["features"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|r| !r["degradation"].is_null())
            .map(|r| json!({"feature_id":r["id"],"degradation":r["degradation"]}))
            .collect();
        let body = ManifestBody {
            schema: "colocad.query-manifest/1".into(),
            executor_version: EXECUTOR_VERSION.into(),
            kernel_revision: KERNEL_REV.into(),
            model_sha256: model_hash(m),
            definitions: self.definitions,
            occurrences,
            degradations,
            datums: self.datums,
            interfaces: self.interfaces,
        };
        let query_revision_id = Manifest::revision(&body)?;
        let manifest = Manifest {
            query_revision_id: query_revision_id.clone(),
            body,
        };
        if !manifest.body.datums.is_empty() {
            let mut observations = BTreeMap::new();
            for (definition, datums) in &manifest.body.datums {
                let values = datums
                    .iter()
                    .map(|(id, d)| {
                        Ok((
                            id.clone(),
                            d.observation(id, definition, &manifest, Default::default(), None)?,
                        ))
                    })
                    .collect::<Result<BTreeMap<_, _>>>()?;
                observations.insert(definition.clone(), values);
            }
            result.report["datums"] = json!(observations);
        }
        if !manifest.body.interfaces.is_empty() {
            let mut observations = BTreeMap::new();
            for (definition, interfaces) in &manifest.body.interfaces {
                let values = interfaces
                    .iter()
                    .map(|(id, d)| {
                        Ok((
                            id.clone(),
                            d.observation(definition, &manifest, Default::default(), None)?,
                        ))
                    })
                    .collect::<Result<BTreeMap<_, _>>>()?;
                observations.insert(definition.clone(), values);
            }
            result.report["interfaces"] = json!(observations);
        }
        if let Some(checks) = result.report["assembly"]["checks"].as_array_mut() {
            for check in checks
                .iter_mut()
                .filter(|c| c["kind"] == "interface_compatibility")
            {
                check["query_revision_id"] = json!(manifest.query_revision_id);
                for side in ["a", "b"] {
                    let instance = check[side]["instance"].as_str().unwrap();
                    let name = check[side]["interface"].as_str().unwrap();
                    let o = &manifest.body.occurrences[instance];
                    let d = &manifest.body.interfaces[&o.definition_id][name];
                    let observation = d.observation(
                        &o.definition_id,
                        &manifest,
                        o.placement()?,
                        Some(o.instance_path.clone()),
                    )?;
                    check[format!("{side}_interface_evidence")] = observation;
                }
            }
        }
        if !self.display.is_empty() {
            let display = json!({"schema":"colocad.query-display/1","query_revision_id":manifest.query_revision_id,"definitions":self.display,"definition_digests":manifest.body.definitions.iter().map(|(id,d)|(id,&d.definition_sha256)).collect::<BTreeMap<_,_>>(),"occurrences":manifest.body.occurrences,"space":"definition","display_only":true,"approximation":"adaptive_midpoint_curve_samples_not_measurement_geometry","chord_mm":m.preview.as_ref().unwrap().chord_mm});
            let bytes = serde_json::to_vec(&display)
                .map_err(|e| CadError::new("QUERY_ARGUMENT", e.to_string()))?;
            if bytes.len() > 16 * 1024 * 1024 {
                return Err(CadError::new("QUERY_LIMIT", "query display exceeds 16 MiB"));
            }
            self.files.insert("query-display.json".into(), bytes);
        }
        let data = serde_json::to_vec(&manifest)
            .map_err(|e| CadError::new("QUERY_ARGUMENT", e.to_string()))?;
        if data.len() > 8 * 1024 * 1024 {
            return Err(CadError::new("QUERY_LIMIT", "query manifest exceeds 8 MiB"));
        }
        self.files.insert(MANIFEST.into(), data);
        result.artifacts.append(&mut self.files);
        result.report["query_revision_id"] = json!(query_revision_id);
        result.report["query_manifest"] = json!(MANIFEST);
        Ok(())
    }
}
