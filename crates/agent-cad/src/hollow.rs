//! Closed cavities with the facade's explicit affine fallback policy and measured clearance.
use crate::*;
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    Offset,
    Scaled,
    #[default]
    OffsetThenScaled,
}
fn copy(k: &mut Kernel, b: &Brep) -> Result<Brep> {
    Ok(solidcore::topology::brep::p3::brep_copy(k, b)?)
}
pub(crate) fn execute(
    k: &mut Kernel,
    f: &Feature,
    m: &Model,
    bodies: &mut BTreeMap<String, Brep>,
    uses: &mut BTreeMap<String, usize>,
) -> Result<(Brep, Value)> {
    let Feature::Hollow {
        input,
        wall,
        cavity_strategy,
        ..
    } = f
    else {
        unreachable!()
    };
    let source = take_body(k, input, bodies, uses)?;
    let wall = positive(wall.resolve(&m.parameters)?)?;
    let bb = brep_bounding_box(k, &source, true)?;
    let size = bb.max() - bb.min();
    if 2. * wall >= size.x.min(size.y).min(size.z) {
        return Err(CadError::new(
            "INVALID_DIMENSION",
            "wall leaves no closed cavity",
        ));
    }
    let mut error = None;
    let mut offset = None;
    if !matches!(cavity_strategy, Strategy::Scaled) {
        use solidcore::offset::offset_executive::p2::{
            shell_brep_simple, OffsetOptions, ShellOptions,
        };
        let attempt = (|| -> Result<Brep> {
            let mut b = shell_brep_simple(
                k,
                &source,
                ShellOptions {
                    offset: OffsetOptions {
                        distance: -wall,
                        extend_convex_edges: true,
                        merge_results: true,
                    },
                    create_offset_solid: true,
                    merge_sheet_side_walls: true,
                },
                &[],
            )?;
            let check = check_brep(k, &mut b, CheckLevel::Level1)?;
            if !check.is_valid() || !brep_is_manifold_solid(&b)? || brep_material_census(&b)?.0 != 1
            {
                return Err(CadError::new("HOLLOW_FAILED", check.lines().join("; ")));
            }
            Ok(b)
        })();
        match attempt {
            Ok(b) => offset = Some(b),
            Err(e) => {
                if matches!(cavity_strategy, Strategy::Offset) {
                    return Err(e);
                }
                error = Some(e);
            }
        }
    }
    let (mut body, cavity, method) = if let Some(b) = offset {
        let outer = copy(k, &source)?;
        let inner = copy(k, &b)?;
        let cavity = boolean_difference(k, outer, inner)?;
        (b, cavity, "normal_offset")
    } else {
        let mut cavity = copy(k, &source)?;
        // transform_matrix converts the analytic geometry as needed for the affine map.
        let scales = [
            1. - 2. * wall / size.x,
            1. - 2. * wall / size.y,
            1. - 2. * wall / size.z,
        ];
        let c = (bb.min() + bb.max()) * 0.5;
        operations::matrix(
            k,
            &mut cavity,
            [
                Vec3::X * scales[0],
                Vec3::Y * scales[1],
                Vec3::Z * scales[2],
            ],
            Vec3::new(
                c.x * (1. - scales[0]),
                c.y * (1. - scales[1]),
                c.z * (1. - scales[2]),
            ),
        )?;
        let a = copy(k, &cavity)?;
        let b = copy(k, &source)?;
        let mut outside = boolean_difference(k, a, b)?;
        if !outside.face_list().is_empty() && measured_volume(k, &mut outside)? > 1e-8 {
            return Err(CadError::new(
                "HOLLOW_OUTSIDE",
                "scaled cavity protrudes outside the source",
            ));
        }
        let a = copy(k, &source)?;
        let b = copy(k, &cavity)?;
        (boolean_difference(k, a, b)?, cavity, "affine_scaled_cavity")
    };
    if cavity.face_list().is_empty() || brep_material_census(&body)?.0 != 1 {
        return Err(CadError::new(
            "HOLLOW_FAILED",
            "one solid with a nonempty cavity required",
        ));
    }
    let minimum = distance::boundary_distance(k, &source, &cavity)?;
    if !minimum.is_finite() || minimum <= 1e-6 {
        return Err(CadError::new(
            "HOLLOW_OUTSIDE",
            "cavity must be strictly inside its source",
        ));
    }
    let volume = measured_volume(k, &mut body)?;
    Ok((
        body,
        json!({"cavity_method":method,"requested_wall_mm":wall,"minimum_wall_mm":minimum,"region_exact":false,"variable_wall":method=="affine_scaled_cavity","offset_failure":error,"volume_mm3":volume}),
    ))
}
