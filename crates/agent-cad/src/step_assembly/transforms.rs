//! [INPUT]: Part21轴系、显式representation尺度及原刚体约束。
//! [OUTPUT]: 长度转为mm、方向保持无量纲的刚体变换。
//! [POS]: STEP装配位姿解析；单位归属由units模块判定。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use super::*;
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
pub(super) fn axis(f: &StepFile, id: u64, scale: f64) -> Result<Rigid> {
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
pub(super) fn mapping_transform(f: &StepFile, id: u64, scale: f64) -> Result<Rigid> {
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
