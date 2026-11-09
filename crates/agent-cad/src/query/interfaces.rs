//! Geometry-verified installation declarations; numeric orientation remains explicit intent.
use super::{
    contract::*,
    engine::{arr, unit, vec, Context},
    groups, selection,
};
use crate::*;
pub const PROFILE: &str = "planar_hole_mount/v1";
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Declaration {
    pub id: String,
    pub definition: String,
    pub profile: String,
    pub selectors: Selectors,
    pub datum: DatumRules,
    #[serde(default)]
    pub orientation_datums: Vec<OrientationDatum>,
    pub requirements: Requirements,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Cardinality {
    pub min: usize,
    pub max: usize,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FaceSelector {
    pub r#where: super::contract::Filter,
    pub cardinality: Cardinality,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GroupSelector {
    pub opening_on: String,
    pub r#where: groups::Filter,
    pub cardinality: Cardinality,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RegisterSelector {
    pub r#where: super::contract::Filter,
    pub cardinality: Cardinality,
    pub material_side: MaterialSide,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MaterialSide {
    Internal,
    External,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Selectors {
    pub plane: FaceSelector,
    pub hole_group: GroupSelector,
    pub register: Option<RegisterSelector>,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DatumRules {
    pub origin: Origin,
    pub normal: String,
    pub x_axis: Option<OrientationName>,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Origin {
    HoleGroupCenter,
    AxisPlaneIntersection { axis_slot: String },
    DeclaredPoint { point: operations::V3 },
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OrientationName {
    OrientationDatum { name: String },
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AttachmentOrientation {
    Datum { name: String },
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OrientationDatum {
    Direction {
        name: String,
        vector: operations::V3,
    },
    GeometricDatum {
        name: String,
        datum: String,
    },
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "InterfacePatternRequirement")]
pub enum PatternRequirement {
    Rectangular {
        count: [Scalar; 2],
        pitch_mm: [Scalar; 2],
    },
    Circular {
        count: Scalar,
        pcd_mm: Scalar,
    },
    Linear {
        count: Scalar,
        pitch_mm: Scalar,
    },
    Unstructured {
        count: Scalar,
    },
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Requirements {
    pub verification_tolerance: Tolerance,
    pub pattern: PatternRequirement,
    pub bore_diameter_mm: Scalar,
    pub termination: super::holes::Termination,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Resolved {
    pub value: Value,
}
pub(crate) type Definitions = BTreeMap<String, BTreeMap<String, Resolved>>;
fn xyz(v: &Value) -> Vec3 {
    vec(serde_json::from_value(v.clone()).unwrap())
}
fn count(v: &Scalar, p: &BTreeMap<String, f64>) -> Result<usize> {
    let n = v.resolve(p)?;
    if n < 1. || n > 64. || n.fract() != 0. {
        return Err(CadError::new(
            "QUERY_ARGUMENT",
            "interface count must be an integer in 1..64",
        ));
    }
    Ok(n as usize)
}
fn one<T>(values: Vec<T>, label: &str) -> Result<T> {
    if values.is_empty() {
        return Err(CadError::new("INTERFACE_NOT_FOUND", label));
    }
    if values.len() != 1 {
        return Err(CadError::new(
            "INTERFACE_AMBIGUOUS",
            format!("{label}: {} candidates", values.len()),
        ));
    }
    Ok(values.into_iter().next().unwrap())
}
fn cardinality(c: &Cardinality) -> Result<()> {
    if c.min != 1 || c.max != 1 {
        return Err(CadError::new(
            "INTERFACE_UNSUPPORTED",
            "profile v1 requires one support face, group and optional register",
        ));
    }
    Ok(())
}
fn select_face(c: &mut Context, f: &super::contract::Filter) -> Result<u64> {
    let s = Selection::Filter {
        input: Box::new(Selection::Entities { entity: Kind::Face }),
        r#where: f.clone(),
    };
    selection::selection_budget(&s, 1, &mut 0)?;
    one(
        selection::select(c, &s)?.into_iter().collect(),
        "support face",
    )
}
pub(crate) fn validate(m: &Model) -> Result<()> {
    if m.interfaces.len() > 64 {
        return Err(CadError::new(
            "RESOURCE_LIMIT",
            "at most 64 interface declarations",
        ));
    }
    let definitions: BTreeSet<_> = if m.assembly.is_empty() {
        BTreeSet::from([m.output.as_str()])
    } else {
        m.assembly.iter().map(|i| i.input.as_str()).collect()
    };
    let mut seen = BTreeSet::new();
    for d in &m.interfaces {
        if !valid_id(&d.id)
            || !definitions.contains(d.definition.as_str())
            || !seen.insert((d.definition.as_str(), d.id.as_str()))
        {
            return Err(CadError::new(
                "INTERFACE_NOT_FOUND",
                "invalid or duplicate interface owner/name",
            ));
        }
        if d.profile != PROFILE {
            return Err(CadError::new("INTERFACE_UNSUPPORTED", &d.profile));
        }
        if d.datum.normal != "outward" || d.selectors.hole_group.opening_on != "plane" {
            return Err(CadError::new(
                "INTERFACE_UNSUPPORTED",
                "profile requires outward normal and openings bound to plane",
            ));
        }
        super::engine::validate_tolerance(d.requirements.verification_tolerance)?;
        cardinality(&d.selectors.plane.cardinality)?;
        cardinality(&d.selectors.hole_group.cardinality)?;
        selection::filter_budget(&d.selectors.plane.r#where, 1, &mut 0)?;
        if let Some(r) = &d.selectors.register {
            cardinality(&r.cardinality)?;
            selection::filter_budget(&r.r#where, 1, &mut 0)?;
        }
        if d.requirements.bore_diameter_mm.resolve(&m.parameters)? <= 0. {
            return Err(CadError::new(
                "INVALID_DIMENSION",
                "positive bore diameter required",
            ));
        }
        let mut names = BTreeSet::new();
        for item in &d.orientation_datums {
            let name = match item {
                OrientationDatum::Direction { name, vector } => {
                    unit(arr(point(vector, &m.parameters)?))?;
                    name
                }
                OrientationDatum::GeometricDatum { name, datum } => {
                    if !m
                        .datums
                        .iter()
                        .any(|x| x.id == *datum && x.definition == d.definition)
                    {
                        return Err(CadError::new("DATUM_REFERENCE", datum));
                    }
                    name
                }
            };
            if !valid_id(name) || !names.insert(name) {
                return Err(CadError::new(
                    "DATUM_REFERENCE",
                    "orientation names must be unique and valid",
                ));
            }
        }
        if let Some(OrientationName::OrientationDatum { name }) = &d.datum.x_axis {
            if !names.contains(name) {
                return Err(CadError::new("DATUM_REFERENCE", name));
            }
        }
        if let Origin::DeclaredPoint { point: p } = &d.datum.origin {
            point(p, &m.parameters)?;
        }
    }
    if let Some(system) = &m.mates {
        for mate in &system.constraints {
            let (_, a, b) = mate.fields();
            for a in [a, b] {
                if [a.frame.is_some(), a.datum.is_some(), a.interface.is_some()]
                    .into_iter()
                    .filter(|b| *b)
                    .count()
                    > 1
                    || a.orientation.is_some() && a.interface.is_none()
                {
                    return Err(CadError::new("MATE_ATTACHMENT","frame, datum and interface are alternatives; orientation requires interface"));
                }
                if let Some(id) = &a.interface {
                    let i = m
                        .assembly
                        .iter()
                        .find(|i| i.id == a.instance)
                        .ok_or_else(|| CadError::new("MATE_REFERENCE", &a.instance))?;
                    if !seen.contains(&(i.input.as_str(), id.as_str())) {
                        return Err(CadError::new("INTERFACE_NOT_FOUND", id));
                    }
                }
            }
        }
    }
    Ok(())
}
fn verify_pattern(
    group: &Value,
    requirements: &PatternRequirement,
    x: Option<Vec3>,
    p: &BTreeMap<String, f64>,
    tol: Tolerance,
) -> Result<()> {
    let params = &group["parameters"];
    let members = group["members"].as_array().unwrap().len();
    let bad = match requirements {
        PatternRequirement::Rectangular {
            count: counts,
            pitch_mm,
        } => {
            if group["pattern"] != "rectangular" {
                true
            } else {
                let expected = [count(&counts[0], p)?, count(&counts[1], p)?];
                let pitch = [pitch_mm[0].resolve(p)?, pitch_mm[1].resolve(p)?];
                let actual = [
                    params["count"][0].as_u64().unwrap() as usize,
                    params["count"][1].as_u64().unwrap() as usize,
                ];
                let distances = [
                    params["pitch_mm"][0].as_f64().unwrap(),
                    params["pitch_mm"][1].as_f64().unwrap(),
                ];
                let candidates = if let Some(x) = x {
                    let axes = [xyz(&params["axes"][0]), xyz(&params["axes"][1])];
                    (0..2)
                        .filter(|i| {
                            x.cross(axes[*i]).length().atan2(x.dot(axes[*i]).abs())
                                <= tol.angular_rad
                        })
                        .collect::<Vec<_>>()
                } else {
                    vec![0, 1]
                };
                !candidates.into_iter().any(|i| {
                    expected == [actual[i], actual[1 - i]]
                        && (pitch[0] - distances[i]).abs() <= tol.linear_mm
                        && (pitch[1] - distances[1 - i]).abs() <= tol.linear_mm
                })
            }
        }
        PatternRequirement::Circular { count: n, pcd_mm } => {
            group["pattern"] != "circular"
                || members != count(n, p)?
                || (params["pcd_mm"].as_f64().unwrap_or(f64::INFINITY) - pcd_mm.resolve(p)?).abs()
                    > tol.linear_mm
        }
        PatternRequirement::Linear { count: n, pitch_mm } => {
            group["pattern"] != "linear"
                || members != count(n, p)?
                || (params["pitch_mm"].as_f64().unwrap_or(f64::INFINITY) - pitch_mm.resolve(p)?)
                    .abs()
                    > tol.linear_mm
        }
        PatternRequirement::Unstructured { count: n } => members != count(n, p)?,
    };
    if bad {
        return Err(CadError::new(
            "INTERFACE_INCOMPATIBLE",
            format!(
                "pattern requirements do not match measured {}",
                group["parameters"]
            ),
        ));
    }
    Ok(())
}
fn strip_refs(v: &mut Value) {
    if let Some(object) = v.as_object() {
        if object.contains_key("topology_id") && object.contains_key("query_revision_id") {
            let token = v["topology_id"].as_str().unwrap()[1..]
                .parse::<u64>()
                .unwrap();
            *v = json!({"topology_token":token});
            return;
        }
    }
    match v {
        Value::Object(m) => {
            for value in m.values_mut() {
                strip_refs(value)
            }
        }
        Value::Array(a) => {
            for value in a {
                strip_refs(value)
            }
        }
        _ => {}
    }
}
fn materialize(
    v: &mut Value,
    definition: &str,
    manifest: &super::artifact::Manifest,
    pose: placement::Rigid,
    occurrence: &Option<Vec<String>>,
) -> Result<()> {
    if let Some(t) = v.get("topology_token").and_then(Value::as_u64) {
        let def =
            manifest.body.definitions.get(definition).ok_or_else(|| {
                CadError::new("ARTIFACT_INTEGRITY", "interface definition missing")
            })?;
        let binding = def.bindings.get(&t).ok_or_else(|| {
            CadError::new("ARTIFACT_INTEGRITY", "interface topology binding missing")
        })?;
        *v = json!({"query_revision_id":manifest.query_revision_id,"definition_id":definition,"definition_sha256":def.definition_sha256,"kind":binding.kind,"topology_id":format!("t{t}"),"instance_path":occurrence});
        return Ok(());
    }
    match v {
        Value::Object(m) => {
            for (key, value) in m.iter_mut() {
                if [
                    "point_mm",
                    "origin_mm",
                    "center_mm",
                    "arc_start_mm",
                    "arc_end_mm",
                    "plane_intersection_mm",
                ]
                .contains(&key.as_str())
                    && value.is_array()
                {
                    *value = json!(arr(pose.point(xyz(value))));
                } else if [
                    "normal",
                    "direction",
                    "axis_direction",
                    "inward_direction",
                    "x_axis",
                ]
                .contains(&key.as_str())
                    && value.is_array()
                {
                    *value = json!(arr(pose.vector(xyz(value))));
                } else if key == "axes" && value.is_array() {
                    for a in value.as_array_mut().unwrap() {
                        *a = json!(arr(pose.vector(xyz(a))));
                    }
                } else {
                    materialize(value, definition, manifest, pose, occurrence)?;
                }
            }
        }
        Value::Array(a) => {
            for value in a {
                materialize(value, definition, manifest, pose, occurrence)?;
            }
        }
        _ => {}
    }
    Ok(())
}
fn origin_trim(c: &mut Context, t: u64, origin: Vec3) -> Result<Value> {
    let solidcore::EntityRef::Face(face) = c.loaded.entities[&t] else {
        unreachable!()
    };
    let surface = c.loaded.body.face(face)?.surface.as_ref().unwrap().clone();
    let local = c.pose.inverse().point(origin);
    let projection =
        solidcore::query::api_queries::surface_closest_point(&c.kernel, &surface, local)?;
    if projection.closest.distance.abs() > c.tolerance.linear_mm {
        return Ok(json!("outside_trim"));
    }
    let uv = projection.uvs.first().ok_or_else(|| {
        CadError::new(
            "QUERY_UNSUPPORTED",
            "support plane projection returned no UV",
        )
    })?;
    Ok(json!(super::face::classify_uv(
        &c.kernel,
        &mut c.loaded.body,
        face,
        *uv
    )?))
}
fn candidate(c: &mut Context, group: Value, id: &str) -> Result<Value> {
    let normal = group["normal"].clone();
    let origin = group["center_mm"].clone();
    let t = group["support_face"]["topology_id"].as_str().unwrap()[1..]
        .parse::<u64>()
        .unwrap();
    let trim = origin_trim(c, t, xyz(&origin))?;
    Ok(
        json!({"id":id,"profile":PROFILE,"support_face":group["support_face"],"origin_mm":origin,"normal":normal,"origin_rule":"hole_group_center","origin_trim_membership":trim,"group":group,"orientation":null,"orientation_datums":{},"register":null,"continuous_freedoms":["rotation_about_normal"],"provenance":"recognized","engineering_fit":"not_asserted"}),
    )
}
pub(crate) fn resolve(
    c: &mut Context,
    d: &Declaration,
    p: &BTreeMap<String, f64>,
) -> Result<Resolved> {
    c.tolerance = d.requirements.verification_tolerance;
    let plane = select_face(c, &d.selectors.plane.r#where)?;
    let f = c.describe(plane)?;
    if f["surface_type"] != "plane" || f["normal"].is_null() {
        return Err(CadError::new(
            "INTERFACE_UNSUPPORTED",
            "support requires an oriented planar material face",
        ));
    }
    let groups = groups::recognize(c, &[plane], &d.selectors.hole_group.r#where)?;
    if groups.partial {
        return Err(CadError::new(
            "INTERFACE_UNSUPPORTED",
            format!("incomplete hole recognition: {}", json!(groups.diagnostics)),
        ));
    }
    let group = one(groups.groups, "hole group")?;
    let normal = xyz(&f["normal"]);
    let mut result = candidate(c, group, &d.id)?;
    result["recognition_coverage"] =
        json!({"scope":"declared_support_face","complete":true,"diagnostics":groups.diagnostics});
    let mut orientations = serde_json::Map::new();
    for orientation in &d.orientation_datums {
        let (name, axis, source) = match orientation {
            OrientationDatum::Direction { name, vector } => (
                name,
                unit(arr(point(vector, p)?))?,
                "declared_numeric_direction",
            ),
            OrientationDatum::GeometricDatum { name, datum } => (
                name,
                selection::direction_vector(
                    c,
                    &DirectionInput::Datum {
                        datum: datum.clone(),
                    },
                )?,
                "geometry_derived_named_datum",
            ),
        };
        if axis.dot(normal).abs().asin() > c.tolerance.angular_rad {
            return Err(CadError::new(
                "DATUM_DEGENERATE",
                format!("{name}: orientation must lie in the support plane"),
            ));
        }
        let x = unit(arr(axis - normal * axis.dot(normal)))?;
        orientations.insert(
            name.clone(),
            json!({"x_axis":arr(x),"source":source,"name":name}),
        );
    }
    result["orientation_datums"] = json!(orientations);
    if let Some(OrientationName::OrientationDatum { name }) = &d.datum.x_axis {
        result["orientation"] = result["orientation_datums"][name].clone();
    }
    let x = result["orientation"]["x_axis"]
        .as_array()
        .map(|_| xyz(&result["orientation"]["x_axis"]));
    verify_pattern(&result["group"], &d.requirements.pattern, x, p, c.tolerance)?;
    let expected = d.requirements.bore_diameter_mm.resolve(p)?;
    for hole in result["group"]["members"].as_array().unwrap() {
        if (hole["diameter_mm"].as_f64().unwrap() - expected).abs() > c.tolerance.linear_mm
            || hole["termination"] != json!(d.requirements.termination)
        {
            return Err(CadError::new(
                "INTERFACE_INCOMPATIBLE",
                format!("bore size/termination mismatch: {}", hole["hole_id"]),
            ));
        }
    }
    if let Some(selector) = &d.selectors.register {
        let t = select_face(c, &selector.r#where)?;
        let register = c.describe(t)?;
        if register["surface_type"] != "cylinder"
            || register["material_side"] != json!(selector.material_side)
        {
            return Err(CadError::new(
                "INTERFACE_INCOMPATIBLE",
                "register must have the declared cylindrical material side",
            ));
        }
        let axis = xyz(&register["axis"]["direction"]);
        if axis.cross(normal).length().atan2(axis.dot(normal).abs()) > c.tolerance.angular_rad {
            return Err(CadError::new(
                "INTERFACE_INCOMPATIBLE",
                "register axis is not normal to support",
            ));
        }
        let o = xyz(&register["axis"]["point_mm"]);
        let h = (xyz(&f["plane_point_mm"]) - o).dot(normal) / axis.dot(normal);
        let interval = &register["axial_interval_mm"];
        if h < interval[0].as_f64().unwrap() - c.tolerance.linear_mm
            || h > interval[1].as_f64().unwrap() + c.tolerance.linear_mm
        {
            return Err(CadError::new(
                "INTERFACE_INCOMPATIBLE",
                "register does not meet support plane",
            ));
        }
        if (xyz(&result["group"]["center_mm"]) - o)
            .cross(axis)
            .length()
            > c.tolerance.linear_mm
        {
            return Err(CadError::new(
                "INTERFACE_INCOMPATIBLE",
                "register is not coaxial with pattern center",
            ));
        }
        result["register"] = json!({"source_ref":register["ref"],"axis":register["axis"],"radius_mm":register["radius_mm"],"material_side":register["material_side"],"plane_intersection_mm":arr(o+axis*h)});
    }
    let origin = match &d.datum.origin {
        Origin::HoleGroupCenter => xyz(&result["group"]["center_mm"]),
        Origin::DeclaredPoint { point: p0 } => point(p0, p)?,
        Origin::AxisPlaneIntersection { axis_slot } => {
            if axis_slot != "register" || result["register"].is_null() {
                return Err(CadError::new(
                    "INTERFACE_NOT_FOUND",
                    "origin requires register axis",
                ));
            }
            xyz(&result["register"]["plane_intersection_mm"])
        }
    };
    if (origin - xyz(&f["plane_point_mm"])).dot(normal).abs() > c.tolerance.linear_mm {
        return Err(CadError::new(
            "INTERFACE_INCOMPATIBLE",
            "declared origin is not on support plane",
        ));
    }
    result["origin_trim_membership"] = origin_trim(c, plane, origin)?;
    result["origin_mm"] = json!(arr(origin));
    result["origin_rule"] = json!(d.datum.origin);
    result["provenance"] = json!("declared");
    result["requirements"] = json!(d.requirements);
    if x.is_some() {
        result["continuous_freedoms"] = json!([]);
    }
    strip_refs(&mut result);
    Ok(Resolved { value: result })
}
fn transform_geometry(v: &mut Value, pose: placement::Rigid) {
    match v {
        Value::Object(m) => {
            for (key, v) in m {
                if [
                    "point_mm",
                    "origin_mm",
                    "center_mm",
                    "arc_start_mm",
                    "arc_end_mm",
                    "plane_intersection_mm",
                ]
                .contains(&key.as_str())
                    && v.is_array()
                {
                    *v = json!(arr(pose.point(xyz(v))));
                } else if [
                    "normal",
                    "direction",
                    "axis_direction",
                    "inward_direction",
                    "x_axis",
                ]
                .contains(&key.as_str())
                    && v.is_array()
                {
                    *v = json!(arr(pose.vector(xyz(v))));
                } else if key == "axes" && v.is_array() {
                    for axis in v.as_array_mut().unwrap() {
                        *axis = json!(arr(pose.vector(xyz(axis))));
                    }
                } else {
                    transform_geometry(v, pose)
                }
            }
        }
        Value::Array(a) => {
            for v in a {
                transform_geometry(v, pose)
            }
        }
        _ => {}
    }
}
impl Resolved {
    pub(crate) fn transformed(&self, pose: placement::Rigid) -> Value {
        let mut v = self.value.clone();
        transform_geometry(&mut v, pose);
        v
    }
    pub(crate) fn observation(
        &self,
        definition: &str,
        manifest: &super::artifact::Manifest,
        pose: placement::Rigid,
        occurrence: Option<Vec<String>>,
    ) -> Result<Value> {
        let mut v = self.value.clone();
        materialize(&mut v, definition, manifest, pose, &occurrence)?;
        v["query_revision_id"] = json!(manifest.query_revision_id);
        v["definition_id"] = json!(definition);
        Ok(v)
    }
    pub(crate) fn attachment(
        &self,
        orientation: &Option<AttachmentOrientation>,
        allowed: &[&str],
    ) -> Result<operations::Frame> {
        let selected = if let Some(AttachmentOrientation::Datum { name }) = orientation {
            let v = &self.value["orientation_datums"][name];
            if v.is_null() {
                return Err(CadError::new("DATUM_REFERENCE", name));
            }
            v
        } else {
            &self.value["orientation"]
        };
        let o = xyz(&self.value["origin_mm"]);
        let z = xyz(&self.value["normal"]);
        if allowed == ["frame"] && selected.is_null() {
            return Err(CadError::new("INTERFACE_AMBIGUOUS","fixed/prismatic/revolute interface attachment requires an explicit orientation datum"));
        }
        let x = if selected.is_null() {
            let seed = if z.dot(Vec3::X).abs() < 0.8 {
                Vec3::X
            } else {
                Vec3::Y
            };
            unit(arr(seed - z * seed.dot(z)))?
        } else {
            xyz(&selected["x_axis"])
        };
        Ok(operations::Frame {
            origin: arr(o).map(Scalar::Number),
            x_axis: arr(x).map(Scalar::Number),
            y_axis: arr(z.cross(x)).map(Scalar::Number),
        })
    }
}
pub(crate) fn query(
    c: &mut Context,
    ids: &Option<Vec<String>>,
    profile: &Option<String>,
    support: &[TopologyRef],
    filter: &groups::Filter,
) -> Result<(Vec<Value>, bool, Vec<Value>)> {
    if ids.is_some() == profile.is_some() {
        return Err(CadError::new(
            "QUERY_ARGUMENT",
            "interfaces requires exactly one of ids or profile",
        ));
    }
    if let Some(ids) = ids {
        if ids.is_empty()
            || ids.len() > 64
            || ids.iter().collect::<BTreeSet<_>>().len() != ids.len()
        {
            return Err(CadError::new(
                "QUERY_ARGUMENT",
                "interfaces requires 1..64 distinct IDs",
            ));
        }
        let definitions = c
            .manifest
            .body
            .interfaces
            .get(&c.definition)
            .ok_or_else(|| {
                CadError::new(
                    "INTERFACE_NOT_FOUND",
                    "definition has no declared interfaces",
                )
            })?;
        let mut items = vec![];
        for id in ids {
            let d = definitions
                .get(id)
                .ok_or_else(|| CadError::new("INTERFACE_NOT_FOUND", id))?;
            items.push(d.observation(&c.definition, &c.manifest, c.pose, c.occurrence.clone())?);
        }
        return Ok((items, false, vec![]));
    }
    if profile.as_deref() != Some(PROFILE) {
        return Err(CadError::new(
            "QUERY_UNSUPPORTED",
            "interface profile unsupported",
        ));
    }
    let groups = groups::query(c, support, filter)?;
    let mut items = vec![];
    for group in groups.groups {
        let id = group["id"].as_str().unwrap().to_string();
        items.push(candidate(c, group, &id)?);
    }
    Ok((items, groups.partial, groups.diagnostics))
}
