//! Named instances and exact, explicitly requested assembly checks.
use crate::*;
use operations::V3;

fn zero() -> V3 {
    [Scalar::Number(0.), Scalar::Number(0.), Scalar::Number(0.)]
}
fn yes() -> bool {
    true
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Representation {
    #[default]
    Exact,
    Envelope {
        reason: String,
    },
    Simplified {
        reason: String,
        omitted: Vec<String>,
        affects: Vec<String>,
    },
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Instance {
    pub id: String,
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub rotation_order: placement::RotationOrder,
    #[serde(default)]
    pub poses: Option<placement::Poses>,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
    pub input: String,
    #[serde(default)]
    pub representation: Representation,
    #[serde(default = "zero")]
    pub translation: V3,
    #[serde(default = "zero")]
    pub rotation_deg: V3,
    /// Construction volumes can participate in checks without being exported.
    #[serde(default = "yes")]
    pub export: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AssemblyCheck {
    InterfaceCompatibility {
        a: query::compatibility::Attachment,
        b: query::compatibility::Attachment,
        plane_relation: query::compatibility::PlaneRelation,
        correspondence: query::compatibility::Correspondence,
        tolerance: query::contract::Tolerance,
        requirements: query::compatibility::Requirements,
    },
    GeometricRelationship {
        a: relationships::Target,
        b: relationships::Target,
        tolerance: query::contract::Tolerance,
        max_overlap_mm3: Scalar,
        #[serde(default)]
        min_clearance_mm: Option<Scalar>,
        #[serde(default)]
        allowed_contacts: Vec<relationships::ContactPair>,
        #[serde(default)]
        pose_samples: Vec<relationships::PoseSample>,
    },
    Collision {
        parts: Vec<String>,
        #[serde(default)]
        allowed_pairs: Vec<[String; 2]>,
        max_overlap_mm3: Scalar,
    },
    Opening {
        opening: String,
        target: String,
        max_distance_mm: Scalar,
        min_overlap_mm3: Scalar,
    },
    Containment {
        container: String,
        targets: Vec<String>,
        max_outside_mm3: Scalar,
    },
}

pub(crate) fn validate(m: &Model, features: &BTreeSet<String>) -> Result<()> {
    let group_poses = placement::groups(m)?;
    if m.assembly.is_empty() {
        if !m.checks.is_empty() || m.mates.is_some() {
            return Err(CadError::new(
                "INVALID_ASSEMBLY",
                "checks require an assembly",
            ));
        }
        return Ok(());
    }
    if !m.output.is_empty() {
        return Err(CadError::new(
            "INVALID_ASSEMBLY",
            "choose output or assembly, not both",
        ));
    }
    if m.assembly.len() > 128 || m.checks.len() > 64 {
        return Err(CadError::new(
            "RESOURCE_LIMIT",
            "at most 128 instances and 64 assembly checks",
        ));
    }
    if !m.assembly.iter().any(|v| v.export) {
        return Err(CadError::new(
            "INVALID_ASSEMBLY",
            "at least one exported instance required",
        ));
    }
    let mut ids = BTreeSet::new();
    for part in &m.assembly {
        if !valid_id(&part.id) || !ids.insert(part.id.clone()) {
            return Err(CadError::new("INVALID_INSTANCE_ID", &part.id));
        }
        if !features.contains(&part.input) {
            return Err(CadError::new("INVALID_REFERENCE", &part.input));
        }
        let text_ok = |s: &str| !s.trim().is_empty() && s.len() <= 800;
        let declared = match &part.representation {
            Representation::Exact => true,
            Representation::Envelope { reason } => text_ok(reason),
            Representation::Simplified {
                reason,
                omitted,
                affects,
            } => {
                text_ok(reason)
                    && !omitted.is_empty()
                    && !affects.is_empty()
                    && omitted.len() <= 32
                    && affects.len() <= 32
                    && omitted.iter().chain(affects).all(|s| text_ok(s))
            }
        };
        if !declared {
            return Err(CadError::new("INVALID_REPRESENTATION", &part.id));
        }
        placement::world(m, part, placement::PoseMode::Normal, &group_poses)?;
        if let Some(poses) = &part.poses {
            for p in [&poses.normal, &poses.internal, &poses.exploded] {
                p.resolve(&m.parameters)?;
            }
        }
        if serde_json::to_vec(&part.metadata)
            .map_err(|e| CadError::new("INVALID_METADATA", e.to_string()))?
            .len()
            > 32 * 1024
        {
            return Err(CadError::new(
                "RESOURCE_LIMIT",
                "instance metadata at most 32 KiB",
            ));
        }
        point(&part.translation, &m.parameters)?;
        point(&part.rotation_deg, &m.parameters)?;
    }
    let mut pairs = 0;
    for check in &m.checks {
        let (refs, threshold) = match check {
            AssemblyCheck::InterfaceCompatibility {
                a,
                b,
                correspondence,
                tolerance,
                requirements,
                ..
            } => {
                query::compatibility::validate(m, a, b, correspondence, *tolerance, requirements)?;
                pairs += 1;
                (
                    vec![&a.instance, &b.instance],
                    &requirements.max_plane_gap_mm,
                )
            }
            AssemblyCheck::GeometricRelationship {
                a,
                b,
                tolerance,
                max_overlap_mm3,
                min_clearance_mm,
                allowed_contacts,
                pose_samples,
            } => {
                relationships::validate(
                    m,
                    a,
                    b,
                    *tolerance,
                    max_overlap_mm3,
                    min_clearance_mm,
                    allowed_contacts,
                    pose_samples,
                )?;
                pairs += pose_samples.len().max(1);
                (vec![&a.instance, &b.instance], max_overlap_mm3)
            }
            AssemblyCheck::Collision {
                parts,
                allowed_pairs,
                max_overlap_mm3,
            } => {
                if parts.len() < 2 {
                    return Err(CadError::new(
                        "INVALID_ASSEMBLY",
                        "collision check needs at least two parts",
                    ));
                }
                let mut distinct = BTreeSet::new();
                for pair in allowed_pairs {
                    if pair[0] == pair[1] || !parts.contains(&pair[0]) || !parts.contains(&pair[1])
                    {
                        return Err(CadError::new(
                            "INVALID_ASSEMBLY",
                            "allowed pair must name distinct checked parts",
                        ));
                    }
                    let mut ordered = pair.clone();
                    ordered.sort();
                    if !distinct.insert(ordered) {
                        return Err(CadError::new(
                            "INVALID_ASSEMBLY",
                            "duplicate allowed collision pair",
                        ));
                    }
                }
                pairs += parts.len() * (parts.len() - 1) / 2;
                (parts.iter().collect::<Vec<_>>(), max_overlap_mm3)
            }
            AssemblyCheck::Opening {
                opening,
                target,
                max_distance_mm,
                min_overlap_mm3,
            } => {
                if min_overlap_mm3.resolve(&m.parameters)? < 0. {
                    return Err(CadError::new(
                        "INVALID_DIMENSION",
                        "overlap cannot be negative",
                    ));
                }
                pairs += 1;
                (vec![opening, target], max_distance_mm)
            }
            AssemblyCheck::Containment {
                container,
                targets,
                max_outside_mm3,
            } => {
                if targets.is_empty() || targets.contains(container) {
                    return Err(CadError::new(
                        "INVALID_ASSEMBLY",
                        "containment requires distinct target instances",
                    ));
                }
                pairs += targets.len();
                let mut r = targets.iter().collect::<Vec<_>>();
                r.push(container);
                (r, max_outside_mm3)
            }
        };
        if threshold.resolve(&m.parameters)? < 0. {
            return Err(CadError::new(
                "INVALID_DIMENSION",
                "check threshold cannot be negative",
            ));
        }
        let mut seen = BTreeSet::new();
        for id in refs {
            if !ids.contains(id) || !seen.insert(id) {
                return Err(CadError::new("INVALID_REFERENCE", id));
            }
        }
    }
    if pairs > 8128 {
        return Err(CadError::new(
            "RESOURCE_LIMIT",
            "at most 8128 checked pairs",
        ));
    }
    Ok(())
}
fn volume(k: &mut Kernel, b: &mut Brep) -> Result<f64> {
    if b.face_list().is_empty() {
        return Ok(0.);
    }
    measured_volume(k, b)
}
fn overlapping_boxes(k: &Kernel, a: &Brep, b: &Brep) -> Result<bool> {
    let a = brep_bounding_box(k, a, true)?;
    let b = brep_bounding_box(k, b, true)?;
    Ok((0..3).all(|i| a.max_at(i).min(b.max_at(i)) - a.min_at(i).max(b.min_at(i)) > 1e-8))
}

#[derive(Clone, Copy)]
struct MeasuredPart {
    bounds: [[f64; 3]; 2],
    volume: f64,
    solids: usize,
    faces: usize,
    edges: usize,
    warnings: usize,
}

impl MeasuredPart {
    fn inspect(k: &mut Kernel, b: &mut Brep, id: &str) -> Result<Self> {
        let check = check_brep(k, b, CheckLevel::Level1)?;
        if !check.is_valid() || !brep_is_manifold_solid(b)? {
            return Err(CadError::new(
                "GEOMETRY_VALIDATION_FAILED",
                format!("instance {}: {}", id, check.lines().join("; ")),
            ));
        }
        let v = volume(k, b)?;
        if !v.is_finite() || v <= 0. {
            return Err(CadError::new("INVALID_VOLUME", id));
        }
        let bb = brep_bounding_box(k, b, true)?;
        let (solids, _) = brep_material_census(b)?;
        Ok(Self {
            bounds: [
                std::array::from_fn(|i| bb.min_at(i)),
                std::array::from_fn(|i| bb.max_at(i)),
            ],
            volume: v,
            solids,
            faces: b.face_list().len(),
            edges: b.edge_list().len(),
            warnings: check.warnings().count(),
        })
    }

    fn translated(mut self, delta: Vec3) -> Option<Self> {
        for corner in &mut self.bounds {
            for (value, offset) in corner.iter_mut().zip([delta.x, delta.y, delta.z]) {
                // Retain the direct measurement path outside the ordinary model coordinate range.
                if value.abs() > MAX_COORD {
                    return None;
                }
                *value += offset;
                if !value.is_finite() || value.abs() > MAX_COORD {
                    return None;
                }
            }
        }
        Some(self)
    }
}

// A terminal definition used by exactly one occurrence has no consumers left
// after its local STEP and exact placed measurements are ready. Keep only those
// products; cross-part checks, mates, preview and query snapshots need live bodies.
#[derive(Default)]
pub(crate) struct EarlyParts {
    eligible: BTreeMap<String, (usize, placement::Rigid)>,
    ready: BTreeMap<String, (MeasuredPart, Option<String>)>,
    pub validation_ms: f64,
    pub export_ms: f64,
}

fn definition(body: &Brep) -> Result<String> {
    step_assembly::write_solids(
        &exchange::to_model(body).map_err(|e| CadError::new("EXPORT_FAILED", e.to_string()))?,
    )
}

impl EarlyParts {
    pub fn plan(m: &Model, uses: &BTreeMap<String, usize>, queryable: bool) -> Result<Self> {
        let mut plan = Self::default();
        if m.assembly.len() < 2
            || queryable
            || m.preview.is_some()
            || !m.checks.is_empty()
            || m.mates.is_some()
        {
            return Ok(plan);
        }
        let groups = placement::groups(m)?;
        for (idx, part) in m.assembly.iter().enumerate() {
            if uses.get(&part.input) == Some(&1) {
                let pose = placement::world(m, part, placement::PoseMode::Normal, &groups)?.0;
                plan.eligible.insert(part.input.clone(), (idx, pose));
            }
        }
        Ok(plan)
    }

    pub fn finish_feature(
        &mut self,
        k: &mut Kernel,
        m: &Model,
        id: &str,
        bodies: &mut BTreeMap<String, Brep>,
    ) -> Result<()> {
        let Some((idx, pose)) = self.eligible.remove(id) else {
            return Ok(());
        };
        let part = &m.assembly[idx];
        let mut body = bodies
            .remove(id)
            .ok_or_else(|| CadError::new("INVALID_REFERENCE", id))?;
        let t = Instant::now();
        let step = if part.export {
            Some(definition(&body)?)
        } else {
            None
        };
        self.export_ms += t.elapsed().as_secs_f64() * 1000.;
        let t = Instant::now();
        pose.apply(k, &mut body)?;
        let measured = MeasuredPart::inspect(k, &mut body, &part.id)?;
        self.validation_ms += t.elapsed().as_secs_f64() * 1000.;
        self.ready.insert(id.to_owned(), (measured, step));
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn finish(
    k: &mut Kernel,
    m: &Model,
    bodies: &mut BTreeMap<String, Brep>,
    uses: &mut BTreeMap<String, usize>,
    trace: Vec<Value>,
    peak: usize,
    mut early: EarlyParts,
    start: Instant,
    geometry_ms: f64,
    cache: &mut incremental::Checkpoints,
    datums: &query::datums::Definitions,
    interfaces: &query::interfaces::Definitions,
) -> Result<Build> {
    let tick = Instant::now();
    let group_poses = placement::groups(m)?;
    let initial = m
        .assembly
        .iter()
        .map(|i| {
            Ok((
                i.id.clone(),
                placement::world(m, i, placement::PoseMode::Normal, &group_poses)?.0,
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let (solved, mate_report) = mates::solve(m, initial, datums, interfaces)?;
    let mut definitions = BTreeMap::new();
    // Retain one local-space definition for every displayed source.
    if m.preview.is_some() {
        for id in m
            .assembly
            .iter()
            .filter(|i| i.export)
            .map(|i| &i.input)
            .collect::<BTreeSet<_>>()
        {
            *uses.entry(id.clone()).or_default() += 1;
        }
    }
    let mut parts = BTreeMap::new();
    let mut measurements = BTreeMap::new();
    let mut reused = 0usize;
    let mut instances = vec![];
    let mut total_volume = 0.;
    let mut total_faces = 0;
    let mut total_edges = 0;
    let mut total_solids = 0;
    let mut warnings = 0;
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    let early_count = early.ready.len();
    let p = &m.parameters;
    for instance in &m.assembly {
        let source = instance.input.as_str();
        let pose = solved[&instance.id];
        let prepared = early.ready.remove(source);
        if prepared.is_none() && instance.export && !definitions.contains_key(source) {
            let body = bodies
                .get(source)
                .ok_or_else(|| CadError::new("INVALID_REFERENCE", source))?;
            definitions.insert(instance.input.clone(), definition(body)?);
        }

        // Translation preserves topology, mass and tight axis-aligned bounds. A different
        // linear transform (including any different rotation) gets its own measured shape.
        // Explicit spatial checks always receive fully placed and independently checked bodies.
        let key = (
            source,
            pose.axes
                .map(|axis| [axis.x, axis.y, axis.z].map(f64::to_bits)),
        );
        let can_reuse = m.checks.is_empty();
        let previous = measurements
            .get(&key)
            .and_then(|&(origin, part): &(Vec3, MeasuredPart)| {
                part.translated(pose.origin - origin)
            });
        let measured = if let Some((measured, local)) = prepared {
            crate::executor::last_body_use(source, uses)?;
            if let Some(text) = local {
                definitions.insert(source.to_owned(), text);
            }
            measured
        } else if let Some(part) = previous {
            if crate::executor::last_body_use(source, uses)? {
                bodies.remove(source);
            }
            reused += 1;
            part
        } else {
            let mut body = take_body(k, source, bodies, uses)?;
            pose.apply(k, &mut body)?;
            let part = MeasuredPart::inspect(k, &mut body, &instance.id)?;
            if can_reuse {
                measurements.insert(key, (pose.origin, part));
            } else {
                parts.insert(instance.id.clone(), body);
            }
            part
        };
        warnings += measured.warnings;
        let translation = point(&instance.translation, p)?;
        let rotation = point(&instance.rotation_deg, p)?;
        instances.push(json!({
            "id": instance.id, "source_feature": source, "representation": instance.representation,
            "parent": instance.parent, "poses": instance.poses, "metadata": instance.metadata,
            "world_placement": pose.json(), "exported": instance.export,
            "volume_mm3": measured.volume, "solids": measured.solids, "bbox_mm": measured.bounds,
            "translation_mm": [translation.x, translation.y, translation.z],
            "rotation_deg": [rotation.x, rotation.y, rotation.z],
        }));
        if instance.export {
            total_volume += measured.volume;
            total_solids += measured.solids;
            total_faces += measured.faces;
            total_edges += measured.edges;
            for i in 0..3 {
                lo[i] = lo[i].min(measured.bounds[0][i]);
                hi[i] = hi[i].max(measured.bounds[1][i]);
            }
        }
    }
    if total_faces > 20_000 {
        return Err(CadError::new(
            "RESOURCE_LIMIT",
            "assembly exceeds face budget",
        ));
    }
    let mut findings = vec![];
    for check in &m.checks {
        match check {
            AssemblyCheck::InterfaceCompatibility {
                a,
                b,
                plane_relation,
                correspondence,
                tolerance,
                requirements,
            } => {
                for target in [a, b] {
                    let i = m.assembly.iter().find(|i| i.id == target.instance).unwrap();
                    if !matches!(i.representation, Representation::Exact) {
                        return Err(CadError::new(
                            "QUERY_UNCERTAIN",
                            "fit cannot be certified from reduced geometry",
                        ));
                    }
                    relationships::require_exact(m, &i.input, &trace)?;
                }
                findings.push(query::compatibility::check(
                    m,
                    a,
                    b,
                    *plane_relation,
                    correspondence,
                    *tolerance,
                    requirements,
                    interfaces,
                    &solved,
                )?);
            }
            AssemblyCheck::GeometricRelationship {
                a,
                b,
                tolerance,
                max_overlap_mm3,
                min_clearance_mm,
                allowed_contacts,
                pose_samples,
            } => {
                findings.push(relationships::check(
                    m,
                    k,
                    &parts,
                    &trace,
                    a,
                    b,
                    *tolerance,
                    max_overlap_mm3,
                    min_clearance_mm,
                    allowed_contacts,
                    pose_samples,
                )?);
            }
            AssemblyCheck::Collision {
                parts: ids,
                allowed_pairs,
                max_overlap_mm3,
            } => {
                let threshold = max_overlap_mm3.resolve(&m.parameters)?;
                for i in 0..ids.len() {
                    for j in i + 1..ids.len() {
                        let (a, b) = (&ids[i], &ids[j]);
                        let allowed = allowed_pairs
                            .iter()
                            .any(|p| (&p[0] == a && &p[1] == b) || (&p[0] == b && &p[1] == a));
                        let va = &parts[a];
                        let vb = &parts[b];
                        let overlap = if overlapping_boxes(k, va, vb)? {
                            let ca = solidcore::topology::brep::p3::brep_copy(k, va)?;
                            let cb = solidcore::topology::brep::p3::brep_copy(k, vb)?;
                            let mut common = boolean_intersection(k, ca, cb)?;
                            volume(k, &mut common)?
                        } else {
                            0.
                        };
                        findings.push(json!({"kind":"collision","parts":[a,b],"overlap_mm3":overlap,"allowed":allowed,"threshold_mm3":threshold}));
                        if !allowed && overlap > threshold + 1e-6 {
                            return Err(CadError::new(
                                "ASSEMBLY_COLLISION",
                                format!("{a} and {b} overlap {overlap} mm3 (limit {threshold})"),
                            ));
                        }
                    }
                }
            }
            AssemblyCheck::Opening {
                opening,
                target,
                max_distance_mm,
                min_overlap_mm3,
            } => {
                let va = &parts[opening];
                let vb = &parts[target];
                let overlap = if overlapping_boxes(k, va, vb)? {
                    let ca = solidcore::topology::brep::p3::brep_copy(k, va)?;
                    let cb = solidcore::topology::brep::p3::brep_copy(k, vb)?;
                    let mut common = boolean_intersection(k, ca, cb)?;
                    volume(k, &mut common)?
                } else {
                    0.
                };
                let distance = if overlap > 1e-8 {
                    0.
                } else {
                    crate::distance::boundary_distance(k, va, vb)?
                };
                if !distance.is_finite() || distance < 0. {
                    return Err(CadError::new(
                        "ASSEMBLY_OPENING",
                        "distance solver returned invalid result",
                    ));
                }
                let max = max_distance_mm.resolve(&m.parameters)?;
                let min = min_overlap_mm3.resolve(&m.parameters)?;
                findings.push(json!({"kind":"opening","opening":opening,"target":target,"distance_mm":distance,"overlap_mm3":overlap,"max_distance_mm":max,"min_overlap_mm3":min}));
                if distance > max + 1e-6 || overlap + 1e-6 < min {
                    return Err(CadError::new(
                        "ASSEMBLY_OPENING",
                        format!(
                            "{opening} / {target}: distance {distance} mm, overlap {overlap} mm3"
                        ),
                    ));
                }
            }
            AssemblyCheck::Containment {
                container,
                targets,
                max_outside_mm3,
            } => {
                let threshold = max_outside_mm3.resolve(&m.parameters)?;
                for target in targets {
                    let a = solidcore::topology::brep::p3::brep_copy(k, &parts[target])?;
                    let b = solidcore::topology::brep::p3::brep_copy(k, &parts[container])?;
                    let mut outside = boolean_difference(k, a, b)?;
                    let outside = volume(k, &mut outside)?;
                    findings.push(json!({"kind":"containment","container":container,"target":target,"outside_mm3":outside,"threshold_mm3":threshold}));
                    if outside > threshold + 1e-6 {
                        return Err(CadError::new(
                            "ASSEMBLY_CONTAINMENT",
                            format!("{target} extends {outside} mm3 outside {container}"),
                        ));
                    }
                }
            }
        }
    }
    let validation_ms = early.validation_ms + tick.elapsed().as_secs_f64() * 1000.;
    let tick = Instant::now();
    let step = crate::step_assembly::write(m, definitions, &solved)?;
    let export_ms = early.export_ms + tick.elapsed().as_secs_f64() * 1000.;
    let mut display = Vec::new();
    if let Some(options) = &m.preview {
        for i in m.assembly.iter().filter(|i| i.export) {
            let (pose, visible) = placement::world(m, i, options.pose, &group_poses)?;
            if visible {
                let pose = if matches!(options.pose, placement::PoseMode::Normal) {
                    solved[&i.id]
                } else {
                    pose
                };
                display.push(preview::InstanceMesh {
                    id: i.id.clone(),
                    definition: &i.input,
                    body: &bodies[&i.input],
                    pose,
                });
            }
        }
        display.sort_by(|a, b| a.id.cmp(&b.id));
    }
    let (artifacts, preview_report) = preview::finish(k, m, display, cache)?;
    Ok(Build {
        artifacts,
        report: json!({"preview":preview_report,"model_hash":model_hash(m),"executor_version":EXECUTOR_VERSION,"execution_policy":EXECUTION_POLICY,"kernel":kernel_policy(),"validation":{"level":"kernel_level_1_per_instance","measured_instances":m.assembly.len()-reused,"translated_instance_reuses":reused,"valid":true,"manifold":true,"warnings":warnings},"geometry":{"volume_mm3":total_volume,"volume_semantics":"sum_of_exported_instances","bbox_mm":[lo,hi],"faces":total_faces,"edges":total_edges,"solids":total_solids},"assembly":{"instances":instances,"checks":findings,"step_representation":"native_product_instance_tree","native_step_instance_tree":true,"groups":m.groups,"mate_solution":mate_report},"features":trace,"peak_live_feature_bodies":peak,"early_released_definitions":early_count,"timing_ms":{"geometry":geometry_ms,"validation":validation_ms,"export":export_ms,"total":start.elapsed().as_secs_f64()*1000.},"step_bytes":step.len(),"editability":{"source":"model.json","parameter_replay":true,"native_commercial_feature_tree":false}}),
        step,
    })
}
