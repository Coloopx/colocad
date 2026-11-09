//! Deterministic feature execution, exact validation and exports.
use crate::*;
pub struct Build {
    pub artifacts: BTreeMap<String, Vec<u8>>,
    pub report: Value,
    pub step: String,
}

// A closed-form oracle is available only for separated, fully interior through holes
// in rectangular stock. Other targets retain the per-cutter material-removal check.
fn grid_stock_volume(model: &Model, feature: &Feature) -> Result<Option<f64>> {
    let Feature::HoleGrid {
        input,
        origin,
        radius,
        depth,
        count,
        spacing,
        ..
    } = feature
    else {
        return Ok(None);
    };
    let Some(Feature::Box {
        origin: stock_origin,
        size,
        ..
    }) = model.features.iter().find(|f| f.id() == input)
    else {
        return Ok(None);
    };
    let p = &model.parameters;
    let base = point(stock_origin, p)?;
    let stock = point(size, p)?;
    let at = point(origin, p)?;
    let radius = radius.resolve(p)?;
    let [nx, ny] = grid_counts(count, p)?;
    if at.z >= base.z - 1e-4 || at.z + depth.resolve(p)? <= base.z + stock.z + 1e-4 {
        return Ok(None);
    }
    for i in 0..2 {
        let start = [at.x, at.y][i];
        let end = start + ([nx, ny][i] - 1) as f64 * spacing[i].resolve(p)?;
        let lower = [base.x, base.y][i];
        let upper = lower + [stock.x, stock.y][i];
        if start.min(end) - radius <= lower + 1e-4 || start.max(end) + radius >= upper - 1e-4 {
            return Ok(None);
        }
    }
    Ok(Some(
        stock.x * stock.y * stock.z
            - (nx * ny) as f64 * std::f64::consts::PI * radius * radius * stock.z,
    ))
}

pub(crate) fn last_body_use(id: &str, uses: &mut BTreeMap<String, usize>) -> Result<bool> {
    let count = uses
        .get_mut(id)
        .ok_or_else(|| CadError::new("INVALID_REFERENCE", id))?;
    *count = count
        .checked_sub(1)
        .ok_or_else(|| CadError::new("INVALID_REFERENCE", id))?;
    Ok(*count == 0)
}

pub(crate) fn take_body(
    k: &mut Kernel,
    id: &str,
    bodies: &mut BTreeMap<String, Brep>,
    uses: &mut BTreeMap<String, usize>,
) -> Result<Brep> {
    if last_body_use(id, uses)? {
        bodies
            .remove(id)
            .ok_or_else(|| CadError::new("INVALID_REFERENCE", id))
    } else {
        let b = bodies
            .get(id)
            .ok_or_else(|| CadError::new("INVALID_REFERENCE", id))?;
        Ok(solidcore::topology::brep::p3::brep_copy(k, b)?)
    }
}

// Keep numerical integration fixes scoped to measurements. Geometry construction
// retains its independently tested policy; a failed integral is never replaced by zero.
pub(crate) fn measured_volume(k: &mut Kernel, b: &mut Brep) -> Result<f64> {
    let previous = k.fixes;
    k.fixes.precise_properties_body_length_scale = true;
    k.fixes.precise_properties_split_at_surface_knots = true;
    k.fixes.precise_properties_span_end_from_below = true;
    k.fixes.precise_properties_trim_base_v = true;
    k.fixes.integrator_propagates_failure = true;
    let result = brep_volume_default(k, b, 1e-8);
    k.fixes = previous;
    Ok(result?)
}

pub fn build(model: &Model) -> Result<Build> {
    build_revision(model, &mut incremental::Checkpoints::default())
}
pub fn build_revision(model: &Model, cache: &mut incremental::Checkpoints) -> Result<Build> {
    build_revision_with_options(model, cache, None)
}
pub fn build_revision_with_options(
    model: &Model,
    cache: &mut incremental::Checkpoints,
    queryable: Option<bool>,
) -> Result<Build> {
    if queryable == Some(false) && (!model.datums.is_empty() || !model.interfaces.is_empty()) {
        return Err(CadError::new(
            "QUERY_SNAPSHOT_REQUIRED",
            "datum/interface declarations require queryable artifacts",
        ));
    }
    let queryable = queryable.unwrap_or(!model.datums.is_empty() || !model.interfaces.is_empty());
    let mut result = execute(model, cache, queryable)?;
    result.artifacts.append(&mut cache.files);
    Ok(result)
}
fn execute(model: &Model, cache: &mut incremental::Checkpoints, queryable: bool) -> Result<Build> {
    let start = Instant::now();
    validate(model)?;
    cache.prepare(model);
    let mut k = Kernel::default();
    k.fixes.transform_scales_each_tolerance = true;
    k.fixes.transform_model_size_from_body_box = true;
    k.fixes.fillet_chamfer_cone_on_arc = true;
    k.fixes.shell_edges_on_analytics_exact = true;
    let mut bodies = BTreeMap::new();
    let mut uses = BTreeMap::<String, usize>::new();
    for f in &model.features {
        for id in f.inputs() {
            *uses.entry(id.into()).or_default() += 1;
        }
    }
    if model.assembly.is_empty() {
        *uses.entry(model.output.clone()).or_default() += 1;
    } else {
        for part in &model.assembly {
            *uses.entry(part.input.clone()).or_default() += 1;
        }
    }
    let mut trace = vec![];
    let required_regions = regions::required(model);
    let mut region_map = BTreeMap::new();
    let mut peak_live_bodies = 0;
    for f in &model.features {
        let tick = Instant::now();
        if !required_regions.contains(f.id()) {
            if let Some((body, mut cached_trace)) = cache.restore(&mut k, f.id()) {
                for parent in f.inputs() {
                    let remaining = uses.get_mut(parent).expect("validated feature reference");
                    *remaining -= 1;
                    if *remaining == 0 {
                        bodies.remove(parent);
                        region_map.remove(parent);
                    }
                }
                cached_trace["cache_hit"] = json!(true);
                cached_trace["elapsed_ms"] = json!(tick.elapsed().as_secs_f64() * 1000.);
                trace.push(cached_trace);
                bodies.insert(f.id().to_owned(), body);
                peak_live_bodies = peak_live_bodies.max(bodies.len());
                continue;
            }
        }
        let failure_policy = match f {
            Feature::Fillet {
                input,
                on_fail:
                    OnFailure::Record {
                        reason,
                        omitted,
                        affects,
                    },
                ..
            }
            | Feature::Chamfer {
                input,
                on_fail:
                    OnFailure::Record {
                        reason,
                        omitted,
                        affects,
                    },
                ..
            } => {
                if reason.trim().is_empty()
                    || reason.len() > 800
                    || omitted.is_empty()
                    || affects.is_empty()
                {
                    return Err(CadError::new(
                        "INVALID_DEGRADATION",
                        "record policy requires reason, omitted items and affected requirements",
                    ));
                }
                Some((
                    solidcore::topology::brep::p3::brep_copy(&mut k, &bodies[input])?,
                    json!({"reason":reason,"omitted":omitted,"affects":affects}),
                ))
            }
            _ => None,
        };
        let pending_regions = if required_regions.contains(f.id()) {
            Some(regions::before(&mut k, f, &region_map, &bodies)?)
        } else {
            None
        };
        let mut boolean_ms = 0.;
        let mut measure_ms = 0.;
        let mut grid_validation = "not_applicable";
        let mut hollow_report = Value::Null;
        let result = (|| -> Result<Brep> {
            let p = &model.parameters;
            let b = match f {
                Feature::Hollow { .. } => {
                    let (body, report) = hollow::execute(&mut k, f, model, &mut bodies, &mut uses)?;
                    hollow_report = report;
                    body
                }
                Feature::NameTopology { .. } => {
                    naming::execute(&mut k, f, model, &mut bodies, &mut uses)?
                }
                Feature::SurfaceNetwork { .. }
                | Feature::SurfacePatch { .. }
                | Feature::SurfaceBridge { .. }
                | Feature::SurfaceEdit { .. }
                | Feature::Thicken { .. }
                | Feature::Sew { .. } => {
                    surfaces::execute(&mut k, f, model, &mut bodies, &mut uses)?
                }
                Feature::InspectionRegion {
                    input,
                    region,
                    index,
                    require_exact,
                    ..
                } => {
                    let out = regions::extract(
                        &mut k,
                        &region_map,
                        input,
                        region,
                        *index,
                        *require_exact,
                    )?;
                    let _ = take_body(&mut k, input, &mut bodies, &mut uses)?;
                    out
                }
                Feature::Loft {
                    sections, degree, ..
                } => {
                    use solidcore::curve::{curve::Curve, ellipse::Ellipse};
                    use solidcore::geom::axis2_placement::Axis2Placement;
                    let mut curves = vec![];
                    for section in sections {
                        let frame =
                            Axis2Placement::new(point(&section.center, p)?, Vec3::X, Vec3::Y);
                        curves.push(Curve::from(Ellipse::create_canonical(
                            &k,
                            &frame,
                            section.radii[0].resolve(p)?,
                            section.radii[1].resolve(p)?,
                            None,
                            None,
                        )?));
                    }
                    let refs: Vec<_> = curves.iter().map(Some).collect();
                    solidcore::sweep::api_surfaces::create_skin_primitive(
                        &mut k,
                        &vec![1; sections.len()],
                        &refs,
                        *degree,
                        true,
                    )?
                    .brep
                }
                Feature::Box { origin, size, .. } => create_box(
                    &mut k,
                    point(origin, p)?,
                    size[0].resolve(p)?,
                    size[1].resolve(p)?,
                    size[2].resolve(p)?,
                )?,
                Feature::Cone {
                    origin,
                    base_radius,
                    top_radius,
                    height,
                    ..
                } => solidcore::primitive::api_primitives::create_cone(
                    &mut k,
                    point(origin, p)?,
                    base_radius.resolve(p)?,
                    top_radius.resolve(p)?,
                    height.resolve(p)?,
                )?,
                Feature::Cylinder {
                    origin,
                    radius,
                    height,
                    ..
                } => create_cylinder(
                    &mut k,
                    point(origin, p)?,
                    radius.resolve(p)?,
                    height.resolve(p)?,
                )?,
                Feature::Sphere { center, radius, .. } => {
                    create_sphere(&mut k, point(center, p)?, radius.resolve(p)?)?
                }
                Feature::Cut { left, right, .. }
                | Feature::Union { left, right, .. }
                | Feature::Intersect { left, right, .. } => {
                    let a = take_body(&mut k, left, &mut bodies, &mut uses)?;
                    let b = take_body(&mut k, right, &mut bodies, &mut uses)?;
                    match f {
                        Feature::Cut { .. } => operations::cut_checked(&mut k, a, b)?,
                        Feature::Union { .. } => boolean_union(&mut k, a, b)?,
                        _ => boolean_intersection(&mut k, a, b)?,
                    }
                }
                Feature::Translate { input, offset, .. } => {
                    let mut b = take_body(&mut k, input, &mut bodies, &mut uses)?;
                    let o = point(offset, p)?;
                    solidcore::topology::brep::mirror::transform_matrix(
                        &mut k,
                        &mut b,
                        &[
                            [1., 0., 0., 0.],
                            [0., 1., 0., 0.],
                            [0., 0., 1., 0.],
                            [o.x, o.y, o.z, 1.],
                        ],
                        true,
                    )?;
                    b
                }
                Feature::HoleGrid {
                    input,
                    origin,
                    radius,
                    depth,
                    count,
                    spacing,
                    ..
                } => {
                    let mut b = take_body(&mut k, input, &mut bodies, &mut uses)?;
                    let o = point(origin, p)?;
                    let [nx, ny] = grid_counts(count, p)?;
                    let expected_volume = grid_stock_volume(model, f)?;
                    grid_validation = if expected_volume.is_some() {
                        "analytic_stock_volume"
                    } else {
                        "per_cut_volume"
                    };
                    let measure_start = Instant::now();
                    let mut volume = if expected_volume.is_none() {
                        brep_volume_default(&k, &mut b, 1e-6)?
                    } else {
                        0.
                    };
                    measure_ms += measure_start.elapsed().as_secs_f64() * 1000.;
                    if expected_volume.is_some() {
                        let tick = Instant::now();
                        b = grid::stock(&mut k, model, f)?;
                        boolean_ms += tick.elapsed().as_secs_f64() * 1000.;
                        grid_validation = "direct_analytic_topology_with_volume_check";
                    } else {
                        for y in 0..ny {
                            for x in 0..nx {
                                let at = Vec3::new(
                                    o.x + x as f64 * spacing[0].resolve(p)?,
                                    o.y + y as f64 * spacing[1].resolve(p)?,
                                    o.z,
                                );
                                let tool = create_cylinder(
                                    &mut k,
                                    at,
                                    radius.resolve(p)?,
                                    depth.resolve(p)?,
                                )?;
                                let boolean_start = Instant::now();
                                b = boolean_difference(&mut k, b, tool)?;
                                boolean_ms += boolean_start.elapsed().as_secs_f64() * 1000.;
                                if expected_volume.is_none() {
                                    let measure_start = Instant::now();
                                    let after = brep_volume_default(&k, &mut b, 1e-6)?;
                                    measure_ms += measure_start.elapsed().as_secs_f64() * 1000.;
                                    if volume - after <= (volume.abs() * 1e-9).max(1e-9) {
                                        return Err(CadError::new(
                                            "NO_MATERIAL_REMOVED",
                                            format!("hole [{x},{y}] did not cut material"),
                                        ));
                                    }
                                    volume = after;
                                }
                            }
                        }
                    }
                    if let Some(expected) = expected_volume {
                        let measure_start = Instant::now();
                        let actual = brep_volume_default(&k, &mut b, 1e-6)?;
                        measure_ms += measure_start.elapsed().as_secs_f64() * 1000.;
                        if (actual - expected).abs() > expected.abs() * 1e-6 + 1e-8 {
                            return Err(CadError::new(
                                "VOLUME_MISMATCH",
                                format!("grid expected {expected}, measured {actual} mm3"),
                            ));
                        }
                    }
                    b
                }
                other => operations::execute(&mut k, other, model, &mut bodies, &mut uses)?,
            };
            if b.face_list().len() > 20_000 {
                return Err(CadError::new("RESOURCE_LIMIT", "face budget exceeded"));
            }
            if !brep_is_manifold_solid(&b)?
                && !matches!(
                    f,
                    Feature::SurfaceNetwork { .. }
                        | Feature::SurfacePatch { .. }
                        | Feature::SurfaceBridge { .. }
                        | Feature::SurfaceEdit { .. }
                        | Feature::Sew { .. }
                        | Feature::Translate { .. }
                        | Feature::Place { .. }
                        | Feature::Rotate { .. }
                        | Feature::Mirror { .. }
                        | Feature::NameTopology { .. }
                )
            {
                return Err(CadError::new(
                    "INVALID_SOLID",
                    "feature did not produce a closed manifold solid",
                ));
            }
            Ok(b)
        })();
        let (result, degradation) = match (result, failure_policy) {
            (Err(e), Some((backup, mut declaration)))
                if matches!(
                    e.code,
                    "KERNEL_ERROR" | "INVALID_SOLID" | "GEOMETRY_VALIDATION_FAILED"
                ) =>
            {
                declaration["error"] = json!(e);
                (Ok(backup), Some(declaration))
            }
            (r, _) => (r, None),
        };
        let b = result.map_err(|mut e| {
            e.feature_id = Some(f.id().to_string());
            e
        })?;
        if let Some(pending) = pending_regions {
            if let Some(r) = regions::after(&mut k, f, model, pending, &b)? {
                region_map.insert(f.id().to_string(), r);
            }
        }
        let surface = if matches!(
            f,
            Feature::SurfaceNetwork { .. }
                | Feature::SurfacePatch { .. }
                | Feature::SurfaceBridge { .. }
                | Feature::SurfaceEdit { .. }
        ) {
            Some(surfaces::report(model, f.id())?)
        } else {
            None
        };
        let inspection = region_map.get(f.id()).map(|r| r.report());
        trace.push(json!({"hollow":hollow_report,"topology_names":naming::report(&b),"surface":surface,"degradation":degradation,"inspection_regions":inspection,"id":f.id(), "elapsed_ms":tick.elapsed().as_secs_f64()*1000., "faces":b.face_list().len(),
            "grid_boolean_ms":boolean_ms,"grid_measure_ms":measure_ms,"grid_validation":grid_validation}));
        if !required_regions.contains(f.id()) {
            let row = trace.last_mut().expect("current feature");
            row["cache_hit"] = json!(false);
            cache.capture(&k, f.id(), &b, row);
        }
        bodies.insert(f.id().to_string(), b);
        peak_live_bodies = peak_live_bodies.max(bodies.len());
        region_map.retain(|id, _| uses.get(id).copied().unwrap_or(0) > 0);
    }
    let geometry_ms = start.elapsed().as_secs_f64() * 1000.;
    let snapshot_start = Instant::now();
    let mut snapshot = if queryable {
        Some(query::artifact::Capture::new(&k, model, &bodies)?)
    } else {
        None
    };
    let snapshot_ms = snapshot_start.elapsed().as_secs_f64() * 1000.;
    let datum_start = Instant::now();
    if let Some(snapshot) = &mut snapshot {
        for definition in model
            .interfaces
            .iter()
            .map(|d| &d.definition)
            .collect::<BTreeSet<_>>()
        {
            relationships::require_exact(model, definition, &trace)?;
        }
        snapshot.resolve_datums(model)?;
    }
    let datum_ms = datum_start.elapsed().as_secs_f64() * 1000.;
    if !model.assembly.is_empty() {
        let mut result = assembly::finish(
            &mut k,
            model,
            &mut bodies,
            &mut uses,
            trace,
            peak_live_bodies,
            start,
            geometry_ms,
            cache,
            &snapshot
                .as_ref()
                .map(|s| &s.datums)
                .cloned()
                .unwrap_or_default(),
            &snapshot
                .as_ref()
                .map(|s| &s.interfaces)
                .cloned()
                .unwrap_or_default(),
        )?;
        if let Some(snapshot) = snapshot {
            let t = Instant::now();
            snapshot.finish(model, &mut result)?;
            result.report["timing_ms"]["datum_resolution"] = json!(datum_ms);
            result.report["timing_ms"]["query_snapshot"] =
                json!(snapshot_ms + t.elapsed().as_secs_f64() * 1000.);
            result.report["timing_ms"]["total"] = json!(start.elapsed().as_secs_f64() * 1000.);
        }
        return Ok(result);
    }
    let mut body = take_body(&mut k, &model.output, &mut bodies, &mut uses)?;
    let tick = Instant::now();
    let check = check_brep(&mut k, &mut body, CheckLevel::Level1)?;
    if !check.is_valid() {
        return Err(CadError::new(
            "GEOMETRY_VALIDATION_FAILED",
            check.lines().join("; "),
        ));
    }
    let manifold = brep_is_manifold_solid(&body)?;
    if !manifold && !matches!(model.output_kind, OutputKind::Surface) {
        return Err(CadError::new(
            "OUTPUT_KIND",
            "surface output must be declared explicitly",
        ));
    }
    if manifold && matches!(model.output_kind, OutputKind::Surface) {
        return Err(CadError::new(
            "OUTPUT_KIND",
            "requested surface output produced a solid",
        ));
    }
    let volume = if manifold {
        measured_volume(&mut k, &mut body)?
    } else {
        0.
    };
    if !volume.is_finite() || (manifold && volume <= 0.) {
        return Err(CadError::new(
            "INVALID_VOLUME",
            "positive finite volume required",
        ));
    }
    let bb = brep_bounding_box(&k, &body, true)?;
    let (solid_count, void_count) = brep_material_census(&body)?;
    let validation_ms = tick.elapsed().as_secs_f64() * 1000.;
    let tick = Instant::now();
    let neutral = exchange::to_model(&body)
        .map_err(|e| CadError::new("EXPORT_UNSUPPORTED", e.to_string()))?;
    let step = if manifold {
        step_assembly::write_solids(&neutral)?
    } else {
        step_assembly::write_surface(&neutral)?
    };
    if step.len() > 32 * 1024 * 1024 {
        return Err(CadError::new(
            "RESOURCE_LIMIT",
            "STEP exceeds 32 MiB artifact budget",
        ));
    }
    let export_ms = tick.elapsed().as_secs_f64() * 1000.;
    let (artifacts, preview_report) = preview::finish(
        &mut k,
        model,
        vec![preview::InstanceMesh {
            id: model.output.clone(),
            definition: &model.output,
            body: &body,
            pose: Default::default(),
        }],
        cache,
    )?;
    let mut result = Build {
        artifacts,
        report: json!({
            "preview":preview_report,"model_hash":model_hash(model),"executor_version":EXECUTOR_VERSION,"execution_policy":EXECUTION_POLICY, "kernel":kernel_policy(),
            "validation":{"level":"kernel_level_1", "valid":true,"warnings":check.warnings().count(),"manifold":manifold},
            "geometry":{"kind":model.output_kind,"volume_mm3":volume,"bbox_mm":[[bb.min_at(0),bb.min_at(1),bb.min_at(2)],[bb.max_at(0),bb.max_at(1),bb.max_at(2)]],
                "faces":body.face_list().len(),"edges":body.edge_list().len(),"solids":solid_count,"void_regions":void_count},
            "timing_ms":{"geometry":geometry_ms,"validation":validation_ms,"export":export_ms,"total":start.elapsed().as_secs_f64()*1000.},
            "peak_live_feature_bodies":peak_live_bodies,"features":trace,"step_bytes":step.len(),
            "editability":{"source":"model.json","parameter_replay":true,"native_commercial_feature_tree":false}
        }),
        step,
    };
    if let Some(snapshot) = snapshot {
        let t = Instant::now();
        snapshot.finish(model, &mut result)?;
        result.report["timing_ms"]["datum_resolution"] = json!(datum_ms);
        result.report["timing_ms"]["query_snapshot"] =
            json!(snapshot_ms + t.elapsed().as_secs_f64() * 1000.);
        result.report["timing_ms"]["total"] = json!(start.elapsed().as_secs_f64() * 1000.);
    }
    Ok(result)
}
