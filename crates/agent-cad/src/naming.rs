//! Semantic topology names carried by kernel copy/split/merge notifications.
//! Ambiguous multiplicity is explicit; a deleted name never binds by nearest geometry.
use crate::*;
use solidcore::{
    geom::attribute::{
        ids, ApplicationValue, Attribute, AttributeHandlers, AttributeValue, AttributeView,
        Behavior, ObjectKey, Placement,
    },
    EntityRef,
};
const ID: u64 = ids::USER_1 + 415;
#[derive(Clone, Debug, PartialEq)]
struct Names(BTreeSet<String>);
fn on(view: &AttributeView<'_>, key: ObjectKey) -> Option<Names> {
    view.find_attribute(key, ID)
        .and_then(|(_, a)| a.application_value::<Names>())
        .cloned()
}
fn joined(a: &Names, b: Option<Names>) -> Names {
    let mut n = a.clone();
    if let Some(b) = b {
        n.0.extend(b.0);
    }
    n
}
fn put(view: &AttributeView<'_>, key: ObjectKey, data: Names) -> Vec<Placement> {
    match view.find_attribute(key, ID) {
        Some((handle, _)) => vec![Placement::Edit {
            handle,
            value: AttributeValue::Application(ApplicationValue::new(data)),
        }],
        None => vec![Placement::Add {
            on: key,
            attribute: Attribute::application(ID, Behavior::Copy, data),
        }],
    }
}
impl AttributeHandlers for Names {
    fn split(
        &self,
        view: &AttributeView<'_>,
        original: ObjectKey,
        a: ObjectKey,
        b: ObjectKey,
    ) -> solidcore::Result<Vec<Placement>> {
        let mut out = vec![];
        for child in [a, b] {
            if child != original {
                out.extend(put(view, child, joined(self, on(view, child))));
            }
        }
        Ok(out)
    }
    fn merge(
        &self,
        view: &AttributeView<'_>,
        a: ObjectKey,
        b: ObjectKey,
        result: ObjectKey,
    ) -> solidcore::Result<Vec<Placement>> {
        let mut data = on(view, result).unwrap_or_else(|| self.clone());
        for from in [a, b] {
            if let Some(n) = on(view, from) {
                data.0.extend(n.0);
            }
        }
        Ok(put(view, result, data))
    }
    fn coincident(
        &self,
        view: &AttributeView<'_>,
        _from: ObjectKey,
        to: ObjectKey,
    ) -> solidcore::Result<Vec<Placement>> {
        Ok(put(view, to, joined(self, on(view, to))))
    }
}
fn read(b: &Brep, e: EntityRef) -> Option<&Names> {
    b.find_attribute(e, ID)
        .and_then(|a| a.application_value::<Names>())
}
fn attach(b: &mut Brep, e: EntityRef, name: &str) -> Result<()> {
    let mut data = read(b, e)
        .cloned()
        .unwrap_or_else(|| Names(BTreeSet::new()));
    data.0.insert(name.into());
    let handles = b
        .entity_attributes(e)
        .into_iter()
        .filter(|(_, a)| a.attribute_id() == ID)
        .map(|(h, _)| h)
        .collect::<Vec<_>>();
    for h in handles {
        b.remove_attribute(e, h)?;
    }
    b.add_attribute(e, Attribute::application(ID, Behavior::Copy, data))?;
    Ok(())
}
pub(crate) fn has(b: &Brep, e: EntityRef, name: &str) -> bool {
    read(b, e).is_some_and(|n| n.0.contains(name))
}
pub(crate) fn validate(f: &Feature, p: &BTreeMap<String, f64>) -> Result<()> {
    let Feature::NameTopology { edges, faces, .. } = f else {
        return Ok(());
    };
    if edges.len() + faces.len() > 64 {
        return Err(CadError::new(
            "NAMING_BUDGET",
            "at most 64 names per feature",
        ));
    }
    for name in edges.keys().chain(faces.keys()) {
        if !valid_id(name) {
            return Err(CadError::new("INVALID_TOPOLOGY_NAME", name));
        }
    }
    for select in edges.values() {
        operations::validate_selector(select, p)?;
    }
    for select in faces.values() {
        let edge = match select {
            FaceSelector::Named {
                name,
                expected_count,
            } => EdgeSelector::Named {
                name: name.clone(),
                expected_count: *expected_count,
            },
            FaceSelector::OnPlane {
                axis,
                value,
                tolerance,
                expected_count,
            } => EdgeSelector::OnPlane {
                axis: axis.clone(),
                value: value.clone(),
                tolerance: tolerance.clone(),
                expected_count: *expected_count,
            },
        };
        operations::validate_selector(&edge, p)?;
    }
    Ok(())
}
pub(crate) fn execute(
    k: &mut Kernel,
    f: &Feature,
    m: &Model,
    bodies: &mut BTreeMap<String, Brep>,
    uses: &mut BTreeMap<String, usize>,
) -> Result<Brep> {
    let Feature::NameTopology {
        id,
        input,
        edges,
        faces,
    } = f
    else {
        unreachable!()
    };
    let mut b = take_body(k, input, bodies, uses)?;
    if edges.len() + faces.len() > 64 {
        return Err(CadError::new(
            "NAMING_BUDGET",
            "at most 64 names per feature",
        ));
    }
    for (name, select) in edges {
        if !valid_id(name) {
            return Err(CadError::new("INVALID_TOPOLOGY_NAME", name));
        }
        let full = format!("{id}/{name}");
        for e in operations::select_edges(k, &b, select, &m.parameters)? {
            attach(&mut b, EntityRef::Edge(e), &full)?;
        }
    }
    for (name, select) in faces {
        if !valid_id(name) {
            return Err(CadError::new("INVALID_TOPOLOGY_NAME", name));
        }
        let full = format!("{id}/{name}");
        for face in operations::select_faces(k, &b, std::slice::from_ref(select), &m.parameters)? {
            attach(&mut b, EntityRef::Face(face), &full)?;
        }
    }
    Ok(b)
}
pub(crate) fn report(b: &Brep) -> Value {
    let mut names = BTreeMap::<String, [usize; 2]>::new();
    for e in b.attributed_entities() {
        let kind = match e {
            EntityRef::Face(_) => 0,
            EntityRef::Edge(_) => 1,
            _ => continue,
        };
        if let Some(n) = read(b, e) {
            for name in &n.0 {
                names.entry(name.clone()).or_default()[kind] += 1;
            }
        }
    }
    json!(names
        .into_iter()
        .map(|(id, [faces, edges])| (id, json!({"faces":faces,"edges":edges})))
        .collect::<BTreeMap<_, _>>())
}
