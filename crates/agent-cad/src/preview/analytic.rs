//! Chord-bounded tessellation for complete circular solids declared by the IR.
//! Trimmed faces and edited primitives use the general B-rep mesher.
use super::*;
use solidcore_record::Vec3 as P;
use solidcore_tess::Mesh;

pub(super) fn mesh(model: &Model, definition: &str, chord: f64) -> Result<Option<Mesh>> {
    let p = &model.parameters;
    let (origin, lower, upper, height) = match model.features.iter().find(|f| f.id() == definition)
    {
        Some(Feature::Cylinder {
            origin,
            radius,
            height,
            ..
        }) => (
            point(origin, p)?,
            radius.resolve(p)?,
            radius.resolve(p)?,
            height.resolve(p)?,
        ),
        Some(Feature::Cone {
            origin,
            base_radius,
            top_radius,
            height,
            ..
        }) => (
            point(origin, p)?,
            base_radius.resolve(p)?,
            top_radius.resolve(p)?,
            height.resolve(p)?,
        ),
        _ => return Ok(None),
    };
    let radius = lower.max(upper);
    let half_angle = (1. - chord.min(radius) / radius).acos();
    let segments = ((std::f64::consts::PI / half_angle).ceil() as usize)
        .max(24)
        .next_multiple_of(4);
    if segments > 50_000 {
        return Err(CadError::new(
            "PREVIEW_BUDGET",
            "circular mesh exceeds triangle budget",
        ));
    }
    let mut mesh = Mesh::default();
    let slope = (lower - upper) / height;
    for i in 0..segments {
        let angle = std::f64::consts::TAU * i as f64 / segments as f64;
        let (s, c) = angle.sin_cos();
        let normal = P::new(c, s, slope) / (1. + slope * slope).sqrt();
        for (radius, z, n) in [
            (lower, 0., normal),
            (upper, height, normal),
            (lower, 0., -P::Z),
            (upper, height, P::Z),
        ] {
            mesh.positions.push(P::new(
                origin.x + radius * c,
                origin.y + radius * s,
                origin.z + z,
            ));
            mesh.normals.push(n);
        }
    }
    let bottom = mesh.positions.len() as u32;
    mesh.positions.extend([
        P::new(origin.x, origin.y, origin.z),
        P::new(origin.x, origin.y, origin.z + height),
    ]);
    mesh.normals.extend([-P::Z, P::Z]);
    for i in 0..segments {
        let a = (4 * i) as u32;
        let b = (4 * ((i + 1) % segments)) as u32;
        if lower > 0. {
            mesh.triangles
                .extend([[a, b, b + 1], [bottom, b + 2, a + 2]]);
            mesh.tri_face.extend([0, 1]);
        }
        if upper > 0. {
            mesh.triangles
                .extend([[a, b + 1, a + 1], [bottom + 1, a + 3, b + 3]]);
            mesh.tri_face.extend([0, 2]);
        }
    }
    Ok(Some(mesh))
}
