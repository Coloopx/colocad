//! [INPUT]: 两个原生 B-rep 和显式 union/intersect 操作。
//! [OUTPUT]: 拒绝有顶点越出操作数或结果包围盒的闭实体布尔结果。
//! [POS]: 特征执行与 SolidCore 布尔调用的边界。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use crate::{
    boolean_intersection, boolean_union, brep_bounding_box, brep_is_manifold_solid, Brep, CadError,
    Kernel, Result,
};
use serde_json::json;
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

pub(crate) fn run(k: &mut Kernel, left: Brep, right: Brep, op: Operation) -> Result<Brep> {
    let inputs = if brep_is_manifold_solid(&left)? && brep_is_manifold_solid(&right)? {
        Some([Bounds::read(k, &left)?, Bounds::read(k, &right)?])
    } else {
        None
    };
    let result = match op {
        Operation::Union => boolean_union(k, left, right)?,
        Operation::Intersection => boolean_intersection(k, left, right)?,
    };
    if let Some([a, b]) = inputs {
        if brep_is_manifold_solid(&result)? {
            check(op, &a, &b, &Bounds::read(k, &result)?)?;
        }
    }
    Ok(result)
}

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

fn outside_bounds(name: &str, vertices: &Extent3d, enclosing: &Extent3d) -> CadError {
    let corners = |b: &Extent3d| {
        [
            [b.min_at(0), b.min_at(1), b.min_at(2)],
            [b.max_at(0), b.max_at(1), b.max_at(2)],
        ]
    };
    let (vertices, enclosing) = (corners(vertices), corners(enclosing));
    let mut error = CadError::new(
        "GEOMETRY_VALIDATION_FAILED",
        format!("{name} vertices {vertices:?} mm outside enclosing bounds {enclosing:?} mm"),
    );
    error.details = Some(json!({
        "reason": "boolean_vertices_outside_bounds", "operation": name,
        "vertices_bbox_mm": vertices, "enclosing_bbox_mm": enclosing
    }));
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
