//! Two explicit snapshot targets; immutable definitions are loaded once per batch.
use super::{contract::*, engine::Context, snapshot::LoadedDefinition};
use crate::*;
use std::path::Path;
pub(crate) fn target(
    manifest: &super::artifact::Manifest,
    t: &Target,
    space: Space,
) -> Result<(String, Option<Vec<String>>, placement::Rigid)> {
    match t {
        Target::Definition { definition_id } => {
            if space == Space::Assembly {
                return Err(CadError::new(
                    "QUERY_ARGUMENT",
                    "assembly space requires occurrence targets",
                ));
            }
            if !manifest.body.definitions.contains_key(definition_id) {
                return Err(CadError::new(
                    "TOPOLOGY_REF_INVALID",
                    "definition not in snapshot",
                ));
            }
            Ok((definition_id.clone(), None, Default::default()))
        }
        Target::Instance { instance_path } => {
            let o = manifest
                .body
                .occurrences
                .values()
                .find(|o| &o.instance_path == instance_path)
                .ok_or_else(|| {
                    CadError::new("TOPOLOGY_REF_INVALID", "instance path not in snapshot")
                })?;
            if !o.representation.is_null() && o.representation["kind"] != "exact" {
                return Err(CadError::new(
                    "QUERY_UNCERTAIN",
                    "cannot measure exact relationship on reduced representation",
                ));
            }
            Ok((
                o.definition_id.clone(),
                Some(instance_path.clone()),
                if space == Space::Assembly {
                    o.placement()?
                } else {
                    Default::default()
                },
            ))
        }
    }
}
pub(crate) fn run(
    c: &mut Context,
    root: &Path,
    cache: &mut BTreeMap<String, LoadedDefinition>,
    space: Space,
    a: &Target,
    b: &Target,
) -> Result<Value> {
    let (ad, _, ap) = target(&c.manifest, a, space)?;
    let (bd, _, bp) = target(&c.manifest, b, space)?;
    for id in [&ad, &bd] {
        if id != &c.definition && !cache.contains_key(id) {
            let loaded = c.manifest.load(root, id, &mut c.kernel)?;
            cache.insert(id.clone(), loaded);
        }
    }
    let mut faces = c.loaded.body.faces().len();
    let mut edges = c.loaded.body.edges().len();
    for d in cache.values() {
        faces += d.body.faces().len();
        edges += d.body.edges().len();
    }
    if faces > 10000 || edges > 50000 {
        return Err(CadError::new(
            "QUERY_LIMIT",
            "batch definition topology budget exceeded",
        ));
    }
    if !c.manifest.body.degradations.is_empty() {
        return Err(CadError::new("QUERY_UNCERTAIN","exact measurement on a snapshot with degraded features requires rebuilding without degradation"));
    }
    let mut ac = solidcore::topology::brep::p3::brep_copy(
        &mut c.kernel,
        if ad == c.definition {
            &c.loaded.body
        } else {
            &cache[&ad].body
        },
    )?;
    let mut bc = solidcore::topology::brep::p3::brep_copy(
        &mut c.kernel,
        if bd == c.definition {
            &c.loaded.body
        } else {
            &cache[&bd].body
        },
    )?;
    ap.apply(&mut c.kernel, &mut ac)?;
    bp.apply(&mut c.kernel, &mut bc)?;
    let mut result = relationships::measure(&mut c.kernel, &ac, &bc, c.tolerance)?;
    result["a"] = json!(a);
    result["b"] = json!(b);
    result["space"] = json!(space);
    Ok(result)
}
