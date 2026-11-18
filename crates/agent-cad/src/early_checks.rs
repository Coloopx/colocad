//! [INPUT]: 已构造的不可变定义与无mate的固定实例位姿。
//! [OUTPUT]: 依赖就绪即执行的原精确碰撞检查；通过记录被最终装配复用。
//! [POS]: 特征执行与装配收口之间的检查调度，不放宽几何门限。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use crate::*;
type Bounds = [[f64; 3]; 2];
type PairKey = (usize, usize, usize);
struct Pair {
    key: PairKey,
    a: usize,
    b: usize,
    allowed: bool,
    threshold: f64,
}

#[derive(Default)]
pub(crate) struct EarlyChecks {
    pending: BTreeMap<String, Vec<Pair>>,
    poses: BTreeMap<usize, placement::Rigid>,
    bounds: BTreeMap<usize, Bounds>,
    pub overlaps: BTreeMap<PairKey, f64>,
}

impl EarlyChecks {
    pub fn plan(m: &Model) -> Result<Self> {
        let mut plan = Self::default();
        if m.mates.is_some()
            || !m
                .checks
                .iter()
                .any(|c| matches!(c, AssemblyCheck::Collision { .. }))
        {
            return Ok(plan);
        }
        let groups = placement::groups(m)?;
        let ranks: BTreeMap<_, _> = m
            .features
            .iter()
            .enumerate()
            .map(|(i, f)| (f.id(), i))
            .collect();
        let instances: BTreeMap<_, _> = m
            .assembly
            .iter()
            .enumerate()
            .map(|(i, p)| (p.id.as_str(), i))
            .collect();
        for (index, check) in m.checks.iter().enumerate() {
            let AssemblyCheck::Collision {
                parts,
                allowed_pairs,
                max_overlap_mm3,
            } = check
            else {
                continue;
            };
            let threshold = max_overlap_mm3.resolve(&m.parameters)?;
            for (i, id) in parts.iter().enumerate() {
                let a = instances[id.as_str()];
                if let std::collections::btree_map::Entry::Vacant(slot) = plan.poses.entry(a) {
                    slot.insert(
                        placement::world(m, &m.assembly[a], placement::PoseMode::Normal, &groups)?
                            .0,
                    );
                }
                for (j, other) in parts.iter().enumerate().skip(i + 1) {
                    let b = instances[other.as_str()];
                    let left = m.assembly[a].input.as_str();
                    let right = m.assembly[b].input.as_str();
                    let ready = if ranks[left] >= ranks[right] {
                        left
                    } else {
                        right
                    };
                    let allowed = allowed_pairs.iter().any(|p| {
                        (&p[0] == id && &p[1] == other) || (&p[1] == id && &p[0] == other)
                    });
                    plan.pending
                        .entry(ready.to_owned())
                        .or_default()
                        .push(Pair {
                            key: (index, i, j),
                            a,
                            b,
                            allowed,
                            threshold,
                        });
                }
            }
        }
        Ok(plan)
    }
    fn placed(
        &self,
        k: &mut Kernel,
        m: &Model,
        bodies: &BTreeMap<String, Brep>,
        index: usize,
    ) -> Result<Brep> {
        let mut body =
            solidcore::topology::brep::p3::brep_copy(k, &bodies[&m.assembly[index].input])?;
        self.poses[&index].apply(k, &mut body)?;
        Ok(body)
    }
    fn bounds(
        &mut self,
        k: &mut Kernel,
        m: &Model,
        bodies: &BTreeMap<String, Brep>,
        index: usize,
    ) -> Result<Bounds> {
        if let Some(bounds) = self.bounds.get(&index) {
            return Ok(*bounds);
        }
        let body = self.placed(k, m, bodies, index)?;
        let bbox = brep_bounding_box(k, &body, true)?;
        let bounds = [
            std::array::from_fn(|i| bbox.min_at(i)),
            std::array::from_fn(|i| bbox.max_at(i)),
        ];
        self.bounds.insert(index, bounds);
        Ok(bounds)
    }
    pub fn evaluate(
        &mut self,
        k: &mut Kernel,
        m: &Model,
        bodies: &BTreeMap<String, Brep>,
        feature: &str,
    ) -> Result<()> {
        let Some(pairs) = self.pending.remove(feature) else {
            return Ok(());
        };
        for pair in pairs {
            crate::progress::emit("check", "started", Some(feature));
            let a = self.bounds(k, m, bodies, pair.a)?;
            let b = self.bounds(k, m, bodies, pair.b)?;
            let overlap = if (0..3).all(|i| a[1][i].min(b[1][i]) - a[0][i].max(b[0][i]) > 1e-8) {
                let left = self.placed(k, m, bodies, pair.a)?;
                let right = self.placed(k, m, bodies, pair.b)?;
                let mut common = boolean_intersection(k, left, right)?;
                assembly::volume(k, &mut common)?
            } else {
                0.
            };
            if !pair.allowed && overlap > pair.threshold + 1e-6 {
                let a = &m.assembly[pair.a];
                let b = &m.assembly[pair.b];
                let mut error = overlap_error(&a.id, &b.id, pair.threshold, overlap);
                error.feature_id = Some(feature.to_owned());
                error.details.as_mut().unwrap()["source_features"] = json!([a.input, b.input]);
                return Err(error);
            }
            self.overlaps.insert(pair.key, overlap);
            crate::progress::emit("check", "completed", Some(feature));
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
    finding(a_id, b_id, allowed, threshold, overlap)
}

pub(crate) fn finding(
    a_id: &str,
    b_id: &str,
    allowed: bool,
    threshold: f64,
    overlap: f64,
) -> Result<Value> {
    if !allowed && overlap > threshold + 1e-6 {
        return Err(overlap_error(a_id, b_id, threshold, overlap));
    }
    Ok(
        json!({"kind":"collision","parts":[a_id,b_id],"overlap_mm3":overlap,"allowed":allowed,"threshold_mm3":threshold}),
    )
}

fn overlap_error(a_id: &str, b_id: &str, threshold: f64, overlap: f64) -> CadError {
    let mut error = CadError::new(
        "ASSEMBLY_COLLISION",
        format!("{a_id} and {b_id} overlap {overlap} mm3 (limit {threshold})"),
    );
    error.details =
        Some(json!({"parts":[a_id,b_id],"overlap_mm3":overlap,"threshold_mm3":threshold}));
    error
}
