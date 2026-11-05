//! STEP boundary conventions, isolated from the pinned kernel sources.
use crate::*;
use step_import::model::{Model as NeutralModel, Surface};

pub(crate) fn to_model(b: &Brep) -> std::result::Result<NeutralModel, step_brep::ExportModelError> {
    let mut m = step_brep::brep_to_model(b)?;
    // A material region is one solid, including all of its closed cavity shells.
    // The pinned bridge emits one body per shell; reconstruct ownership before writing STEP.
    if brep_is_manifold_solid(b).map_err(|e| step_brep::ExportModelError {
        message: e.to_string(),
    })? {
        let topology = (|| -> Result<Vec<step_import::model::Body>> {
            let indices = b
                .face_list()
                .iter()
                .enumerate()
                .map(|(i, f)| (*f, i))
                .collect::<BTreeMap<_, _>>();
            let mut bodies = vec![];
            for id in b.regions().ids() {
                let region = b.region(id)?;
                if Some(id) == b.infinite_region() || region.is_void() != 0 {
                    continue;
                }
                let mut shells = vec![];
                for shell in region.get_shells(None, |_, _| Ok(true))? {
                    let mut faces = BTreeSet::new();
                    for use_id in b.shell(shell)?.get_faceuses()? {
                        if let Some(face) = b.faceuse(use_id)?.face {
                            faces.insert(indices[&face]);
                        }
                    }
                    if faces.is_empty() {
                        continue;
                    }
                    let matched = m
                        .bodies
                        .iter()
                        .flat_map(|b| &b.shells)
                        .find(|s| s.faces.iter().copied().collect::<BTreeSet<_>>() == faces)
                        .ok_or_else(|| CadError::new("EXPORT_FAILED", "unmapped material shell"))?;
                    shells.push(matched.clone());
                }
                if shells.is_empty() {
                    continue;
                }
                let mut body = m.bodies[0].clone();
                body.step_id = bodies.len() as u64 + 1;
                body.kind = if shells.len() > 1 {
                    step_import::model::BodyKind::SolidWithVoids
                } else {
                    step_import::model::BodyKind::ManifoldSolid
                };
                body.shells = shells;
                bodies.push(body);
            }
            Ok(bodies)
        })()
        .map_err(|e| step_brep::ExportModelError {
            message: e.to_string(),
        })?;
        if !topology.is_empty() {
            m.bodies = topology;
        }
    }
    // Solidcore's cone STEP conversion uses axial height for V. OCCT / ISO cone
    // parameters use distance along the generator: z = v*cos(semi_angle).
    // Geometry, edge curves and cone angles are already correct; fix only pcurves.
    let mut scales = BTreeMap::new();
    for body in &mut m.bodies {
        for face in &mut body.faces {
            if let Surface::Cone {
                semi_angle_deg,
                frame,
                ..
            } = &mut face.surface
            {
                let mut u = 1.;
                let mut v = 1. / semi_angle_deg.to_radians().cos();
                // STEP conical_surface requires a positive semi-angle. Reverse the axis
                // for a decreasing-radius cone, and transform both trim parameters.
                if *semi_angle_deg < 0. {
                    *semi_angle_deg = -*semi_angle_deg;
                    frame.y = frame.y.map(|x| -x);
                    frame.z = frame.z.map(|x| -x);
                    u = -1.;
                    v = -v;
                }
                scales.insert(face.step_id, (u, v));
            }
        }
    }
    for ((face, _, _), curves) in &mut m.pcurves {
        if let Some((u, v)) = scales.get(face) {
            for curve in curves {
                for uv in curve {
                    uv[0] *= u;
                    uv[1] *= v;
                }
            }
        }
    }

    Ok(m)
}

/// Build enclosing/void shells separately, then subtract their exact solids.
/// The pinned constructor's nested-region classification fails on valid oriented void shells.
pub(crate) fn build_body(k: &mut Kernel, body: &step_import::model::Body) -> Result<Brep> {
    use step_import::model::BodyKind;
    if body.kind != BodyKind::SolidWithVoids {
        // The bridge shares geometry arrays across bodies. A standalone shell must not
        // feed unrelated faces to the stitched fallback, which builds every array entry.
        let mut source = body.clone();
        let used = source
            .shells
            .iter()
            .flat_map(|s| s.faces.iter().copied())
            .collect::<BTreeSet<_>>();
        let indices = used
            .iter()
            .enumerate()
            .map(|(new, old)| (*old, new))
            .collect::<BTreeMap<_, _>>();
        source.faces = used.iter().map(|i| body.faces[*i].clone()).collect();
        for shell in &mut source.shells {
            for face in &mut shell.faces {
                *face = indices[face];
            }
        }
        let made = step_brep::build_body(&source, step_brep::BuildOptions::default())
            .map_err(|e| CadError::new("IMPORT_FAILED", e.to_string()))?;
        if !made.face_failures.is_empty() {
            return Err(CadError::new("IMPORT_INCOMPLETE", "STEP body lost faces"));
        }
        return Ok(made.brep);
    }
    if body.shells.len() < 2 || body.shells.len() > 129 {
        return Err(CadError::new(
            "IMPORT_BUDGET",
            "1..128 enclosed voids required",
        ));
    }
    let mut parts = vec![];
    for (index, shell) in body.shells.iter().enumerate() {
        let mut part = body.clone();
        part.kind = BodyKind::ManifoldSolid;
        part.shells = vec![shell.clone()];
        // Normalize every isolated shell to outward orientation before subtraction.
        if (index == 0 && !shell.orientation) || (index > 0 && shell.orientation) {
            for &face in &shell.faces {
                part.faces[face].same_sense = !part.faces[face].same_sense;
                for lp in &mut part.faces[face].loops {
                    lp.orientation = !lp.orientation;
                }
            }
        }
        part.shells[0].orientation = true;
        parts.push(build_body(k, &part)?);
    }
    let mut result = parts.remove(0);
    for cavity in parts {
        let a = solidcore::topology::brep::p3::brep_copy(k, &cavity)?;
        let b = solidcore::topology::brep::p3::brep_copy(k, &result)?;
        let mut outside = boolean_difference(k, a, b)?;
        if !outside.face_list().is_empty() && measured_volume(k, &mut outside)? > 1e-8 {
            return Err(CadError::new(
                "IMPORT_INVALID_VOID",
                "cavity is outside material or overlaps another void",
            ));
        }
        if distance::boundary_distance(k, &result, &cavity)? <= 1e-6 {
            return Err(CadError::new(
                "IMPORT_INVALID_VOID",
                "void shells must be strictly disjoint",
            ));
        }
        result = boolean_difference(k, result, cavity)?;
    }
    Ok(result)
}
