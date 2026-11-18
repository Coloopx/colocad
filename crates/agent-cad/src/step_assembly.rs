//! [INPUT]: 外部 STEP 实体/定义预算、显式 body 上下文与独立宿主输出字节预算。
//! [OUTPUT]: 本地定义、逐表示归一的产品树位姿与预算约束下的 STEP。
//! [POS]: 定义和装配共用解析、映射与上下文校验；整树另查 occurrence/group。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
mod export;
mod units;
pub(crate) use export::{write_solids, write_surface};
mod transforms;
use crate::*;
use placement::Rigid;
use step_import::part21::{self, Instance as StepInstance, Param, Record, StepFile};
use transforms::{axis, mapping_transform};
fn failure(e: impl std::fmt::Display) -> CadError {
    CadError::new("STEP_ASSEMBLY", e.to_string())
}
#[cfg(test)]
fn limits() -> step_import::Limits {
    step_import_limits::current().parser()
}
// Internal serialization is bounded separately from untrusted imported assets.
// A valid lofted screw can exceed 100k STEP records before it is instanced.
fn generated_limits() -> step_import::Limits {
    step_import::Limits {
        max_entities: 1_000_000,
        ..step_import_limits::StepImportLimits::default().parser()
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
const STEP_HEADER: &str = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('Exact B-rep product assembly'),'2;1');\nFILE_NAME('model.step','',(''),(''),'agent-cad','agent-cad','');\nFILE_SCHEMA(('CONFIG_CONTROL_DESIGN'));\nENDSEC;\nDATA;\n";
const STEP_END: &str = "ENDSEC;\nEND-ISO-10303-21;\n";

struct Writer {
    output: String,
    count: u64,
    limit: usize,
}
impl Writer {
    fn add(&mut self, entity: String) -> Result<u64> {
        use std::fmt::Write;
        let id = self.count + 1;
        let digits = id.ilog10() as usize + 1;
        // ---------- 完整语法与尾部计入字节，尚未生成实体仅报告下界 ----------
        let observed = self.output.len() + digits + 4 + entity.len() + STEP_END.len();
        resources::check_bytes("outputStepBytes", observed, self.limit, true)?;
        writeln!(&mut self.output, "#{id}={entity};").expect("writing to String");
        self.count = id;
        Ok(id)
    }
    fn frame(&mut self, t: Rigid) -> Result<u64> {
        let v = |p: Vec3| format!("({:.17E},{:.17E},{:.17E})", p.x, p.y, p.z);
        let p = self.add(format!("CARTESIAN_POINT('',{})", v(t.origin)))?;
        let z = self.add(format!("DIRECTION('',{})", v(t.axes[2])))?;
        let x = self.add(format!("DIRECTION('',{})", v(t.axes[0])))?;
        self.add(format!("AXIS2_PLACEMENT_3D('',#{p},#{z},#{x})"))
    }
    fn product(
        &mut self,
        name: &str,
        pc: u64,
        dc: u64,
        context: u64,
        identity: u64,
    ) -> Result<(u64, u64)> {
        let product = self.add(format!("PRODUCT({0},{0},'',(#{pc}))", quoted(name)))?;
        let form = self.add(format!("PRODUCT_DEFINITION_FORMATION('','',#{product})"))?;
        let pd = self.add(format!("PRODUCT_DEFINITION('design','',#{form},#{dc})"))?;
        let shape = self.add(format!("PRODUCT_DEFINITION_SHAPE('','',#{pd})"))?;
        let rep = self.add(format!(
            "SHAPE_REPRESENTATION({},(#{identity}),#{context})",
            quoted(name)
        ))?;
        self.add(format!("SHAPE_DEFINITION_REPRESENTATION(#{shape},#{rep})"))?;
        Ok((pd, rep))
    }
    fn occurrence(
        &mut self,
        id: &str,
        parent: (u64, u64),
        child: (u64, u64),
        t: Rigid,
        identity: u64,
    ) -> Result<()> {
        let frame = self.frame(t)?;
        let transform = self.add(format!(
            "ITEM_DEFINED_TRANSFORMATION('','',#{identity},#{frame})"
        ))?;
        let relation=self.add(format!("(REPRESENTATION_RELATIONSHIP('','',#{},#{}) REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#{transform}) SHAPE_REPRESENTATION_RELATIONSHIP())",child.1,parent.1))?;
        let use_id = self.add(format!(
            "NEXT_ASSEMBLY_USAGE_OCCURRENCE({0},{0},'',#{1},#{2},$)",
            quoted(id),
            parent.0,
            child.0
        ))?;
        let shape = self.add(format!(
            "PRODUCT_DEFINITION_SHAPE('Placement','',#{use_id})"
        ))?;
        self.add(format!(
            "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#{relation},#{shape})"
        ))?;
        Ok(())
    }
}
pub(crate) fn write(
    m: &Model,
    defs: BTreeMap<String, String>,
    solved: &BTreeMap<String, Rigid>,
    limit: usize,
) -> Result<String> {
    resources::check_bytes("outputStepBytes", STEP_HEADER.len() + STEP_END.len(), limit, true)?;
    let group_poses = placement::groups(m)?;
    let mut w = Writer {
        output: String::from(STEP_HEADER),
        count: 0,
        limit,
    };
    let mut definitions = BTreeMap::new();
    let mut shared = None;
    for (name, text) in defs {
        let parsed = part21::parse(text.as_bytes(), &generated_limits()).map_err(failure)?;
        drop(text);
        let shift = w.count;
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
        for (id, entity) in parsed.entities {
            if id != w.count + 1 - shift {
                return Err(failure("non-contiguous generated entities"));
            }
            let text = match entity {
                StepInstance::Simple(mut r) if r.name == "PRODUCT" => {
                    r.params[0] = Param::Str(name.clone());
                    r.params[1] = Param::Str(name.clone());
                    record(&r, shift)
                }
                StepInstance::Simple(r) => record(&r, shift),
                StepInstance::Complex(rs) => format!(
                    "({})",
                    rs.iter()
                        .map(|r| record(r, shift))
                        .collect::<Vec<_>>()
                        .join(" ")
                ),
            };
            w.add(text)?;
        }
        definitions.insert(name, (pd, rep));
    }
    let (pc, dc, context) = shared.ok_or_else(|| failure("no exported definitions"))?;
    let identity = w.frame(Rigid::default())?;
    let root = w.product("agent-cad-assembly", pc, dc, context, identity)?;
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
        groups.insert(g.id.clone(), w.product(&g.id, pc, dc, context, identity)?);
    }
    for g in m.groups.iter().filter(|g| used.contains(&g.id)) {
        let parent = g.parent.as_ref().map_or(root, |id| groups[id]);
        w.occurrence(
            &g.id,
            parent,
            groups[&g.id],
            g.pose.resolve(&m.parameters)?,
            identity,
        )?;
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
        )?;
    }
    w.output.push_str(STEP_END);
    Ok(w.output)
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
    pub source_entities: usize,
    pub model: step_import::model::Model,
    pub instances: Vec<ImportedInstance>,
    pub groups: Vec<ImportedGroup>,
}
fn definitions(text: &str) -> Result<(StepFile, step_import::model::Model, units::Contexts)> {
    let file = step_import_limits::parse_external(text)?;
    let model = step_import_limits::map_external(&file, text)?;
    let contexts = units::Contexts::validate(&file, &model)?;
    Ok((file, model, contexts))
}

pub(crate) fn read_definitions(text: &str) -> Result<step_import::model::Model> {
    definitions(text).map(|(_, model, _)| model)
}

pub(crate) fn read(text: &str) -> Result<Imported> {
    let (f, model, contexts) = definitions(text)?;
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
            let relation = rr
                .record("REPRESENTATION_RELATIONSHIP")
                .or_else(|| rr.record("SHAPE_REPRESENTATION_RELATIONSHIP"))
                .ok_or_else(|| failure("missing relationship endpoints"))?;
            let (source_rep, target_rep) = (reference(relation, 2)?, reference(relation, 3)?);
            let from = axis(&f, reference(t, 2)?, contexts.scale(&f, source_rep)?)?;
            let to = axis(&f, reference(t, 3)?, contexts.scale(&f, target_rep)?)?;
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
        contexts: &'a units::Contexts,
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
                            step_import_limits::check(
                                "step_occurrences",
                                out.len() + 1,
                                step_import_limits::current().max_occurrences,
                                "mapped_product",
                            )?;
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
                let mapped_rep = reference(map, 1)?;
                let source = axis(
                    self.file,
                    reference(map, 0)?,
                    self.contexts.scale(self.file, mapped_rep)?,
                )?;
                let target = mapping_transform(
                    self.file,
                    reference(mi, 2)?,
                    self.contexts.scale(self.file, rep)?,
                )?;
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
                    step_import_limits::check(
                        "step_occurrences",
                        out.len() + 1,
                        step_import_limits::current().max_occurrences,
                        "mapped_product",
                    )?;
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
            step_import_limits::check(
                "step_occurrences",
                out.len(),
                step_import_limits::current().max_occurrences,
                "mapped_product",
            )?;
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
            step_import_limits::check(
                "step_groups",
                self.groups.len() + 1,
                step_import_limits::current().max_groups,
                "product_walk",
            )?;
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
                        step_import_limits::check(
                            "step_groups",
                            self.groups.len() + 1,
                            step_import_limits::current().max_groups,
                            "mapped_walk",
                        )?;
                        parent = format!("mapping-{}", self.groups.len());
                        self.groups.push(ImportedGroup {
                            id: parent.clone(),
                            parent: Some(group_id.clone()),
                            local: entry.local,
                        });
                        entry_path.extend(entry.maps.iter().map(|id| format!("mapped-{id}")));
                    }
                    step_import_limits::check(
                        "step_occurrences",
                        self.out.len() + 1,
                        step_import_limits::current().max_occurrences,
                        "product_walk",
                    )?;
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
        contexts: &contexts,
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
    step_import_limits::check(
        "step_occurrences",
        walk.out.len(),
        step_import_limits::current().max_occurrences,
        "complete_source",
    )?;
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
        source_entities: f.entities.len(),
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
pub fn inspect_with_import_limits(asset: &StepAsset, limits: StepImportLimits) -> Result<Value> {
    step_import_limits::with_import_limits(limits, || inspect(asset))
}
pub fn inspect(asset: &StepAsset) -> Result<Value> {
    inspect_with_resource_limits(asset, &ResourceLimits::default())
}
pub fn inspect_with_resource_limits(asset: &StepAsset, limits: &ResourceLimits) -> Result<Value> {
    limits.validate()?;
    resources::check_bytes("stepBytes", asset.step.len(), limits.step_bytes, false)?;
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
    Ok(json!({"step_import_limits":step_import_limits::current(),"source_entities":data.source_entities,"asset_sha256":asset.sha256,"definitions":data.model.bodies.len(),"instances":data.instances.iter().map(|i|json!({"id":i.id,"body_index":i.body_index,"path":i.path,"world_placement":i.placement.json()})).collect::<Vec<_>>(),"model_template":{"version":1,"units":"mm","parameters":{},"features":features,"groups":groups,"assembly":instances},"required_asset_id":"source",
        "resource_limits":limits,"resource_usage":{"step_bytes":asset.step.len()}}))
}

#[cfg(test)]
mod export_limits_tests {
    use super::*;
    #[test]
    fn generated_definitions_can_exceed_external_import_budget() {
        let mut text = String::from("ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\n");
        for id in 1..=100_001 {
            text.push_str(&format!("#{id}=CARTESIAN_POINT('',(0.,0.,0.));\n"));
        }
        text.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
        assert!(part21::parse(text.as_bytes(), &limits()).is_err());
        assert_eq!(
            part21::parse(text.as_bytes(), &generated_limits())
                .unwrap()
                .entities
                .len(),
            100_001
        );
    }
}
