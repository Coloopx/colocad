//! [INPUT]: 两个原生 B-rep 和显式 union/intersect 操作。
//! [OUTPUT]: 拒绝有顶点越出操作数或结果包围盒的闭实体布尔结果。
//! [POS]: 特征执行与 SolidCore 布尔调用的边界；守卫禁止内联进导入匹配臂。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use crate::{
    boolean_intersection, boolean_union, brep_bounding_box, brep_is_manifold_solid, Brep, CadError,
    Kernel, Result,
};
use serde_json::{Map, Value};
use solidcore::{geom::extent3d::Extent3d, topology::vertex};

#[derive(Clone, Copy)]
pub(crate) enum Operation {
    Union,
    Intersection,
}

struct Bounds {
    vertices: Extent3d,
    enclosing: Extent3d,
}

impl Bounds {
    #[inline(never)]
    fn read(k: &Kernel, body: &Brep) -> Result<Self> {
        let mut vertices = Extent3d::new();
        for &id in body.vertex_list() {
            vertices.add_point(vertex::point(body, id)?);
        }
        // The loose box includes faces, edges and their own native tolerances.
        Ok(Self {
            vertices,
            enclosing: brep_bounding_box(k, body, false)?,
        })
    }
}

#[inline(never)]
pub(crate) fn run(k: &mut Kernel, left: Brep, right: Brep, op: Operation) -> Result<Brep> {
    // Kernel merge consumes both B-reps, so operand boxes are snapshotted first.
    // `inline(never)` keeps that read out of the import match arm.
    let operands = snapshot_solids(k, &left, &right)?;
    let result = match op {
        Operation::Union => boolean_union(k, left, right)?,
        Operation::Intersection => boolean_intersection(k, left, right)?,
    };
    if let Some(operands) = operands {
        verify(k, op, operands, &result)?;
    }
    Ok(result)
}

#[inline(never)]
fn snapshot_solids(k: &Kernel, left: &Brep, right: &Brep) -> Result<Option<[Bounds; 2]>> {
    if brep_is_manifold_solid(left)? && brep_is_manifold_solid(right)? {
        Ok(Some([Bounds::read(k, left)?, Bounds::read(k, right)?]))
    } else {
        Ok(None)
    }
}

#[cold]
#[inline(never)]
fn verify(k: &Kernel, op: Operation, operands: [Bounds; 2], result: &Brep) -> Result<()> {
    if brep_is_manifold_solid(result)? {
        check(op, &operands[0], &operands[1], &Bounds::read(k, result)?)?;
    }
    Ok(())
}

#[cold]
#[inline(never)]
fn check(op: Operation, left: &Bounds, right: &Bounds, result: &Bounds) -> Result<()> {
    let (name, checks) = match op {
        Operation::Union => (
            "union",
            [
                (left.vertices.union(&right.vertices), result.enclosing),
                (result.vertices, left.enclosing.union(&right.enclosing)),
            ],
        ),
        Operation::Intersection => (
            "intersect",
            [
                (result.vertices, left.enclosing),
                (result.vertices, right.enclosing),
            ],
        ),
    };
    for (vertices, enclosing) in checks {
        // Only actual vertices can witness a violation. Loose boxes may overestimate.
        if !vertices.has_negative_volume()
            && !enclosing.has_negative_volume()
            && !vertices.is_contained_by(&enclosing, 1e-6)
        {
            return Err(outside_bounds(name, &vertices, &enclosing));
        }
    }
    Ok(())
}

fn bbox_value(b: &Extent3d) -> Value {
    Value::Array(vec![
        Value::Array(vec![
            b.min_at(0).into(),
            b.min_at(1).into(),
            b.min_at(2).into(),
        ]),
        Value::Array(vec![
            b.max_at(0).into(),
            b.max_at(1).into(),
            b.max_at(2).into(),
        ]),
    ])
}

#[cold]
fn details(operation: &str, vertices: &Extent3d, enclosing: &Extent3d) -> Value {
    let mut map = Map::with_capacity(4);
    map.insert(
        "reason".into(),
        Value::String("boolean_vertices_outside_bounds".into()),
    );
    map.insert("operation".into(), Value::String(operation.into()));
    map.insert("vertices_bbox_mm".into(), bbox_value(vertices));
    map.insert("enclosing_bbox_mm".into(), bbox_value(enclosing));
    Value::Object(map)
}

#[cold]
#[inline(never)]
fn outside_bounds(name: &str, vertices: &Extent3d, enclosing: &Extent3d) -> CadError {
    let mut error = CadError::new(
        "GEOMETRY_VALIDATION_FAILED",
        format!(
            "{name} vertices {:?} mm outside enclosing bounds {:?} mm",
            [
                [vertices.min_at(0), vertices.min_at(1), vertices.min_at(2)],
                [vertices.max_at(0), vertices.max_at(1), vertices.max_at(2)]
            ],
            [
                [
                    enclosing.min_at(0),
                    enclosing.min_at(1),
                    enclosing.min_at(2)
                ],
                [
                    enclosing.max_at(0),
                    enclosing.max_at(1),
                    enclosing.max_at(2)
                ]
            ]
        ),
    );
    error.details = Some(details(name, vertices, enclosing));
    error
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn union_cannot_lose_the_larger_operand_in_either_order() {
        let small = Extent3d::from_point(crate::Vec3::ZERO);
        let big = small.union(&Extent3d::from_point(crate::Vec3::new(8., 0., 0.)));
        let a = Bounds {
            vertices: small,
            enclosing: small,
        };
        let b = Bounds {
            vertices: big,
            enclosing: big,
        };
        for (left, right) in [(&a, &b), (&b, &a)] {
            let err = check(Operation::Union, left, right, &a).unwrap_err();
            assert_eq!(err.code, "GEOMETRY_VALIDATION_FAILED");
            check(Operation::Union, left, right, &b).unwrap();
        }
    }
}
