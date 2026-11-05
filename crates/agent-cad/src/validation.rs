//! Schema, reference and resource validation, without kernel execution.
use crate::*;
pub fn validate(model: &Model) -> Result<()> {
    if let Some(p) = &model.preview {
        p.validate()?;
    }
    if model.version != 1 || model.units != "mm" {
        return Err(CadError::new(
            "SCHEMA_ERROR",
            "version 1 and units mm required",
        ));
    }
    if model.features.is_empty()
        || model.features.len() > MAX_FEATURES
        || model.parameters.len() > 128
    {
        return Err(CadError::new(
            "RESOURCE_LIMIT",
            "1..128 features and at most 128 parameters required",
        ));
    }
    for (id, n) in &model.parameters {
        if !valid_id(id) || !n.is_finite() || n.abs() > MAX_COORD {
            return Err(CadError::new("INVALID_PARAMETER", id));
        }
    }
    let mut seen = BTreeSet::new();
    let mut cutters = 0;
    for f in &model.features {
        let result = (|| {
            if !valid_id(f.id()) || seen.contains(f.id()) {
                return Err(CadError::new("INVALID_FEATURE_ID", f.id()));
            }
            for input in f.inputs() {
                if !seen.contains(input) {
                    return Err(CadError::new(
                        "INVALID_REFERENCE",
                        format!("{input} must reference an earlier feature"),
                    ));
                }
            }
            if let Feature::Holes { positions, .. } | Feature::FaceHoles { positions, .. } = f {
                cutters += positions.len();
            }
            if let Feature::FaceHoleGrid { count, .. } = f {
                let [nx, ny] = grid_counts(count, &model.parameters)?;
                cutters += nx * ny;
            }
            if let Feature::Extrude { holes, .. } = f {
                cutters += holes.len();
            }
            if cutters > MAX_CUTTERS {
                return Err(CadError::new(
                    "RESOURCE_LIMIT",
                    "at most 1024 cutters per model",
                ));
            }
            let p = &model.parameters;
            naming::validate(f, p)?;
            if let Feature::Hollow { wall, .. } = f {
                positive(wall.resolve(p)?)?;
            }
            if let Feature::Fillet {
                on_fail:
                    OnFailure::Record {
                        reason,
                        omitted,
                        affects,
                    },
                ..
            }
            | Feature::Chamfer {
                on_fail:
                    OnFailure::Record {
                        reason,
                        omitted,
                        affects,
                    },
                ..
            } = f
            {
                if reason.trim().is_empty()
                    || reason.len() > 800
                    || omitted.is_empty()
                    || affects.is_empty()
                    || omitted
                        .iter()
                        .chain(affects)
                        .any(|s| s.trim().is_empty() || s.len() > 800)
                {
                    return Err(CadError::new(
                        "INVALID_DEGRADATION",
                        "reason, omitted features and impacts are required",
                    ));
                }
            }
            match f {
                Feature::Loft {
                    sections, degree, ..
                } => {
                    if sections.len() < 2
                        || sections.len() > 12
                        || !(1..=3).contains(degree)
                        || *degree >= sections.len()
                    {
                        return Err(CadError::new(
                            "INVALID_LOFT",
                            "2..12 sections; degree 1..3 and below section count",
                        ));
                    }
                    let mut last_z = f64::NEG_INFINITY;
                    for section in sections {
                        let c = point(&section.center, p)?;
                        if c.z - last_z <= 1e-4 {
                            return Err(CadError::new(
                                "INVALID_LOFT",
                                "section z coordinates must increase",
                            ));
                        }
                        last_z = c.z;
                        for r in &section.radii {
                            positive(r.resolve(p)?)?;
                        }
                    }
                }
                Feature::Box { origin, size, .. } => {
                    point(origin, p)?;
                    for s in size {
                        positive(s.resolve(p)?)?;
                    }
                }
                Feature::Cone {
                    origin,
                    base_radius,
                    top_radius,
                    height,
                    ..
                } => {
                    point(origin, p)?;
                    positive(height.resolve(p)?)?;
                    let (a, b) = (base_radius.resolve(p)?, top_radius.resolve(p)?);
                    if a < 0. || b < 0. || a.max(b) <= 0. {
                        return Err(CadError::new(
                            "INVALID_DIMENSION",
                            "nonnegative cone radii, at least one positive",
                        ));
                    }
                }
                Feature::Cylinder {
                    origin,
                    radius,
                    height,
                    ..
                } => {
                    point(origin, p)?;
                    positive(radius.resolve(p)?)?;
                    positive(height.resolve(p)?)?;
                }
                Feature::Sphere { center, radius, .. } => {
                    point(center, p)?;
                    positive(radius.resolve(p)?)?;
                }
                Feature::Translate { offset, .. } => {
                    point(offset, p)?;
                }
                Feature::HoleGrid {
                    origin,
                    radius,
                    depth,
                    count,
                    spacing,
                    ..
                } => {
                    let o = point(origin, p)?;
                    let r = positive(radius.resolve(p)?)?;
                    let d = positive(depth.resolve(p)?)?;
                    let [nx, ny] = grid_counts(count, p)?;
                    cutters += nx * ny;
                    if cutters > MAX_CUTTERS {
                        return Err(CadError::new(
                            "RESOURCE_LIMIT",
                            "at most 1024 cutters per model",
                        ));
                    }
                    for (i, s) in spacing.iter().enumerate() {
                        let s = s.resolve(p)?;
                        let count = [nx, ny][i];
                        if count > 1 && s.abs() <= 2. * r {
                            return Err(CadError::new(
                                "OVERLAPPING_HOLES",
                                "grid spacing must exceed diameter",
                            ));
                        }
                        let start = [o.x, o.y][i];
                        if (start + (count - 1) as f64 * s).abs() + r > MAX_COORD {
                            return Err(CadError::new(
                                "RESOURCE_LIMIT",
                                "grid extent exceeds coordinate budget",
                            ));
                        }
                    }
                    if (o.z + d).abs() > MAX_COORD {
                        return Err(CadError::new(
                            "RESOURCE_LIMIT",
                            "grid depth exceeds coordinate budget",
                        ));
                    }
                }
                _ => operations::validate_feature(f, p)?,
            }
            Ok(())
        })();
        result.map_err(|mut e: CadError| {
            e.feature_id = Some(f.id().to_string());
            e
        })?;
        seen.insert(f.id().to_string());
    }
    if model.assets.len() > 16 {
        return Err(CadError::new("RESOURCE_LIMIT", "at most 16 STEP assets"));
    }
    let mut asset_bytes = 0;
    for (id, a) in &model.assets {
        asset_bytes += a.step.len();
        if !valid_id(id) || format!("{:x}", Sha256::digest(a.step.as_bytes())) != a.sha256 {
            return Err(CadError::new("ASSET_DIGEST_MISMATCH", id));
        }
    }
    if asset_bytes > 24 * 1024 * 1024 {
        return Err(CadError::new(
            "RESOURCE_LIMIT",
            "STEP asset bytes exceed 24 MiB",
        ));
    }
    for f in &model.features {
        surfaces::validate(f, model)?;
        if let Feature::ImportStep { asset, .. } | Feature::ComponentCad { asset, .. } = f {
            if !model.assets.contains_key(asset) {
                return Err(CadError::new("ASSET_NOT_FOUND", asset));
            }
        }
    }
    assembly::validate(model, &seen)?;
    if model.assembly.is_empty() && !seen.contains(&model.output) {
        return Err(CadError::new("INVALID_REFERENCE", "output does not exist"));
    }
    // Reject disconnected construction rather than silently dropping requested features.
    let mut needed = if model.assembly.is_empty() {
        BTreeSet::from([model.output.as_str()])
    } else {
        model.assembly.iter().map(|x| x.input.as_str()).collect()
    };
    for f in model.features.iter().rev() {
        if needed.contains(f.id()) {
            needed.extend(f.inputs());
        }
    }
    if needed.len() != model.features.len() {
        return Err(CadError::new(
            "UNUSED_FEATURE",
            "every feature must contribute to output",
        ));
    }
    Ok(())
}
