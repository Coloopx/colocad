//! AP203/AP214 product instances with exact local B-rep definitions and rigid placements.
use crate::*;
use placement::Rigid;
use step_import::part21::{self, Instance as StepInstance, Param, Record, StepFile};
fn failure(e: impl std::fmt::Display) -> CadError {
    CadError::new("STEP_ASSEMBLY", e.to_string())
}
fn limits() -> step_import::Limits {
    step_import::Limits {
        max_entities: 100_000,
        max_list_depth: 24,
        max_ref_depth: 32,
        max_list_len: 16_384,
    }
}
fn quoted(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}
fn param(p: &Param, shift: u64) -> String {
    match p {
        Param::Int(n) => n.to_string(),
        Param::Real(n) => format!("{n:.17E}"),
        Param::Str(s) => quoted(s),
        Param::Enum(s) => format!(".{s}."),
        Param::Binary(s) => format!("\"{s}\""),
        Param::Ref(id) => format!("#{}", id + shift),
        Param::Null => "$".into(),
        Param::Derived => "*".into(),
        Param::List(v) => format!(
            "({})",
            v.iter()
                .map(|p| param(p, shift))
                .collect::<Vec<_>>()
                .join(",")
        ),
        Param::Typed(n, p) => format!("{n}({})", param(p, shift)),
    }
}
fn record(r: &Record, shift: u64) -> String {
    format!(
        "{}({})",
        r.name,
        r.params
            .iter()
            .map(|p| param(p, shift))
            .collect::<Vec<_>>()
            .join(",")
    )
}
struct Writer {
    items: Vec<String>,
}
impl Writer {
    fn add(&mut self, s: String) -> u64 {
        self.items.push(s);
        self.items.len() as u64
    }
    fn frame(&mut self, t: Rigid) -> u64 {
        let v = |p: Vec3| format!("({:.17E},{:.17E},{:.17E})", p.x, p.y, p.z);
        let p = self.add(format!("CARTESIAN_POINT('',{})", v(t.origin)));
        let z = self.add(format!("DIRECTION('',{})", v(t.axes[2])));
        let x = self.add(format!("DIRECTION('',{})", v(t.axes[0])));
        self.add(format!("AXIS2_PLACEMENT_3D('',#{p},#{z},#{x})"))
    }
    fn product(&mut self, name: &str, pc: u64, dc: u64, context: u64, identity: u64) -> (u64, u64) {
        let product = self.add(format!("PRODUCT({0},{0},'',(#{pc}))", quoted(name)));
        let form = self.add(format!("PRODUCT_DEFINITION_FORMATION('','',#{product})"));
        let pd = self.add(format!("PRODUCT_DEFINITION('design','',#{form},#{dc})"));
        let shape = self.add(format!("PRODUCT_DEFINITION_SHAPE('','',#{pd})"));
        let rep = self.add(format!(
            "SHAPE_REPRESENTATION({},(#{identity}),#{context})",
            quoted(name)
        ));
        self.add(format!("SHAPE_DEFINITION_REPRESENTATION(#{shape},#{rep})"));
        (pd, rep)
    }
    fn occurrence(
        &mut self,
        id: &str,
        parent: (u64, u64),
        child: (u64, u64),
        t: Rigid,
        identity: u64,
    ) {
        let frame = self.frame(t);
        let transform = self.add(format!(
            "ITEM_DEFINED_TRANSFORMATION('','',#{identity},#{frame})"
        ));
        let relation=self.add(format!("(REPRESENTATION_RELATIONSHIP('','',#{},#{}) REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#{transform}) SHAPE_REPRESENTATION_RELATIONSHIP())",child.1,parent.1));
        let use_id = self.add(format!(
            "NEXT_ASSEMBLY_USAGE_OCCURRENCE({0},{0},'',#{1},#{2},$)",
            quoted(id),
            parent.0,
            child.0
        ));
        let shape = self.add(format!(
            "PRODUCT_DEFINITION_SHAPE('Placement','',#{use_id})"
        ));
        self.add(format!(
            "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#{relation},#{shape})"
        ));
    }
}
pub(crate) fn write(
    m: &Model,
    defs: &BTreeMap<String, step_import::model::Model>,
    solved: &BTreeMap<String, Rigid>,
) -> Result<String> {
    let group_poses = placement::groups(m)?;
    let mut w = Writer { items: vec![] };
    let mut definitions = BTreeMap::new();
    let mut shared = None;
    for (name, body) in defs {
        let text = write_solids(body)?;
        let parsed = part21::parse(text.as_bytes(), &limits()).map_err(failure)?;
        let shift = w.items.len() as u64;
        let find = |ty| {
            parsed
                .entities
                .iter()
                .find(|(_, i)| i.record(ty).is_some())
                .map(|(id, _)| id + shift)
                .ok_or_else(|| failure(ty))
        };
        let pd = find("PRODUCT_DEFINITION")?;
        let rep = find("ADVANCED_BREP_SHAPE_REPRESENTATION")?;
        if shared.is_none() {
            shared = Some((
                find("PRODUCT_CONTEXT")?,
                find("PRODUCT_DEFINITION_CONTEXT")?,
                find("GEOMETRIC_REPRESENTATION_CONTEXT")?,
            ));
        }
        for (id, entity) in &parsed.entities {
            if *id != w.items.len() as u64 + 1 - shift {
                return Err(failure("non-contiguous generated entities"));
            }
            let text = match entity {
                StepInstance::Simple(r) if r.name == "PRODUCT" => {
                    let mut r = r.clone();
                    r.params[0] = Param::Str(name.clone());
                    r.params[1] = Param::Str(name.clone());
                    record(&r, shift)
                }
                StepInstance::Simple(r) => record(r, shift),
                StepInstance::Complex(rs) => format!(
                    "({})",
                    rs.iter()
                        .map(|r| record(r, shift))
                        .collect::<Vec<_>>()
                        .join(" ")
                ),
            };
            w.add(text);
        }
        definitions.insert(name.clone(), (pd, rep));
    }
    let (pc, dc, context) = shared.ok_or_else(|| failure("no exported definitions"))?;
    let identity = w.frame(Rigid::default());
    let root = w.product("agent-cad-assembly", pc, dc, context, identity);
    // Empty construction-only branches are omitted so they cannot become extra free roots.
    let mut used = BTreeSet::new();
    for i in m.assembly.iter().filter(|i| i.export) {
        let mut p = i.parent.as_ref();
        while let Some(id) = p {
            used.insert(id.clone());
            p = m
                .groups
                .iter()
                .find(|g| g.id == *id)
                .and_then(|g| g.parent.as_ref());
        }
    }
    let mut groups = BTreeMap::new();
    for g in m.groups.iter().filter(|g| used.contains(&g.id)) {
        groups.insert(g.id.clone(), w.product(&g.id, pc, dc, context, identity));
    }
    for g in m.groups.iter().filter(|g| used.contains(&g.id)) {
        let parent = g.parent.as_ref().map_or(root, |id| groups[id]);
        w.occurrence(
            &g.id,
            parent,
            groups[&g.id],
            g.pose.resolve(&m.parameters)?,
            identity,
        );
    }
    for i in m.assembly.iter().filter(|i| i.export) {
        let parent = i.parent.as_ref().map_or(root, |id| groups[id]);
        w.occurrence(
            &i.id,
            parent,
            definitions[&i.input],
            i.parent
                .as_ref()
                .map_or(Rigid::default(), |id| group_poses[id])
                .inverse()
                .compose(solved[&i.id]),
            identity,
        );
    }
    let mut out=String::from("ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('Exact B-rep product assembly'),'2;1');\nFILE_NAME('model.step','',(''),(''),'agent-cad','agent-cad','');\nFILE_SCHEMA(('CONFIG_CONTROL_DESIGN'));\nENDSEC;\nDATA;\n");
    for (i, item) in w.items.iter().enumerate() {
        out.push_str(&format!("#{}={item};\n", i + 1));
    }
    out.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    Ok(out)
}
fn rec<'a>(f: &'a StepFile, id: u64, ty: &str) -> Result<&'a Record> {
    f.entities
        .get(&id)
        .and_then(|v| v.record(ty))
        .ok_or_else(|| failure(format!("#{id} must be {ty}")))
}
fn reference(r: &Record, i: usize) -> Result<u64> {
    r.params
        .get(i)
        .and_then(Param::as_ref_id)
        .ok_or_else(|| failure(format!("{} missing reference {i}", r.name)))
}
fn vector(f: &StepFile, id: u64, ty: &str) -> Result<Vec3> {
    let r = rec(f, id, ty)?;
    let a = r
        .params
        .get(1)
        .and_then(Param::as_list)
        .ok_or_else(|| failure("3D vector required"))?;
    if a.len() != 3 {
        return Err(failure("3D vector required"));
    }
    let n = a
        .iter()
        .map(|p| {
            p.as_f64()
                .filter(|v| v.is_finite())
                .ok_or_else(|| failure("nonfinite vector"))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Vec3::new(n[0], n[1], n[2]))
}
fn axis(f: &StepFile, id: u64, scale: f64) -> Result<Rigid> {
    let r = rec(f, id, "AXIS2_PLACEMENT_3D")?;
    let origin = vector(f, reference(r, 1)?, "CARTESIAN_POINT")? * scale;
    let z = match r.params.get(2).and_then(Param::as_ref_id) {
        Some(id) => operations::unit(vector(f, id, "DIRECTION")?)?,
        None => Vec3::Z,
    };
    let x = match r.params.get(3).and_then(Param::as_ref_id) {
        Some(id) => vector(f, id, "DIRECTION")?,
        None => Vec3::X,
    };
    let x = operations::unit(x - z * z.dot(x))?;
    Ok(Rigid {
        origin,
        axes: [x, z.cross(x), z],
    })
}
fn mapping_transform(f: &StepFile, id: u64, scale: f64) -> Result<Rigid> {
    let item = f
        .entities
        .get(&id)
        .ok_or_else(|| failure("missing mapped transform"))?;
    if item
        .record("CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM")
        .is_some()
        || item
            .record("CARTESIAN_TRANSFORMATION_OPERATOR_3D_NON_UNIFORM")
            .is_some()
    {
        return Err(failure("non-uniform mapped scale is not a rigid placement"));
    }
    let three = rec(f, id, "CARTESIAN_TRANSFORMATION_OPERATOR_3D")?;
    let (base, offset, third) = if three.params.len() >= 6 {
        (three, 1, three.params.get(5))
    } else {
        let base = rec(f, id, "CARTESIAN_TRANSFORMATION_OPERATOR")?;
        (base, 0, three.params.first())
    };
    let direction = |p: Option<&Param>, default: Vec3| -> Result<Vec3> {
        match p.and_then(Param::as_ref_id) {
            Some(id) => operations::unit(vector(f, id, "DIRECTION")?),
            None => Ok(default),
        }
    };
    let x = direction(base.params.get(offset), Vec3::X)?;
    let y = direction(base.params.get(offset + 1), Vec3::Y)?;
    let z = direction(third, x.cross(y))?;
    let factor = match base.params.get(offset + 3) {
        Some(Param::Null) | None => 1.,
        Some(p) => p.as_f64().ok_or_else(|| failure("mapped scale"))?,
    };
    if !factor.is_finite()
        || (factor - 1.).abs() > 1e-12
        || x.dot(y).abs() > 1e-8
        || x.dot(z).abs() > 1e-8
        || y.dot(z).abs() > 1e-8
        || x.cross(y).dot(z) < 1. - 1e-8
    {
        return Err(failure(
            "mapped transform must be right-handed rigid, scale 1",
        ));
    }
    Ok(Rigid {
        axes: [x, y, z],
        origin: vector(f, reference(base, offset + 2)?, "CARTESIAN_POINT")? * scale,
    })
}
#[derive(Clone)]
pub(crate) struct ImportedInstance {
    pub id: String,
    pub body_index: usize,
    pub placement: Rigid,
    pub path: Vec<String>,
    pub parent: Option<String>,
}
pub(crate) struct ImportedGroup {
    pub id: String,
    pub parent: Option<String>,
    pub local: Rigid,
}
pub(crate) struct Imported {
    pub model: step_import::model::Model,
    pub instances: Vec<ImportedInstance>,
    pub groups: Vec<ImportedGroup>,
}
pub(crate) fn read(text: &str) -> Result<Imported> {
    let f = part21::parse(text.as_bytes(), &limits()).map_err(failure)?;
    let model = step_import::import(text.as_bytes(), &limits()).map_err(failure)?;
    if !model.skipped.is_empty()||model.diagnostics.iter().any(|d|d.step_id.is_some()||d.message!=format!("{} assembly instance(s) are counted but not applied; bodies are in their defining representation coordinates",model.assembly_instances)){return Err(CadError::new("IMPORT_INCOMPLETE",format!("skipped={:?}; diagnostics={:?}",model.skipped,model.diagnostics)));}
    // The geometry mapper has already converted lengths to mm. Apply the same file unit to placements.
    fn unit(f: &StepFile, id: u64, depth: usize) -> Result<f64> {
        if depth > 8 {
            return Err(failure("cyclic length unit"));
        }
        let item = f.entities.get(&id).ok_or_else(|| failure("missing unit"))?;
        if let Some(si) = item.record("SI_UNIT") {
            if si.params.get(1).and_then(Param::as_enum) != Some("METRE") {
                return Err(failure("length unit must resolve to metre"));
            }
            return Ok(match si.params.first() {
                Some(Param::Null) => 1000.,
                Some(Param::Enum(s)) => match s.as_str() {
                    "MILLI" => 1.,
                    "CENTI" => 10.,
                    "DECI" => 100.,
                    "KILO" => 1_000_000.,
                    "MICRO" => 0.001,
                    "NANO" => 0.000001,
                    _ => return Err(failure("unsupported SI prefix")),
                },
                _ => return Err(failure("invalid length unit")),
            });
        }
        if let Some(c) = item.record("CONVERSION_BASED_UNIT") {
            let measure = f
                .entities
                .get(&reference(c, 1)?)
                .ok_or_else(|| failure("unit conversion factor"))?;
            let r = measure
                .record("LENGTH_MEASURE_WITH_UNIT")
                .or_else(|| measure.record("MEASURE_WITH_UNIT"))
                .ok_or_else(|| failure("unit measure"))?;
            let value = r
                .params
                .first()
                .and_then(Param::as_f64)
                .filter(|v| v.is_finite() && *v > 0.)
                .ok_or_else(|| failure("invalid unit factor"))?;
            return Ok(value * unit(f, reference(r, 1)?, depth + 1)?);
        }
        Err(failure("unsupported length unit"))
    }
    let mut unit_scales = BTreeSet::new();
    for item in f.entities.values() {
        if let Some(context) = item.record("GLOBAL_UNIT_ASSIGNED_CONTEXT") {
            let units = context
                .params
                .first()
                .and_then(Param::as_list)
                .ok_or_else(|| failure("unit context"))?;
            for id in units.iter().filter_map(Param::as_ref_id) {
                if f.entities
                    .get(&id)
                    .is_some_and(|i| i.record("LENGTH_UNIT").is_some())
                {
                    unit_scales.insert(format!("{}", unit(&f, id, 0)?));
                }
            }
        }
    }
    if unit_scales.len() != 1 {
        return Err(failure("assembly requires one unambiguous length unit"));
    }
    let scale = unit_scales
        .iter()
        .next()
        .unwrap()
        .parse::<f64>()
        .map_err(failure)?;
    let body_map = model
        .bodies
        .iter()
        .enumerate()
        .map(|(i, b)| (b.step_id, i))
        .collect::<BTreeMap<_, _>>();
    let mut def_reps = BTreeMap::new();
    for item in f.entities.values() {
        if let Some(s) = item.record("SHAPE_DEFINITION_REPRESENTATION") {
            let shape = rec(&f, reference(s, 0)?, "PRODUCT_DEFINITION_SHAPE")?;
            let def = reference(shape, 2)?;
            if f.entities
                .get(&def)
                .is_some_and(|i| i.record("PRODUCT_DEFINITION").is_some())
            {
                def_reps
                    .entry(def)
                    .or_insert_with(Vec::new)
                    .push(reference(s, 1)?);
            }
        }
    }
    let mut placements = BTreeMap::new();
    for item in f.entities.values() {
        if let Some(c) = item.record("CONTEXT_DEPENDENT_SHAPE_REPRESENTATION") {
            let relation = reference(c, 0)?;
            let shape = rec(&f, reference(c, 1)?, "PRODUCT_DEFINITION_SHAPE")?;
            let occurrence = reference(shape, 2)?;
            let rr = f
                .entities
                .get(&relation)
                .ok_or_else(|| failure("missing representation relationship"))?;
            let transform = rr
                .record("REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION")
                .ok_or_else(|| failure("assembly relationship without transform"))?;
            let t = rec(&f, reference(transform, 0)?, "ITEM_DEFINED_TRANSFORMATION")?;
            let from = axis(&f, reference(t, 2)?, scale)?;
            let to = axis(&f, reference(t, 3)?, scale)?;
            placements.insert(occurrence, to.compose(from.inverse()));
        }
    }
    let mut children = BTreeMap::<u64, Vec<(u64, u64, Rigid)>>::new();
    let mut child_defs = BTreeSet::new();
    for (id, item) in &f.entities {
        if let Some(n) = item.record("NEXT_ASSEMBLY_USAGE_OCCURRENCE") {
            let parent = reference(n, 3)?;
            let child = reference(n, 4)?;
            let t = *placements
                .get(id)
                .ok_or_else(|| failure(format!("occurrence #{id} has no applied placement")))?;
            children.entry(parent).or_default().push((*id, child, t));
            child_defs.insert(child);
        }
    }
    #[derive(Clone)]
    struct Owned {
        body_index: usize,
        local: Rigid,
        maps: Vec<u64>,
    }
    struct Items<'a> {
        file: &'a StepFile,
        bodies: &'a BTreeMap<u64, usize>,
        scale: f64,
        mapped: BTreeSet<u64>,
    }
    impl Items<'_> {
        fn collect(
            &mut self,
            rep: u64,
            seen: &mut BTreeSet<u64>,
            ancestors: &mut BTreeSet<u64>,
            out: &mut Vec<Owned>,
        ) -> Result<()> {
            if !seen.insert(rep) {
                return Ok(());
            }
            if seen.len() > 256 || ancestors.len() > 32 {
                return Err(failure("representation traversal budget"));
            }
            let v = self
                .file
                .entities
                .get(&rep)
                .ok_or_else(|| failure("missing representation"))?;
            let records = match v {
                StepInstance::Simple(r) => vec![r],
                StepInstance::Complex(rs) => rs.iter().collect(),
            };
            let mut mapped_items = vec![];
            for r in records {
                if r.name.ends_with("SHAPE_REPRESENTATION") || r.name == "REPRESENTATION" {
                    let list = r
                        .params
                        .get(1)
                        .and_then(Param::as_list)
                        .ok_or_else(|| failure("representation item list"))?;
                    for p in list {
                        let id = p
                            .as_ref_id()
                            .ok_or_else(|| failure("representation item"))?;
                        if let Some(&body_index) = self.bodies.get(&id) {
                            out.push(Owned {
                                body_index,
                                local: Rigid::default(),
                                maps: vec![],
                            });
                        } else if self
                            .file
                            .entities
                            .get(&id)
                            .is_some_and(|v| v.record("MAPPED_ITEM").is_some())
                        {
                            mapped_items.push(id);
                        }
                    }
                }
            }
            for id in mapped_items {
                if !ancestors.insert(id) {
                    return Err(failure("cyclic mapped representation"));
                }
                self.mapped.insert(id);
                let mi = rec(self.file, id, "MAPPED_ITEM")?;
                let map = rec(self.file, reference(mi, 1)?, "REPRESENTATION_MAP")?;
                let source = axis(self.file, reference(map, 0)?, self.scale)?;
                let target = mapping_transform(self.file, reference(mi, 2)?, self.scale)?;
                let transform = target.compose(source.inverse());
                let mut contents = vec![];
                self.collect(
                    reference(map, 1)?,
                    &mut BTreeSet::new(),
                    ancestors,
                    &mut contents,
                )?;
                for mut body in contents {
                    body.local = transform.compose(body.local);
                    body.maps.insert(0, id);
                    out.push(body);
                }
                ancestors.remove(&id);
            }
            let mut related = vec![];
            for item in self.file.entities.values() {
                if item
                    .record("REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION")
                    .is_none()
                {
                    let rr = item
                        .record("REPRESENTATION_RELATIONSHIP")
                        .or_else(|| item.record("SHAPE_REPRESENTATION_RELATIONSHIP"));
                    if let Some(r) = rr.filter(|r| r.params.len() >= 4) {
                        let (a, b) = (reference(r, 2)?, reference(r, 3)?);
                        if a == rep {
                            related.push(b);
                        } else if b == rep {
                            related.push(a);
                        }
                    }
                }
            }
            for rep in related {
                self.collect(rep, seen, ancestors, out)?;
            }
            if out.len() > 128 {
                return Err(failure("at most 128 mapped instances per product"));
            }
            Ok(())
        }
    }
    struct Walk<'a> {
        children: &'a BTreeMap<u64, Vec<(u64, u64, Rigid)>>,
        owned: &'a BTreeMap<u64, Vec<Owned>>,
        out: Vec<ImportedInstance>,
        groups: Vec<ImportedGroup>,
    }
    impl Walk<'_> {
        fn visit(
            &mut self,
            id: u64,
            t: Rigid,
            path: Vec<String>,
            ancestors: &mut BTreeSet<u64>,
            parent: Option<String>,
            local: Rigid,
        ) -> Result<()> {
            if !ancestors.insert(id) || ancestors.len() > 32 {
                return Err(failure("cyclic or too deep product tree"));
            }
            if self.groups.len() >= 512 {
                return Err(failure("at most 512 imported groups"));
            }
            let group_id = format!("assembly-{}", self.groups.len());
            self.groups.push(ImportedGroup {
                id: group_id.clone(),
                parent,
                local,
            });
            if let Some(b) = self.owned.get(&id) {
                for entry in b {
                    let mut parent = group_id.clone();
                    let mut entry_path = path.clone();
                    if !entry.maps.is_empty() {
                        if self.groups.len() >= 512 {
                            return Err(failure("at most 512 imported groups"));
                        }
                        parent = format!("mapping-{}", self.groups.len());
                        self.groups.push(ImportedGroup {
                            id: parent.clone(),
                            parent: Some(group_id.clone()),
                            local: entry.local,
                        });
                        entry_path.extend(entry.maps.iter().map(|id| format!("mapped-{id}")));
                    }
                    if self.out.len() >= 128 {
                        return Err(failure("at most 128 imported instances"));
                    }
                    self.out.push(ImportedInstance {
                        id: format!("instance-{}", self.out.len()),
                        body_index: entry.body_index,
                        placement: t.compose(entry.local),
                        path: entry_path,
                        parent: Some(parent),
                    });
                }
            }
            if let Some(children) = self.children.get(&id) {
                for (oid, child, local) in children {
                    let mut path = path.clone();
                    path.push(format!("occurrence-{oid}"));
                    self.visit(
                        *child,
                        t.compose(*local),
                        path,
                        ancestors,
                        Some(group_id.clone()),
                        *local,
                    )?;
                }
            }
            ancestors.remove(&id);
            Ok(())
        }
    }
    let mut items = Items {
        file: &f,
        bodies: &body_map,
        scale,
        mapped: BTreeSet::new(),
    };
    let mut owned = BTreeMap::new();
    for (def, reps) in &def_reps {
        let mut bodies = vec![];
        let mut seen = BTreeSet::new();
        for rep in reps {
            items.collect(*rep, &mut seen, &mut BTreeSet::new(), &mut bodies)?;
        }
        owned.insert(*def, bodies);
    }
    if placements.len() + items.mapped.len() != model.assembly_instances {
        return Err(failure("unhandled or duplicate assembly transform"));
    }
    let mut walk = Walk {
        children: &children,
        owned: &owned,
        out: vec![],
        groups: vec![],
    };
    for root in def_reps.keys().filter(|id| !child_defs.contains(id)) {
        walk.visit(
            *root,
            Rigid::default(),
            vec![format!("product-{root}")],
            &mut BTreeSet::new(),
            None,
            Rigid::default(),
        )?;
    }
    if walk.out.is_empty() && children.is_empty() {
        for i in 0..model.bodies.len() {
            walk.out.push(ImportedInstance {
                id: format!("body-{i}"),
                body_index: i,
                placement: Rigid::default(),
                path: vec![],
                parent: None,
            });
        }
    }
    let covered = walk
        .out
        .iter()
        .map(|i| i.body_index)
        .collect::<BTreeSet<_>>();
    if covered.len() != model.bodies.len() {
        return Err(failure(
            "orphan geometry: product tree did not account for all bodies",
        ));
    }
    Ok(Imported {
        model,
        instances: walk.out,
        groups: walk.groups,
    })
}
pub(crate) fn build(k: &mut Kernel, text: &str) -> Result<Brep> {
    let imported = read(text)?;
    let mut result = None;
    for i in imported.instances {
        let mut b = exchange::build_body(k, &imported.model.bodies[i.body_index])?;
        i.placement.apply(k, &mut b)?;
        match &mut result {
            None => result = Some(b),
            Some(out) => compound::append(k, out, &b)?,
        }
    }
    result.ok_or_else(|| failure("empty assembly"))
}

/// Inspect a host-registered asset and emit a replayable, definition-sharing IR template.
/// The host must attach the same verified asset under `source` before building it.
pub fn inspect(asset: &StepAsset) -> Result<Value> {
    if asset.step.len() > 24 * 1024 * 1024 {
        return Err(CadError::new("RESOURCE_LIMIT", "STEP at most 24 MiB"));
    }
    if format!("{:x}", Sha256::digest(asset.step.as_bytes())) != asset.sha256 {
        return Err(CadError::new("ASSET_DIGEST_MISMATCH", "source"));
    }
    let data = read(&asset.step)?;
    let frame = |t: Rigid| json!({"origin":[t.origin.x,t.origin.y,t.origin.z],"x_axis":[t.axes[0].x,t.axes[0].y,t.axes[0].z],"y_axis":[t.axes[1].x,t.axes[1].y,t.axes[1].z]});
    let features=(0..data.model.bodies.len()).map(|i|json!({"id":format!("definition-{i}"),"op":"import_step","asset":"source","body_index":i,"definition":true})).collect::<Vec<_>>();
    let groups = data
        .groups
        .iter()
        .map(|g| json!({"id":g.id,"parent":g.parent,"pose":{"frame":frame(g.local)}}))
        .collect::<Vec<_>>();
    let instances=data.instances.iter().map(|i|json!({"id":i.id,"input":format!("definition-{}",i.body_index),"parent":i.parent,"metadata":{"step_path":i.path}})).collect::<Vec<_>>();
    Ok(
        json!({"asset_sha256":asset.sha256,"definitions":data.model.bodies.len(),"instances":data.instances.iter().map(|i|json!({"id":i.id,"body_index":i.body_index,"path":i.path,"world_placement":i.placement.json()})).collect::<Vec<_>>(),"model_template":{"version":1,"units":"mm","parameters":{},"features":features,"groups":groups,"assembly":instances},"required_asset_id":"source"}),
    )
}

/// Serialize sheets as STEP surface models, preserving the exact trimmed surfaces.
pub(crate) fn write_surface(model: &step_import::model::Model) -> Result<String> {
    let mut source = model.clone();
    for b in &mut source.bodies {
        b.kind = step_import::model::BodyKind::ManifoldSolid;
        for shell in &mut b.shells {
            shell.closed = true;
            // A sheet has two faceuses of one face; STEP stores that face once.
            let mut seen = BTreeSet::new();
            shell.faces.retain(|face| seen.insert(*face));
        }
    }
    let text = step_export::write(&source).map_err(failure)?;
    let mut file = part21::parse(text.as_bytes(), &limits()).map_err(failure)?;
    for item in file.entities.values_mut() {
        if let StepInstance::Simple(r) = item {
            match r.name.as_str() {
                "CLOSED_SHELL" => r.name = "OPEN_SHELL".into(),
                "MANIFOLD_SOLID_BREP" => {
                    r.name = "SHELL_BASED_SURFACE_MODEL".into();
                    r.params[1] = Param::List(vec![r.params[1].clone()]);
                }
                "ADVANCED_BREP_SHAPE_REPRESENTATION" => {
                    r.name = "MANIFOLD_SURFACE_SHAPE_REPRESENTATION".into()
                }
                _ => {}
            }
        }
    }
    let mut out=String::from("ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('Exact surface model'),'2;1');\nFILE_NAME('surface.step','',(''),(''),'agent-cad','agent-cad','');\nFILE_SCHEMA(('CONFIG_CONTROL_DESIGN'));\nENDSEC;\nDATA;\n");
    for (id, item) in &file.entities {
        let value = match item {
            StepInstance::Simple(r) => record(r, 0),
            StepInstance::Complex(rs) => format!(
                "({})",
                rs.iter()
                    .map(|r| record(r, 0))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
        };
        out.push_str(&format!("#{id}={value};\n"));
    }
    out.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    Ok(out)
}

/// Preserve one material solid across its outer shell and enclosed cavity shells.
pub(crate) fn write_solids(model: &step_import::model::Model) -> Result<String> {
    if model.bodies.iter().all(|b| b.shells.len() == 1) {
        return step_export::write(model).map_err(failure);
    }
    let mut flat = model.clone();
    flat.bodies.clear();
    let mut groups = vec![];
    for body in &model.bodies {
        let start = flat.bodies.len();
        for (index, shell) in body.shells.iter().enumerate() {
            let mut part = body.clone();
            if index > 0 {
                // STEP requires a positive cavity shell, reversed by its oriented shell.
                for &face in &shell.faces {
                    part.faces[face].same_sense = !part.faces[face].same_sense;
                    for bound in &mut part.faces[face].loops {
                        bound.orientation = !bound.orientation;
                    }
                }
            }
            part.kind = step_import::model::BodyKind::ManifoldSolid;
            part.step_id = flat.bodies.len() as u64 + 1;
            part.shells = vec![shell.clone()];
            flat.bodies.push(part);
        }
        groups.push(start..flat.bodies.len());
    }
    let text = step_export::write(&flat).map_err(failure)?;
    let mut file = part21::parse(text.as_bytes(), &limits()).map_err(failure)?;
    let solids = file
        .entities
        .iter()
        .filter_map(|(id, v)| v.record("MANIFOLD_SOLID_BREP").map(|_| *id))
        .collect::<Vec<_>>();
    if solids.len() != flat.bodies.len() {
        return Err(failure("shell export count mismatch"));
    }
    let mut inner = BTreeSet::new();
    for group in groups.iter().filter(|g| g.len() > 1) {
        let voids = group
            .clone()
            .skip(1)
            .map(|i| Param::Ref(solids[i]))
            .collect::<Vec<_>>();
        if let Some(StepInstance::Simple(r)) = file.entities.get_mut(&solids[group.start]) {
            r.name = "BREP_WITH_VOIDS".into();
            r.params.push(Param::List(voids));
        }
        for i in group.clone().skip(1) {
            inner.insert(solids[i]);
            if let Some(StepInstance::Simple(r)) = file.entities.get_mut(&solids[i]) {
                let shell = r.params[1].clone();
                r.name = "ORIENTED_CLOSED_SHELL".into();
                // Positive cavity shell reversed toward the void (ISO 10303-42).
                r.params = vec![
                    Param::Str(String::new()),
                    Param::Derived,
                    shell,
                    Param::Enum("F".into()),
                ];
            }
        }
    }
    for item in file.entities.values_mut() {
        if let StepInstance::Simple(r) = item {
            if r.name == "ADVANCED_BREP_SHAPE_REPRESENTATION" {
                if let Some(Param::List(items)) = r.params.get_mut(1) {
                    items.retain(|v| v.as_ref_id().is_none_or(|id| !inner.contains(&id)));
                }
            }
        }
    }
    let mut out=String::from("ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('Exact solids with enclosed cavities'),'2;1');\nFILE_NAME('model.step','',(''),(''),'agent-cad','agent-cad','');\nFILE_SCHEMA(('CONFIG_CONTROL_DESIGN'));\nENDSEC;\nDATA;\n");
    for (id, item) in &file.entities {
        let value = match item {
            StepInstance::Simple(r) => record(r, 0),
            StepInstance::Complex(rs) => format!(
                "({})",
                rs.iter()
                    .map(|r| record(r, 0))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
        };
        out.push_str(&format!("#{id}={value};\n"));
    }
    out.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    Ok(out)
}
