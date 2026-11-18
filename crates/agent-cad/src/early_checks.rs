//! [INPUT]: 已构造的不可变定义与无mate的固定实例位姿。
//! [OUTPUT]: 依赖就绪即执行的原精确碰撞检查；通过记录被最终装配复用。
//! [POS]: 特征执行与装配收口之间的检查调度，不放宽几何门限。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use crate::*;
#[derive(Default)]
pub(crate) struct EarlyChecks {
    pub findings: BTreeMap<(usize, usize, usize), Value>,
}
impl EarlyChecks {
    pub fn evaluate(
        &mut self,
        k: &mut Kernel,
        m: &Model,
        bodies: &BTreeMap<String, Brep>,
        feature: &str,
    ) -> Result<()> {
        // Mate求解可能改变最终位姿；该类检查继续在装配收口执行。
        if m.mates.is_some() {
            return Ok(());
        }
        let groups = placement::groups(m)?;
        for (index, check) in m.checks.iter().enumerate() {
            let AssemblyCheck::Collision {
                parts,
                allowed_pairs,
                max_overlap_mm3,
            } = check
            else {
                continue;
            };
            for i in 0..parts.len() {
                for j in i + 1..parts.len() {
                    if self.findings.contains_key(&(index, i, j)) {
                        continue;
                    }
                    let a = m
                        .assembly
                        .iter()
                        .find(|p| p.id == parts[i])
                        .expect("validated part");
                    let b = m
                        .assembly
                        .iter()
                        .find(|p| p.id == parts[j])
                        .expect("validated part");
                    let (Some(a_body), Some(b_body)) = (bodies.get(&a.input), bodies.get(&b.input))
                    else {
                        continue;
                    };
                    crate::progress::emit("check", "started", Some(feature));
                    let mut va = solidcore::topology::brep::p3::brep_copy(k, a_body)?;
                    let mut vb = solidcore::topology::brep::p3::brep_copy(k, b_body)?;
                    placement::world(m, a, placement::PoseMode::Normal, &groups)?
                        .0
                        .apply(k, &mut va)?;
                    placement::world(m, b, placement::PoseMode::Normal, &groups)?
                        .0
                        .apply(k, &mut vb)?;
                    let allowed = allowed_pairs
                        .iter()
                        .any(|p| (p[0] == a.id && p[1] == b.id) || (p[0] == b.id && p[1] == a.id));
                    let finding = collision_finding(
                        k,
                        &va,
                        &vb,
                        &a.id,
                        &b.id,
                        allowed,
                        max_overlap_mm3.resolve(&m.parameters)?,
                    )
                    .map_err(|mut e| {
                        e.feature_id = Some(feature.to_string());
                        if let Some(d) = e.details.as_mut() {
                            d["source_features"] = json!([a.input, b.input]);
                        }
                        e
                    })?;
                    self.findings.insert((index, i, j), finding);
                    crate::progress::emit("check", "completed", Some(feature));
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn collision_finding(
    k: &mut Kernel,
    a: &Brep,
    b: &Brep,
    a_id: &str,
    b_id: &str,
    allowed: bool,
    threshold: f64,
) -> Result<Value> {
    let overlap = if assembly::overlapping_boxes(k, a, b)? {
        let ca = solidcore::topology::brep::p3::brep_copy(k, a)?;
        let cb = solidcore::topology::brep::p3::brep_copy(k, b)?;
        let mut common = boolean_intersection(k, ca, cb)?;
        assembly::volume(k, &mut common)?
    } else {
        0.
    };
    if !allowed && overlap > threshold + 1e-6 {
        let mut error = CadError::new(
            "ASSEMBLY_COLLISION",
            format!("{a_id} and {b_id} overlap {overlap} mm3 (limit {threshold})"),
        );
        error.details =
            Some(json!({"parts":[a_id,b_id],"overlap_mm3":overlap,"threshold_mm3":threshold}));
        return Err(error);
    }
    Ok(
        json!({"kind":"collision","parts":[a_id,b_id],"overlap_mm3":overlap,"allowed":allowed,"threshold_mm3":threshold}),
    )
}
