//! Explicit engineering correspondence, evaluated after the actual mate transform.
use super::{
    contract::{Range, Tolerance},
    interfaces,
};
use crate::*;
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Attachment {
    pub instance: String,
    pub interface: String,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PlaneRelation {
    Aligned,
    Opposed,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Symmetry {
    RequireUnique,
    AllowEquivalent,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Correspondence {
    Pattern {
        require_all: bool,
        symmetry: Symmetry,
    },
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum HoleSize {
    IndependentRanges {
        a_diameter_mm: Range,
        b_diameter_mm: Range,
    },
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RegisterFit {
    pub radial_clearance_mm: Range,
    pub max_axis_offset_mm: Scalar,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "InterfaceFitRequirements")]
pub struct Requirements {
    pub max_plane_gap_mm: Scalar,
    pub max_hole_axis_offset_mm: Scalar,
    pub hole_size: HoleSize,
    pub register: Option<RegisterFit>,
}
fn xyz(v: &Value) -> Vec3 {
    super::engine::vec(serde_json::from_value(v.clone()).unwrap())
}
fn bound(r: &Range) -> Result<()> {
    super::selection::check_range(r)?;
    if r.min.is_none() || r.max.is_none() || r.min.unwrap() < 0. {
        return Err(CadError::new(
            "QUERY_ARGUMENT",
            "fit ranges require finite nonnegative min and max",
        ));
    }
    Ok(())
}
fn in_range(n: f64, r: &Range) -> bool {
    n >= r.min.unwrap() && n <= r.max.unwrap()
}
pub(crate) fn validate(
    m: &Model,
    a: &Attachment,
    b: &Attachment,
    correspondence: &Correspondence,
    t: Tolerance,
    r: &Requirements,
) -> Result<()> {
    super::engine::validate_tolerance(t)?;
    let Correspondence::Pattern { require_all, .. } = correspondence;
    if !require_all {
        return Err(CadError::new(
            "INTERFACE_UNSUPPORTED",
            "v1 requires all pattern members",
        ));
    }
    for attachment in [a, b] {
        let i = m
            .assembly
            .iter()
            .find(|i| i.id == attachment.instance)
            .ok_or_else(|| CadError::new("INVALID_REFERENCE", &attachment.instance))?;
        if !m
            .interfaces
            .iter()
            .any(|d| d.definition == i.input && d.id == attachment.interface)
        {
            return Err(CadError::new("INTERFACE_NOT_FOUND", &attachment.interface));
        }
    }
    if r.max_plane_gap_mm.resolve(&m.parameters)? < 0.
        || r.max_hole_axis_offset_mm.resolve(&m.parameters)? < 0.
    {
        return Err(CadError::new(
            "INVALID_DIMENSION",
            "negative interface fit threshold",
        ));
    }
    let HoleSize::IndependentRanges {
        a_diameter_mm,
        b_diameter_mm,
    } = &r.hole_size;
    bound(a_diameter_mm)?;
    bound(b_diameter_mm)?;
    if let Some(r) = &r.register {
        bound(&r.radial_clearance_mm)?;
        if r.max_axis_offset_mm.resolve(&m.parameters)? < 0. {
            return Err(CadError::new(
                "INVALID_DIMENSION",
                "negative register offset",
            ));
        }
    }
    Ok(())
}
fn enumerate(
    edges: &[Vec<usize>],
    i: usize,
    used: &mut BTreeSet<usize>,
    pairs: &mut Vec<usize>,
    out: &mut Vec<Vec<usize>>,
    steps: &mut usize,
) -> Result<()> {
    *steps += 1;
    if *steps > 10_000 || out.len() > 256 {
        return Err(CadError::new(
            "QUERY_LIMIT",
            "interface correspondence search exceeded budget",
        ));
    }
    if i == edges.len() {
        out.push(pairs.clone());
        return Ok(());
    }
    for j in &edges[i] {
        if used.insert(*j) {
            pairs.push(*j);
            enumerate(edges, i + 1, used, pairs, out, steps)?;
            pairs.pop();
            used.remove(j);
        }
    }
    Ok(())
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn check(
    m: &Model,
    a: &Attachment,
    b: &Attachment,
    relation: PlaneRelation,
    correspondence: &Correspondence,
    t: Tolerance,
    r: &Requirements,
    interfaces: &interfaces::Definitions,
    poses: &BTreeMap<String, placement::Rigid>,
) -> Result<Value> {
    let values = [a, b].map(|a| {
        let i = m.assembly.iter().find(|i| i.id == a.instance).unwrap();
        interfaces
            .get(&i.input)
            .and_then(|ds| ds.get(&a.interface))
            .ok_or_else(|| CadError::new("INTERFACE_NOT_FOUND", &a.interface))
            .map(|d| d.transformed(poses[&a.instance]))
    });
    let [av, bv] = values;
    let (av, bv) = (av?, bv?);
    let n = xyz(&av["normal"]);
    let nb = xyz(&bv["normal"]);
    let dot = n.dot(nb)
        * match relation {
            PlaneRelation::Aligned => 1.,
            PlaneRelation::Opposed => -1.,
        };
    let angle = n.cross(nb).length().atan2(dot);
    let gap = (xyz(&bv["origin_mm"]) - xyz(&av["origin_mm"])).dot(n).abs();
    let max_offset = r.max_hole_axis_offset_mm.resolve(&m.parameters)?;
    let max_gap = r.max_plane_gap_mm.resolve(&m.parameters)?;
    let am = av["group"]["members"].as_array().unwrap();
    let bm = bv["group"]["members"].as_array().unwrap();
    let HoleSize::IndependentRanges {
        a_diameter_mm,
        b_diameter_mm,
    } = &r.hole_size;
    let mut deviations = vec![];
    let mut edges = vec![];
    for ah in am {
        let mut candidates = vec![];
        for (j, bh) in bm.iter().enumerate() {
            let offset = (xyz(&ah["center_mm"]) - xyz(&bh["center_mm"]))
                .cross(n)
                .length();
            let na = xyz(&ah["inward_direction"]);
            let nb = xyz(&bh["inward_direction"]);
            let axis_angle = na.cross(nb).length().atan2(na.dot(nb).abs());
            let ar = ah["diameter_mm"].as_f64().unwrap();
            let br = bh["diameter_mm"].as_f64().unwrap();
            deviations.push(json!({"a_hole_id":ah["hole_id"],"b_hole_id":bh["hole_id"],"axis_offset_mm":offset,"axis_angle_rad":axis_angle,"a_diameter_mm":ar,"b_diameter_mm":br,"radius_difference_mm":(ar-br)/2.}));
            if offset <= max_offset
                && axis_angle <= t.angular_rad
                && in_range(ar, a_diameter_mm)
                && in_range(br, b_diameter_mm)
            {
                candidates.push(j);
            }
        }
        edges.push(candidates);
    }
    let mut alternatives = vec![];
    let mut steps = 0;
    if am.len() == bm.len() {
        enumerate(
            &edges,
            0,
            &mut BTreeSet::new(),
            &mut vec![],
            &mut alternatives,
            &mut steps,
        )?;
    }
    let mut register = json!(null);
    let mut register_pass = true;
    if let Some(req) = &r.register {
        let (ar, br) = (&av["register"], &bv["register"]);
        if ar.is_null() || br.is_null() {
            register_pass = false;
            register = json!({"status":"required_register_missing"});
        } else {
            let aa = xyz(&ar["axis"]["direction"]);
            let ba = xyz(&br["axis"]["direction"]);
            let axis_angle = aa.cross(ba).length().atan2(aa.dot(ba).abs());
            let offset = (xyz(&ar["axis"]["point_mm"]) - xyz(&br["axis"]["point_mm"]))
                .cross(aa)
                .length();
            let radial = if ar["material_side"] == "internal" && br["material_side"] == "external" {
                ar["radius_mm"].as_f64().unwrap() - br["radius_mm"].as_f64().unwrap()
            } else if br["material_side"] == "internal" && ar["material_side"] == "external" {
                br["radius_mm"].as_f64().unwrap() - ar["radius_mm"].as_f64().unwrap()
            } else {
                f64::NAN
            };
            register_pass = radial.is_finite()
                && in_range(radial, &req.radial_clearance_mm)
                && offset <= req.max_axis_offset_mm.resolve(&m.parameters)?
                && axis_angle <= t.angular_rad;
            register = json!({"radial_clearance_mm":radial,"axis_offset_mm":offset,"axis_angle_rad":axis_angle,"passed":register_pass});
        }
    }
    let Correspondence::Pattern { symmetry, .. } = correspondence;
    let unique = matches!(symmetry, Symmetry::AllowEquivalent) || alternatives.len() <= 1;
    let passed = gap <= max_gap
        && angle <= t.angular_rad
        && !alternatives.is_empty()
        && unique
        && register_pass;
    let pairs = |mapping: &Vec<usize>| {
        mapping
            .iter()
            .enumerate()
            .map(|(i, j)| deviations[i * bm.len() + *j].clone())
            .collect::<Vec<_>>()
    };
    let result = json!({"kind":"interface_compatibility","passed":passed,"a":a,"b":b,"plane_relation":relation,"plane_gap_mm":gap,"normal_angle_rad":angle,"tolerance":t,"requirements":r,"correspondence":alternatives.first().map(pairs).unwrap_or_default(),"alternatives":alternatives.iter().map(pairs).collect::<Vec<_>>(),"all_pair_deviations":deviations,"register":register,"pattern_symmetry":{"a":av["group"]["symmetry"],"b":bv["group"]["symmetry"]},"claim":"measured_correspondence_at_solved_pose_no_thread_or_load_claim"});
    if !passed {
        let mut e = CadError::new(
            if !unique {
                "INTERFACE_AMBIGUOUS"
            } else {
                "INTERFACE_INCOMPATIBLE"
            },
            "measured interface fit/correspondence requirements failed",
        );
        e.details = Some(result);
        return Err(e);
    }
    Ok(result)
}
