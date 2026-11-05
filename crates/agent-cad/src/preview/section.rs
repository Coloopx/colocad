//! Display-only half-space clipping. The exact model and reusable source mesh stay intact.
use super::*;
use solidcore_record::Vec3 as P;
use solidcore_tess::Mesh;
#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SectionPlane {
    /// Retain dot(normal / |normal|, position) <= offset_mm.
    pub normal: [f64; 3],
    pub offset_mm: f64,
}
impl SectionPlane {
    pub(super) fn unit(self) -> Result<P> {
        let [x, y, z] = self.normal;
        let n = x.hypot(y).hypot(z);
        if !n.is_finite() || n < 1e-12 || !self.offset_mm.is_finite() {
            return Err(CadError::new(
                "INVALID_SECTION",
                "finite offset and nonzero plane normal required",
            ));
        }
        Ok(P::new(x / n, y / n, z / n))
    }
}
pub(super) fn clip(meshes: &[(String, Mesh)], plane: SectionPlane) -> Result<Vec<(String, Mesh)>> {
    let n = plane.unit()?;
    let distance = |p: P| n.x * p.x + n.y * p.y + n.z * p.z - plane.offset_mm;
    let mut result = Vec::new();
    let mut remaining = 200_000usize;
    for (id, mesh) in meshes {
        let mut out = Mesh::default();
        for (face, tri) in mesh.triangles.iter().enumerate() {
            let vertices = tri.map(|i| (mesh.positions[i as usize], mesh.normals[i as usize]));
            let mut polygon = Vec::with_capacity(4);
            for j in 0..3 {
                let (a, na) = vertices[(j + 2) % 3];
                let (b, nb) = vertices[j];
                let (da, db) = (distance(a), distance(b));
                if (da < 0. && db > 0.) || (da > 0. && db < 0.) {
                    let t = da / (da - db);
                    let normal = na + (nb - na) * t;
                    let length = normal.x.hypot(normal.y).hypot(normal.z);
                    polygon.push((
                        a + (b - a) * t,
                        if length > 1e-12 { normal / length } else { na },
                    ));
                }
                if db <= 0. {
                    polygon.push((b, nb));
                }
            }
            if polygon.len() < 3 {
                continue;
            }
            let count = polygon.len() - 2;
            remaining = remaining.checked_sub(count).ok_or_else(|| {
                CadError::new("PREVIEW_BUDGET", "clipped mesh exceeds triangle budget")
            })?;
            let first = out.positions.len() as u32;
            for (p, n) in polygon {
                out.positions.push(p);
                out.normals.push(n);
            }
            for j in 1..=count as u32 {
                out.triangles.push([first, first + j, first + j + 1]);
                out.tri_face
                    .push(mesh.tri_face.get(face).copied().unwrap_or(0));
            }
        }
        if !out.triangles.is_empty() {
            result.push((id.clone(), out));
        }
    }
    if result.is_empty() {
        return Err(CadError::new(
            "PREVIEW_EMPTY",
            "section plane excludes all displayed geometry",
        ));
    }
    Ok(result)
}
