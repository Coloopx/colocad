//! Preserve independent solids, including overlapping copies; never fuse a pattern.
use crate::*;
pub(crate) fn append(k: &mut Kernel, target: &mut Brep, source: &Brep) -> Result<()> {
    use solidcore::geom::{
        attribute::{AttributeStore, ObjectKey},
        context::TOPOLOGY_KEY_BASE,
        object::ObjectRef,
    };
    use solidcore::topology::{
        brep::p3::{merge_brep, topology_ref},
        edge::NotifyEnv,
        topology::NotifyRecords,
    };
    let key = |e, side: u64| {
        let r = topology_ref(0, e);
        ObjectRef {
            type_code: r.type_code,
            key: ObjectKey((r.key.0 & !TOPOLOGY_KEY_BASE) | (side << 40)),
        }
    };
    let dest = |e| key(e, 0);
    let src = |e| key(e, 1);
    let mut records = NotifyRecords::new();
    let mut attributes = AttributeStore::default();
    let mut env = NotifyEnv {
        records: &mut records,
        attributes: &mut attributes,
        context: None,
        key_of: &dest,
    };
    merge_brep(k, &mut env, target, source, &src, None, None, true)?;
    *target = solidcore::topology::brep::p3::brep_copy(k, target)?;
    Ok(())
}
