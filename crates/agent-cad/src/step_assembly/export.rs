//! [INPUT]: 固定neutral model与原STEP writer/生成序列化预算。
//! [OUTPUT]: surface/void-region STEP字节，保持原序列化行为。
//! [POS]: 从父模块移出的STEP输出适配；与上下文准入逻辑无耦合。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use super::*;
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
    let mut file = part21::parse(text.as_bytes(), &generated_limits()).map_err(failure)?;
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
    let mut file = part21::parse(text.as_bytes(), &generated_limits()).map_err(failure)?;
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
