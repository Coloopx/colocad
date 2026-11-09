//! Persistent reconstruction recipes; observation refs remain revision-local.
use super::{
    contract::*,
    engine::{unit, vec, Context},
    selection,
};
use crate::*;

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Declaration {
    pub id: String,
    pub definition: String,
    pub tolerance: Tolerance,
    pub derive: Recipe,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Recipe {
    Selected {
        selection: Selection,
        derive: Derive,
    },
    DesignPoint {
        point: operations::V3,
    },
    PointDifference {
        from: String,
        to: String,
    },
    Frame {
        origin: String,
        x: Directed,
        z: Directed,
    },
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Directed {
    pub datum: String,
    pub sense: Sense,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Sense {
    Positive,
    Negative,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Resolved {
    pub kind: String,
    pub geometry: Value,
    pub source_tokens: Vec<u64>,
    pub provenance: String,
    pub remaining_rigid_freedoms: Value,
}
pub(crate) type Definitions = BTreeMap<String, BTreeMap<String, Resolved>>;
fn array(v: Vec3) -> [f64; 3] {
    [v.x, v.y, v.z]
}
fn xyz(v: &Value) -> Result<Vec3> {
    let a: [f64; 3] = serde_json::from_value(v.clone())
        .map_err(|_| CadError::new("DATUM_GEOMETRY", "missing or invalid datum vector"))?;
    if a.iter().any(|x| !x.is_finite()) {
        return Err(CadError::new("DATUM_GEOMETRY", "nonfinite datum vector"));
    }
    Ok(vec(a))
}
fn semantic(s: &Selection) -> bool {
    match s {
        Selection::Refs { .. } => false,
        Selection::Entities { .. } => true,
        Selection::Filter { input, .. }
        | Selection::Children { input, .. }
        | Selection::Ancestors { input, .. }
        | Selection::Siblings { input, .. }
        | Selection::Rank { input, .. }
        | Selection::Expect { input, .. } => semantic(input),
        Selection::Union { inputs } | Selection::Intersection { inputs } => {
            inputs.iter().all(semantic)
        }
        Selection::Difference { left, right } => semantic(left) && semantic(right),
    }
}
impl Recipe {
    fn dependencies(&self) -> Vec<&str> {
        match self {
            Self::Selected { .. } | Self::DesignPoint { .. } => vec![],
            Self::PointDifference { from, to } => vec![from, to],
            Self::Frame { origin, x, z } => vec![origin, &x.datum, &z.datum],
        }
    }
}
pub(crate) fn validate(m: &Model) -> Result<()> {
    if m.datums.len() > 128 {
        return Err(CadError::new(
            "RESOURCE_LIMIT",
            "at most 128 datum declarations",
        ));
    }
    let definitions: BTreeSet<_> = if m.assembly.is_empty() {
        BTreeSet::from([m.output.as_str()])
    } else {
        m.assembly.iter().map(|i| i.input.as_str()).collect()
    };
    let mut seen = BTreeSet::new();
    for d in &m.datums {
        let result = (|| {
            if !valid_id(&d.id)
                || !definitions.contains(d.definition.as_str())
                || seen.contains(&(d.definition.as_str(), d.id.as_str()))
            {
                return Err(CadError::new(
                    "DATUM_REFERENCE",
                    "unique datum IDs and final definitions required",
                ));
            }
            super::engine::validate_tolerance(d.tolerance)?;
            match &d.derive {
                Recipe::Selected { selection, .. } => {
                    selection::selection_budget(selection, 1, &mut 0)?;
                    if !semantic(selection) {
                        return Err(CadError::new("DATUM_RECIPE","persistent recipes require semantic selection, not revision-local refs"));
                    }
                }
                Recipe::DesignPoint { point: p } => {
                    point(p, &m.parameters)?;
                }
                _ => {}
            }
            for dep in d.derive.dependencies() {
                if !seen.contains(&(d.definition.as_str(), dep)) {
                    return Err(CadError::new(
                        "DATUM_REFERENCE",
                        format!("{dep} must name an earlier datum in {}", d.definition),
                    ));
                }
            }
            Ok(())
        })();
        result.map_err(|mut e: CadError| {
            e.details = Some(json!({"definition_id":d.definition,"datum_id":d.id}));
            e
        })?;
        seen.insert((d.definition.as_str(), d.id.as_str()));
    }
    if let Some(system) = &m.mates {
        for mate in &system.constraints {
            let (_, a, b) = mate.fields();
            for a in [a, b] {
                if a.frame.is_some() && a.datum.is_some() {
                    return Err(CadError::new(
                        "MATE_ATTACHMENT",
                        "frame and datum are mutually exclusive",
                    ));
                }
                if let Some(id) = &a.datum {
                    let i = m
                        .assembly
                        .iter()
                        .find(|i| i.id == a.instance)
                        .ok_or_else(|| CadError::new("MATE_REFERENCE", &a.instance))?;
                    if !seen.contains(&(i.input.as_str(), id.as_str())) {
                        return Err(CadError::new(
                            "DATUM_REFERENCE",
                            format!("{} / {id}", i.input),
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}
fn require<'a>(
    prior: &'a BTreeMap<String, Resolved>,
    id: &str,
    kind: &str,
) -> Result<&'a Resolved> {
    let d = prior
        .get(id)
        .ok_or_else(|| CadError::new("DATUM_REFERENCE", id))?;
    if d.kind != kind {
        return Err(CadError::new(
            "DATUM_KIND_MISMATCH",
            format!("{id} must be {kind}"),
        ));
    }
    Ok(d)
}
fn directed(prior: &BTreeMap<String, Resolved>, r: &Directed) -> Result<Vec3> {
    let d = prior
        .get(&r.datum)
        .ok_or_else(|| CadError::new("DATUM_REFERENCE", &r.datum))?;
    let value = match d.kind.as_str() {
        "axis" | "direction" => &d.geometry["direction"],
        "plane" => &d.geometry["normal"],
        _ => {
            return Err(CadError::new(
                "DATUM_KIND_MISMATCH",
                "frame axes require axis, direction or plane normal",
            ))
        }
    };
    Ok(unit(array(xyz(value)?))?
        * match r.sense {
            Sense::Positive => 1.,
            Sense::Negative => -1.,
        })
}
pub(crate) fn resolve(
    c: &mut Context,
    d: &Declaration,
    prior: &BTreeMap<String, Resolved>,
    parameters: &BTreeMap<String, f64>,
) -> Result<Resolved> {
    c.tolerance = d.tolerance;
    let mut tokens = BTreeSet::new();
    for id in d.derive.dependencies() {
        tokens.extend(&prior[id].source_tokens);
    }
    let (kind, geometry, provenance, freedoms) = match &d.derive {
        Recipe::Selected { selection, derive } => {
            let selected = selection::select(c, selection)?;
            if selected.len() != 1 {
                return Err(CadError::new(
                    "SELECTION_CARDINALITY",
                    format!(
                        "datum requires exactly 1 entity; complete selection contains {}",
                        selected.len()
                    ),
                ));
            }
            let token = *selected.first().unwrap();
            tokens.insert(token);
            let v = selection::datum(c, token, derive)?;
            (
                v["kind"].as_str().unwrap().to_string(),
                v["geometry"].clone(),
                "derived".to_string(),
                v["remaining_rigid_freedoms"].clone(),
            )
        }
        Recipe::DesignPoint { point: p } => (
            "point".into(),
            json!({"point_mm":array(point(p,parameters)?)}),
            "declared_design_intent".into(),
            json!(["rotation_x", "rotation_y", "rotation_z"]),
        ),
        Recipe::PointDifference { from, to } => {
            let a = require(prior, from, "point")?;
            let b = require(prior, to, "point")?;
            let delta = xyz(&b.geometry["point_mm"])? - xyz(&a.geometry["point_mm"])?;
            if delta.length() <= d.tolerance.linear_mm {
                return Err(CadError::new(
                    "DATUM_DEGENERATE",
                    "ordered points are coincident within linear tolerance",
                ));
            }
            (
                "direction".into(),
                json!({"direction":array(delta/delta.length()),"direction_semantics":"ordered_point_difference"}),
                "derived_from_named_datums".into(),
                json!([
                    "translation_x",
                    "translation_y",
                    "translation_z",
                    "rotation_about_direction"
                ]),
            )
        }
        Recipe::Frame { origin, x, z } => {
            let o = xyz(&require(prior, origin, "point")?.geometry["point_mm"])?;
            let x = directed(prior, x)?;
            let z = directed(prior, z)?;
            let error = x.dot(z).clamp(-1., 1.).asin().abs();
            if error > d.tolerance.angular_rad || x.cross(z).length() < 1e-8 {
                return Err(CadError::new(
                    "DATUM_DEGENERATE",
                    format!("frame axes are not orthogonal: angular error {error} rad"),
                ));
            }
            // Re-orthogonalize only within the explicit tolerance, retaining handedness.
            let y = unit(array(z.cross(x)))?;
            let x = unit(array(y.cross(z)))?;
            (
                "frame".into(),
                json!({"origin_mm":array(o),"axes":[array(x),array(y),array(z)],"handedness":"right","orthogonality_correction_rad":error}),
                "derived_from_named_datums".into(),
                json!([]),
            )
        }
    };
    Ok(Resolved {
        kind,
        geometry,
        source_tokens: tokens.into_iter().collect(),
        provenance,
        remaining_rigid_freedoms: freedoms,
    })
}
impl Resolved {
    pub(crate) fn numerical_frame(&self) -> Result<operations::Frame> {
        let (o, x, y) = if self.kind == "frame" {
            (
                xyz(&self.geometry["origin_mm"])?,
                xyz(&self.geometry["axes"][0])?,
                xyz(&self.geometry["axes"][1])?,
            )
        } else {
            let o = xyz(&self.geometry["point_mm"])?;
            let z = match self.kind.as_str() {
                "point" => Vec3::Z,
                "plane" => xyz(&self.geometry["normal"])?,
                "axis" => xyz(&self.geometry["direction"])?,
                _ => {
                    return Err(CadError::new(
                        "DATUM_KIND_MISMATCH",
                        "direction alone has no attachment position",
                    ))
                }
            };
            let seed = if z.dot(Vec3::X).abs() < 0.8 {
                Vec3::X
            } else {
                Vec3::Y
            };
            let x = unit(array(seed - z * seed.dot(z)))?;
            (o, x, z.cross(x))
        };
        Ok(operations::Frame {
            origin: array(o).map(Scalar::Number),
            x_axis: array(x).map(Scalar::Number),
            y_axis: array(y).map(Scalar::Number),
        })
    }
    pub(crate) fn observation(
        &self,
        id: &str,
        definition: &str,
        manifest: &super::artifact::Manifest,
        pose: placement::Rigid,
        occurrence: Option<Vec<String>>,
    ) -> Result<Value> {
        let mut geometry = self.geometry.clone();
        for key in ["point_mm", "origin_mm"] {
            if !geometry[key].is_null() {
                geometry[key] = json!(array(pose.point(xyz(&geometry[key])?)));
            }
        }
        for key in ["normal", "direction"] {
            if !geometry[key].is_null() {
                geometry[key] = json!(array(pose.vector(xyz(&geometry[key])?)));
            }
        }
        if let Some(axes) = geometry.get_mut("axes").and_then(Value::as_array_mut) {
            for a in axes {
                *a = json!(array(pose.vector(xyz(a)?)));
            }
        }
        let bindings = &manifest.body.definitions[definition];
        let refs=self.source_tokens.iter().map(|t|{
            let b=&bindings.bindings[t];
            json!({"query_revision_id":manifest.query_revision_id,"definition_id":definition,"definition_sha256":bindings.definition_sha256,"kind":b.kind,"topology_id":format!("t{t}"),"instance_path":occurrence})
        }).collect::<Vec<_>>();
        Ok(
            json!({"id":id,"kind":self.kind,"geometry":geometry,"provenance":self.provenance,"source_refs":refs,"query_revision_id":manifest.query_revision_id,"definition_id":definition,"space":"definition","remaining_rigid_freedoms":self.remaining_rigid_freedoms,"engineering_fit":"not_asserted"}),
        )
    }
}
