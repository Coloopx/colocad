//! [INPUT]: 原Part21 representation/item归属、引用关系与mapper的mm化body。
//! [OUTPUT]: body单位覆盖、几何/位姿context一致性及纯描述元数据隔离。
//! [POS]: STEP边界的上下文证据；不改vendor，不修几何/容差/预算。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use super::*;
use step_import::model::Model as NeutralModel;

fn incomplete(message: impl Into<String>) -> CadError {
    CadError::new("IMPORT_INCOMPLETE", message)
}
fn representation(item: &StepInstance) -> Option<&Record> {
    match item {
        StepInstance::Simple(r) if r.name.ends_with("REPRESENTATION") => Some(r),
        StepInstance::Complex(rs) => rs.iter().find(|r| r.name == "REPRESENTATION"),
        _ => None,
    }
}
// ---- 仅无入边的纯描述representation可免长度；维数不是豁免依据 ----
fn record_owners(
    param: &Param,
    owner: u64,
    targets: &BTreeSet<u64>,
    owners: &mut BTreeMap<u64, BTreeSet<u64>>,
) {
    match param {
        Param::Ref(id) if targets.contains(id) => {
            owners.entry(*id).or_default().insert(owner);
        }
        Param::List(items) => {
            for item in items {
                record_owners(item, owner, targets, owners);
            }
        }
        Param::Typed(_, value) => record_owners(value, owner, targets, owners),
        _ => {}
    }
}

fn reference_owners(f: &StepFile, targets: &BTreeSet<u64>) -> BTreeMap<u64, BTreeSet<u64>> {
    let mut owners = BTreeMap::new();
    for (&owner, item) in &f.entities {
        let records = match item {
            StepInstance::Simple(record) => std::slice::from_ref(record),
            StepInstance::Complex(records) => records,
        };
        for record in records {
            for param in &record.params {
                record_owners(param, owner, targets, &mut owners);
            }
        }
    }
    owners
}
fn descriptive_metadata(f: &StepFile, id: u64, owners: &BTreeMap<u64, BTreeSet<u64>>) -> bool {
    if owners.contains_key(&id) {
        return false;
    }
    let Some(StepInstance::Simple(r)) = f.entities.get(&id) else {
        return false;
    };
    if r.name != "REPRESENTATION" || r.params.len() != 3 {
        return false;
    }
    let Some(items) = r.params.get(1).and_then(Param::as_list) else {
        return false;
    };
    !items.is_empty() && items.iter().all(|p| {
        let Some(item_id) = p.as_ref_id() else { return false; };
        owners.get(&item_id).is_some_and(|users| users.len() == 1 && users.contains(&id))
            && matches!(f.entities.get(&item_id),
                Some(StepInstance::Simple(item)) if item.name == "DESCRIPTIVE_REPRESENTATION_ITEM"
                    && item.params.len() == 2 && item.params.iter().all(|p| matches!(p, Param::Str(_))))
    })
}
#[derive(Clone, Copy, PartialEq)]
struct UnitContext {
    mm: f64,
    degrees: f64,
    uncertainty: Option<f64>,
}

// ---- 单位因子仅描述来源尺度，不是几何容差 ----
fn factor(f: &StepFile, id: u64, depth: usize) -> Result<(char, f64)> {
    if depth > 8 {
        return Err(incomplete("cyclic or too deep unit conversion"));
    }
    let item = f
        .entities
        .get(&id)
        .ok_or_else(|| incomplete("missing unit"))?;
    let kind = if item.record("LENGTH_UNIT").is_some() {
        'L'
    } else if item.record("PLANE_ANGLE_UNIT").is_some() {
        'A'
    } else {
        return Ok(('?', 1.));
    };
    let value = if let Some(si) = item.record("SI_UNIT") {
        let base = match (kind, si.params.get(1).and_then(Param::as_enum)) {
            ('L', Some("METRE")) => 1000.,
            ('A', Some("RADIAN")) => 180. / std::f64::consts::PI,
            _ => return Err(incomplete("unit kind does not match SI measure")),
        };
        let prefix = match si.params.first() {
            Some(Param::Null) => 1.,
            Some(Param::Enum(p)) => match p.as_str() {
                "EXA" => 1e18,
                "PETA" => 1e15,
                "TERA" => 1e12,
                "GIGA" => 1e9,
                "MEGA" => 1e6,
                "KILO" => 1e3,
                "HECTO" => 1e2,
                "DECA" => 1e1,
                "DECI" => 1e-1,
                "CENTI" => 1e-2,
                "MILLI" => 1e-3,
                "MICRO" => 1e-6,
                "NANO" => 1e-9,
                "PICO" => 1e-12,
                _ => return Err(incomplete("unsupported SI prefix")),
            },
            _ => return Err(incomplete("invalid SI prefix")),
        };
        base * prefix
    } else if let Some(cb) = item.record("CONVERSION_BASED_UNIT") {
        let measure = f
            .entities
            .get(&reference(cb, 1)?)
            .ok_or_else(|| incomplete("missing unit measure"))?;
        let measure = measure
            .record("LENGTH_MEASURE_WITH_UNIT")
            .or_else(|| measure.record("PLANE_ANGLE_MEASURE_WITH_UNIT"))
            .or_else(|| measure.record("MEASURE_WITH_UNIT"))
            .ok_or_else(|| incomplete("unsupported unit measure"))?;
        let value = measure
            .params
            .first()
            .and_then(Param::as_f64)
            .ok_or_else(|| incomplete("bad unit factor"))?;
        let (base_kind, base) = factor(f, reference(measure, 1)?, depth + 1)?;
        if base_kind != kind {
            return Err(incomplete("conversion unit has another kind"));
        }
        value * base
    } else {
        return Err(incomplete("unsupported unit definition"));
    };
    if !value.is_finite() || value <= 0. {
        return Err(incomplete("unit factor must be finite and positive"));
    }
    Ok((kind, value))
}
fn context(f: &StepFile, id: u64) -> Result<UnitContext> {
    let item = f
        .entities
        .get(&id)
        .ok_or_else(|| incomplete("missing representation context"))?;
    let guc = item
        .record("GLOBAL_UNIT_ASSIGNED_CONTEXT")
        .ok_or_else(|| incomplete("representation has no explicit units"))?;
    let list = guc
        .params
        .first()
        .and_then(Param::as_list)
        .ok_or_else(|| incomplete("invalid unit context"))?;
    let (mut lengths, mut angles) = (vec![], vec![]);
    for unit in list {
        let (kind, value) = factor(
            f,
            unit.as_ref_id()
                .ok_or_else(|| incomplete("unit reference required"))?,
            0,
        )?;
        match kind {
            'L' => lengths.push(value),
            'A' => angles.push(value),
            _ => {}
        }
    }
    if lengths.len() != 1 || angles.len() > 1 {
        return Err(incomplete(
            "context must have one explicit length unit and at most one angle unit",
        ));
    }
    let mut units = UnitContext {
        mm: lengths[0],
        degrees: angles
            .first()
            .copied()
            .unwrap_or(180. / std::f64::consts::PI),
        uncertainty: None,
    };
    if let Some(g) = item.record("GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT") {
        let values = g
            .params
            .first()
            .and_then(Param::as_list)
            .ok_or_else(|| incomplete("invalid uncertainty context"))?;
        for p in values {
            let measure = rec(
                f,
                p.as_ref_id()
                    .ok_or_else(|| incomplete("uncertainty reference required"))?,
                "UNCERTAINTY_MEASURE_WITH_UNIT",
            )?;
            let (kind, scale) = factor(f, reference(measure, 1)?, 0)?;
            if kind == 'L' {
                let value = measure
                    .params
                    .first()
                    .and_then(Param::as_f64)
                    .ok_or_else(|| incomplete("invalid uncertainty"))?
                    * scale;
                if !value.is_finite() || value < 0. {
                    return Err(incomplete("invalid uncertainty"));
                }
                units.uncertainty = Some(value);
            }
        }
    }
    Ok(units)
}

fn verify_length_contexts(f: &StepFile, contexts: &Contexts, declared: &[u64]) -> Result<()> {
    // 单一有效的长度上下文不会产生混合尺度，且每个 body 已验证显式归属。
    // 单一但声明不合法的情形仍须区分未引用声明和实际消费者。
    if declared.is_empty()
        || (declared.len() == 1 && declared.first().is_some_and(|id| context(f, *id).is_ok()))
    {
        return Ok(());
    }
    let targets = declared
        .iter()
        .chain(contexts.reps.keys())
        .chain(contexts.owners.keys())
        .copied()
        .collect::<BTreeSet<_>>();
    let owners = reference_owners(f, &targets);
    let metadata = contexts
        .reps
        .keys()
        .filter(|id| descriptive_metadata(f, **id, &owners))
        .copied()
        .collect::<BTreeSet<_>>();
    let mut length = None;
    for &id in declared {
        // 未使用的声明无几何含义；context/item/rep的任何未知入边均保守保留。
        if !owners
            .get(&id)
            .is_some_and(|users| users.iter().any(|rep| !metadata.contains(rep)))
        {
            continue;
        }
        let mm = context(f, id)?.mm;
        if length.is_some_and(|prior| prior != mm) {
            return Err(incomplete("mixed length contexts are not supported"));
        }
        length = Some(mm);
    }
    Ok(())
}

pub(super) struct Contexts {
    reps: BTreeMap<u64, u64>,
    owners: BTreeMap<u64, Vec<u64>>,
}
impl Contexts {
    pub(super) fn validate(f: &StepFile, model: &NeutralModel) -> Result<Self> {
        let mut out = Self {
            reps: BTreeMap::new(),
            owners: BTreeMap::new(),
        };
        let mut declared = Vec::new();
        for (id, item) in &f.entities {
            if item.record("GLOBAL_UNIT_ASSIGNED_CONTEXT").is_some() {
                declared.push(*id);
            }
            if let Some(r) = representation(item) {
                if let (Some(items), Some(ctx)) = (
                    r.params.get(1).and_then(Param::as_list),
                    r.params.get(2).and_then(Param::as_ref_id),
                ) {
                    out.reps.insert(*id, ctx);
                    for item in items.iter().filter_map(Param::as_ref_id) {
                        out.owners.entry(item).or_default().push(*id);
                    }
                }
            }
        }
        // mapper已按body所属context归一；只在逐body证实后接受全局信息性诊断。
        for body in &model.bodies {
            let reps = out.owners.get(&body.step_id).ok_or_else(|| {
                incomplete(format!(
                    "body #{} has no explicit representation context",
                    body.step_id
                ))
            })?;
            let mut expected = None;
            for rep in reps {
                let unit = context(f, out.reps[rep])?;
                if expected.is_some_and(|prior| prior != unit) {
                    return Err(incomplete(format!(
                        "body #{} has conflicting representation contexts",
                        body.step_id
                    )));
                }
                if body.unit_mm != unit.mm || body.uncertainty_mm != unit.uncertainty {
                    return Err(incomplete(format!(
                        "body #{} normalization does not match its context",
                        body.step_id
                    )));
                }
                expected = Some(unit);
            }
        }
        verify_length_contexts(f, &out, &declared)?;
        let counted=format!("{} assembly instance(s) are counted but not applied; bodies are in their defining representation coordinates",model.assembly_instances);
        let summary=" distinct unit contexts; using the default fallback for bodies without an explicit context";
        let accepted = |d: &step_import::model::Diagnostic| {
            d.step_id.is_none()
                && (d.message == counted
                    || d.message
                        .strip_suffix(summary)
                        .and_then(|n| n.parse::<usize>().ok())
                        .is_some_and(|n| n > 1))
        };
        if !model.skipped.is_empty() || model.diagnostics.iter().any(|d| !accepted(d)) {
            return Err(incomplete(format!(
                "skipped={:?}; diagnostics={:?}",
                model.skipped, model.diagnostics
            )));
        }
        Ok(out)
    }
    pub(super) fn scale(&self, f: &StepFile, rep: u64) -> Result<f64> {
        let ctx = self
            .reps
            .get(&rep)
            .ok_or_else(|| incomplete(format!("representation #{rep} has no explicit context")))?;
        Ok(context(f, *ctx)?.mm)
    }
}
