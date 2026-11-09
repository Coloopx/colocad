//! Definition-level exact serialization spike for SDD 0004 M0.
use crate::*;
use solidcore::geom::attribute::{AttributeStore, ObjectKey};
use solidcore::topology::{
    api_files, brep::p10::TopologyEnv, brep_data::BrepData, brep_data_write::FromBrepEnv,
    database_io::FileType, topology::NotifyRecords,
};
use solidcore::EntityRef;

const SCHEMA: &str = "colocad.definition-snapshot/1";
const MAX_PAYLOAD: usize = 64 * 1024 * 1024;
const MAX_BINDINGS: usize = 200_000;
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn entities(b: &Brep) -> Vec<EntityRef> {
    b.face_list()
        .iter()
        .copied()
        .map(EntityRef::Face)
        .chain(b.edge_list().iter().copied().map(EntityRef::Edge))
        .chain(b.vertex_list().iter().copied().map(EntityRef::Vertex))
        .chain(b.loops().ids().map(EntityRef::Loop))
        .collect()
}
fn kind(e: EntityRef) -> &'static str {
    match e {
        EntityRef::Face(_) => "face",
        EntityRef::Edge(_) => "edge",
        EntityRef::Vertex(_) => "vertex",
        EntityRef::Loop(_) => "loop",
        _ => unreachable!(),
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Binding {
    pub kind: String,
    pub names: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DefinitionSnapshot {
    pub schema: String,
    pub kernel_revision: String,
    pub geometry_sha256: String,
    pub bindings: BTreeMap<u64, Binding>,
    pub geometry: Vec<u8>,
}
pub struct LoadedDefinition {
    pub body: Brep,
    pub entities: BTreeMap<u64, EntityRef>,
}
impl DefinitionSnapshot {
    /// Assign identities once, write them into native topology records, and bind
    /// semantic names to those persisted identities. Arena IDs are never the ID.
    pub fn capture(k: &Kernel, b: &Brep) -> Result<Self> {
        if b.face_list().len() > 10_000 || b.edge_list().len() > 50_000 {
            return Err(CadError::new(
                "QUERY_LIMIT",
                "definition topology budget exceeded",
            ));
        }
        let all = entities(b);
        if all.len() > MAX_BINDINGS {
            return Err(CadError::new(
                "QUERY_LIMIT",
                "definition binding budget exceeded",
            ));
        }
        let attrs = AttributeStore::new();
        let mut records = NotifyRecords::new();
        let mut bindings = BTreeMap::new();
        for (i, e) in all.into_iter().enumerate() {
            let token = (i + 1) as u64;
            records
                .or_default(b.owner_identity(e).key)
                .set_user_index1(token);
            bindings.insert(
                token,
                Binding {
                    kind: kind(e).into(),
                    names: crate::naming::names(b, e),
                },
            );
        }
        let owner = |e| b.owner_identity(e);
        let env = FromBrepEnv {
            attributes: &attrs,
            records: &records,
            key_of: &owner,
            curve_key: &|_| ObjectKey(0),
            uv_curve_key: &|_| ObjectKey(0),
            surface_key: &|_| ObjectKey(0),
        };
        let mut data = BrepData::from_brep(b, &env, &mut Vec::new())?;
        // Materialize only exact lazy UV line records on the disposable data copy.
        // No healing, STEP fallback or tessellation; analytic surfaces retain type.
        for trim in &mut data.uv_curves {
            if let solidcore::curve::curve::p2::CurveKind::Line(line) = trim.object.kind() {
                trim.object =
                    solidcore::curve::curve::Curve::from(line.base_with_nurb()?.into_owned());
            }
        }
        // Direct/imported periodic faces may own seam edges represented as a
        // curve-on-surface. Its nested UV line is not in data.uv_curves, and the
        // native writer does not materialize that line's lazy NURB. Prepare the
        // exact line on this disposable serialization copy. Keep the seam,
        // support surface, parameter interval and topology identity intact.
        for curve in &mut data.curves_3d {
            if let solidcore::curve::curve::p2::CurveKind::CrvOnSurf(on_surface) =
                curve.object.kind_mut()
            {
                if let solidcore::curve::curve::p2::CurveKind::Line(line) =
                    on_surface.uv_curve().kind()
                {
                    if line.base().gw_nurb().is_none() {
                        let uv = solidcore::curve::curve::Curve::from(
                            line.base_with_nurb()?.into_owned(),
                        );
                        on_surface.set_uv_curve(uv)?;
                    }
                }
            }
        }
        let geometry = data.write_to_file_with_nurbs(k, FileType::Binary)?;
        if geometry.len() > MAX_PAYLOAD {
            return Err(CadError::new("QUERY_LIMIT", "exact payload exceeds 64 MiB"));
        }
        Ok(Self {
            schema: SCHEMA.into(),
            kernel_revision: KERNEL_REV.into(),
            geometry_sha256: sha(&geometry),
            bindings,
            geometry,
        })
    }
    /// Definition identity; assembly/query manifests add instance/source identities.
    /// JSON field order is fixed here and bindings are sorted by numeric token.
    pub fn digest(&self) -> String {
        sha(&serde_json::to_vec(&(
            &self.schema,
            &self.kernel_revision,
            &self.geometry_sha256,
            &self.bindings,
        ))
        .expect("serializable snapshot metadata"))
    }
    pub fn load(&self, k: &mut Kernel, expected_digest: &str) -> Result<LoadedDefinition> {
        if self.schema != SCHEMA || self.kernel_revision != KERNEL_REV {
            return Err(CadError::new(
                "QUERY_SNAPSHOT_INCOMPATIBLE",
                "unsupported definition snapshot format/kernel",
            ));
        }
        if self.geometry.len() > MAX_PAYLOAD
            || self.bindings.len() > MAX_BINDINGS
            || serde_json::to_vec(&self.bindings)
                .map_err(|e| CadError::new("QUERY_ARGUMENT", e.to_string()))?
                .len()
                > 8 * 1024 * 1024
        {
            return Err(CadError::new(
                "QUERY_LIMIT",
                "snapshot payload or binding budget exceeded",
            ));
        }
        if sha(&self.geometry) != self.geometry_sha256 {
            return Err(CadError::new(
                "ARTIFACT_INTEGRITY",
                "exact geometry digest mismatch",
            ));
        }
        if self.digest() != expected_digest {
            return Err(CadError::new(
                "QUERY_REVISION_MISMATCH",
                "definition bindings/digest differ from expected snapshot",
            ));
        }
        let mut attrs = AttributeStore::new();
        let mut records = NotifyRecords::new();
        // Deliberately use a fresh owner namespace. IDs are recovered from the
        // persisted records, not from list positions or nearest geometric matches.
        let owner = |e| solidcore::topology::brep::p3::topology_ref(419, e);
        let mut env = TopologyEnv {
            all_attributes: &[],
            attributes: &mut attrs,
            key_of: &owner,
            records: &mut records,
            next_key: 1 << 50,
        };
        let mut body =
            api_files::read_brep_from_file(k, Some(self.geometry.clone()), false, false, &mut env)?;
        body.refresh_cached_topology();
        let mut by_token = BTreeMap::new();
        for e in entities(&body) {
            let token = records
                .get(&owner(e).key)
                .map(|r| r.user_index1())
                .unwrap_or(0);
            let binding = self.bindings.get(&token).ok_or_else(|| {
                CadError::new("ARTIFACT_INTEGRITY", "missing persisted entity identity")
            })?;
            if binding.kind != kind(e) || by_token.insert(token, e).is_some() {
                return Err(CadError::new(
                    "ARTIFACT_INTEGRITY",
                    "duplicate or mismatched entity binding",
                ));
            }
            for name in &binding.names {
                crate::naming::attach(&mut body, e, name)?;
            }
        }
        if by_token.len() != self.bindings.len() {
            return Err(CadError::new(
                "ARTIFACT_INTEGRITY",
                "unresolved topology bindings",
            ));
        }
        Ok(LoadedDefinition {
            body,
            entities: by_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solidcore::topology::brep::validation::{check_brep, CheckLevel};
    fn named_box(k: &mut Kernel) -> Brep {
        let mut body = create_box(k, Vec3::ZERO, 10., 20., 30.).unwrap();
        let f = body.face_list()[0];
        crate::naming::attach(&mut body, EntityRef::Face(f), "stock/mount").unwrap();
        body
    }
    #[test]
    fn named_geometry_and_ids_survive_serialization_reload() {
        let mut k = Kernel::default();
        let body = named_box(&mut k);
        let snapshot = DefinitionSnapshot::capture(&k, &body).unwrap();
        let digest = snapshot.digest();
        // New process-equivalent kernel and a serialized sidecar; no original arenas.
        let restored: DefinitionSnapshot =
            serde_json::from_slice(&serde_json::to_vec(&snapshot).unwrap()).unwrap();
        let mut fresh = Kernel::default();
        let mut loaded = restored.load(&mut fresh, &digest).unwrap();
        assert_eq!(loaded.entities.len(), snapshot.bindings.len());
        let (&token, _) = snapshot
            .bindings
            .iter()
            .find(|(_, b)| b.names.contains(&"stock/mount".to_string()))
            .unwrap();
        assert!(crate::naming::has(
            &loaded.body,
            loaded.entities[&token],
            "stock/mount"
        ));
        assert!(check_brep(&mut fresh, &mut loaded.body, CheckLevel::Level1)
            .unwrap()
            .is_valid());
        assert!((measured_volume(&mut fresh, &mut loaded.body).unwrap() - 6000.).abs() < 1e-6);
    }
    #[test]
    fn analytic_bore_geometry_survives_without_step_or_mesh_fallback() {
        let mut k = Kernel::default();
        let block = create_box(&mut k, Vec3::ZERO, 20., 20., 10.).unwrap();
        let tool = create_cylinder(&mut k, Vec3::new(10., 10., -1.), 3., 12.).unwrap();
        let body = boolean_difference(&mut k, block, tool).unwrap();
        let snapshot = DefinitionSnapshot::capture(&k, &body).unwrap();
        let mut loaded = snapshot.load(&mut k, &snapshot.digest()).unwrap();
        assert_eq!(body.face_list().len(), loaded.body.face_list().len());
        let before = body
            .face_list()
            .iter()
            .map(|f| {
                format!(
                    "{:?}",
                    std::mem::discriminant(body.face(*f).unwrap().surface.as_ref().unwrap().kind())
                )
            })
            .collect::<Vec<_>>();
        let after = loaded
            .body
            .face_list()
            .iter()
            .map(|f| {
                format!(
                    "{:?}",
                    std::mem::discriminant(
                        loaded
                            .body
                            .face(*f)
                            .unwrap()
                            .surface
                            .as_ref()
                            .unwrap()
                            .kind()
                    )
                )
            })
            .collect::<Vec<_>>();
        let mut before = before;
        before.sort();
        let mut after = after;
        after.sort();
        assert_eq!(before, after);
        assert!(
            (measured_volume(&mut k, &mut loaded.body).unwrap()
                - (4000. - 90. * std::f64::consts::PI))
                .abs()
                < 1e-5
        );
    }
    #[test]
    fn changed_geometry_or_name_sidecar_is_not_the_same_snapshot() {
        let mut k = Kernel::default();
        let body = named_box(&mut k);
        let snapshot = DefinitionSnapshot::capture(&k, &body).unwrap();
        let digest = snapshot.digest();
        let mut bad = snapshot.clone();
        bad.geometry[0] ^= 1;
        assert_eq!(
            bad.load(&mut k, &digest).err().unwrap().code,
            "ARTIFACT_INTEGRITY"
        );
        let mut bad = snapshot.clone();
        bad.bindings
            .values_mut()
            .next()
            .unwrap()
            .names
            .push("forged".into());
        assert_eq!(
            bad.load(&mut k, &digest).err().unwrap().code,
            "QUERY_REVISION_MISMATCH"
        );
        let mut bad = snapshot.clone();
        bad.schema = "future".into();
        assert_eq!(
            bad.load(&mut k, &digest).err().unwrap().code,
            "QUERY_SNAPSHOT_INCOMPATIBLE"
        );
    }
}
