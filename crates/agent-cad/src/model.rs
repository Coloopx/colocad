//! Declarative input contract and bounded scalar evaluation.
use crate::*;
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum Scalar {
    Number(f64),
    Ref(ParamRef),
    Expr(Expression),
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Expression {
    pub op: Arithmetic,
    pub a: Box<Scalar>,
    pub b: Box<Scalar>,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Arithmetic {
    Add,
    Sub,
    Mul,
    Div,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParamRef {
    pub param: String,
}
impl Scalar {
    pub(crate) fn resolve(&self, params: &BTreeMap<String, f64>) -> Result<f64> {
        self.resolve_depth(params, 0)
    }
    fn resolve_depth(&self, params: &BTreeMap<String, f64>, depth: usize) -> Result<f64> {
        if depth > 16 {
            return Err(CadError::new(
                "RESOURCE_LIMIT",
                "expression depth exceeds 16",
            ));
        }
        let n = match self {
            Self::Number(n) => *n,
            Self::Ref(r) => *params
                .get(&r.param)
                .ok_or_else(|| CadError::new("UNKNOWN_PARAMETER", &r.param))?,
            Self::Expr(e) => {
                let a = e.a.resolve_depth(params, depth + 1)?;
                let b = e.b.resolve_depth(params, depth + 1)?;
                match e.op {
                    Arithmetic::Add => a + b,
                    Arithmetic::Sub => a - b,
                    Arithmetic::Mul => a * b,
                    Arithmetic::Div => a / b,
                }
            }
        };
        if !n.is_finite() || n.abs() > MAX_COORD {
            return Err(CadError::new(
                "INVALID_DIMENSION",
                "expected finite value within ±100000 mm",
            ));
        }
        Ok(n)
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OutputKind {
    #[default]
    Solid,
    Surface,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Model {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<preview::PreviewOptions>,
    pub version: u32,
    pub units: String,
    pub parameters: BTreeMap<String, f64>,
    pub features: Vec<Feature>,
    #[serde(default)]
    pub output: String,
    #[serde(default)]
    pub output_kind: OutputKind,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub assets: BTreeMap<String, StepAsset>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assembly: Vec<Instance>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<placement::Group>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mates: Option<mates::MateSystem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub checks: Vec<AssemblyCheck>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StepAsset {
    pub sha256: String,
    pub step: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LoftSection {
    pub center: [Scalar; 3],
    pub radii: [Scalar; 2],
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OnFailure {
    #[default]
    Raise,
    Record {
        reason: String,
        omitted: Vec<String>,
        affects: Vec<String>,
    },
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Feature {
    DraftExtrude {
        id: String,
        profile: Profile,
        #[serde(default)]
        frame: Frame,
        height: Scalar,
        angle_deg: Scalar,
    },
    NameTopology {
        id: String,
        input: String,
        #[serde(default)]
        edges: BTreeMap<String, EdgeSelector>,
        #[serde(default)]
        faces: BTreeMap<String, FaceSelector>,
    },
    SurfaceNetwork {
        id: String,
        u_curves: Vec<Vec<[Scalar; 3]>>,
        v_curves: Vec<Vec<[Scalar; 3]>>,
        u_stations: Vec<f64>,
        v_stations: Vec<f64>,
    },
    SurfacePatch {
        id: String,
        control_points: Vec<Vec<[Scalar; 3]>>,
    },
    SurfaceBridge {
        id: String,
        start: String,
        end: String,
        start_scale: Scalar,
        end_scale: Scalar,
    },
    SurfaceEdit {
        id: String,
        input: String,
        edits: Vec<surfaces::ControlEdit>,
    },
    Thicken {
        id: String,
        input: String,
        thickness: Scalar,
    },
    Sew {
        id: String,
        inputs: Vec<String>,
    },
    InspectionRegion {
        id: String,
        input: String,
        region: regions::RegionKind,
        #[serde(default)]
        index: usize,
        #[serde(default)]
        require_exact: bool,
    },
    Hollow {
        id: String,
        input: String,
        wall: Scalar,
        #[serde(default)]
        cavity_strategy: hollow::Strategy,
    },
    ShellBody {
        #[serde(default = "operations::zero")]
        edge_radius: Scalar,
        id: String,
        profile: Profile,
        height: Scalar,
        wall: Scalar,
        floor: Scalar,
        open_face: OpenFace,
        #[serde(default)]
        frame: Frame,
    },
    LidFor {
        id: String,
        profile: Profile,
        wall: Scalar,
        thickness: Scalar,
        lip_height: Scalar,
        clearance: Scalar,
        #[serde(default)]
        frame: Frame,
    },
    FaceCutout {
        id: String,
        input: String,
        face: engineering::BoxFace,
        center: operations::V2,
        profile: Profile,
        depth: Scalar,
    },
    FaceHoleGrid {
        id: String,
        input: String,
        face: engineering::BoxFace,
        center: operations::V2,
        count: [Scalar; 2],
        spacing: [Scalar; 2],
        radius: Scalar,
        depth: Scalar,
        #[serde(default)]
        counterbore: Option<Counterbore>,
    },
    FaceTool {
        id: String,
        input: String,
        face: engineering::BoxFace,
        center: operations::V2,
        profile: Profile,
        depth: Scalar,
    },
    FaceHoles {
        id: String,
        input: String,
        face: engineering::BoxFace,
        positions: Vec<operations::V2>,
        radius: Scalar,
        depth: Scalar,
        #[serde(default)]
        counterbore: Option<Counterbore>,
    },
    Cutout {
        id: String,
        input: String,
        profile: Profile,
        frame: Frame,
        depth: Scalar,
    },
    Holes {
        id: String,
        input: String,
        frame: Frame,
        positions: Vec<operations::V2>,
        radius: Scalar,
        depth: Scalar,
        #[serde(default)]
        counterbore: Option<Counterbore>,
    },
    Split {
        id: String,
        input: String,
        axis: operations::Axis,
        value: Scalar,
        keep_positive: bool,
    },
    Extrude {
        id: String,
        profile: Profile,
        #[serde(default)]
        holes: Vec<Profile>,
        #[serde(default)]
        frame: Frame,
        height: Scalar,
    },
    Revolve {
        id: String,
        profile: Profile,
        #[serde(default)]
        frame: Frame,
        axis_origin: [Scalar; 3],
        axis: [Scalar; 3],
        angle_deg: Scalar,
    },
    Sweep {
        id: String,
        profile: Profile,
        frame: Frame,
        path: Vec<PathCurve>,
    },
    CurveSweep {
        id: String,
        profile: Profile,
        frame: Frame,
        path: PathCurve,
        #[serde(default)]
        translational: bool,
    },
    Pipe {
        id: String,
        path: PathCurve,
        radius: Scalar,
    },
    ProfileLoft {
        id: String,
        sections: Vec<ProfileSection>,
        degree: usize,
    },
    Place {
        #[serde(default)]
        rotation_order: placement::RotationOrder,
        id: String,
        input: String,
        translation: [Scalar; 3],
        rotation_deg: [Scalar; 3],
    },
    Rotate {
        id: String,
        input: String,
        origin: [Scalar; 3],
        axis: [Scalar; 3],
        angle_deg: Scalar,
    },
    Mirror {
        id: String,
        input: String,
        origin: [Scalar; 3],
        normal: [Scalar; 3],
    },
    Fillet {
        #[serde(default)]
        on_fail: OnFailure,
        id: String,
        input: String,
        select: EdgeSelector,
        radius: Scalar,
    },
    Chamfer {
        #[serde(default)]
        on_fail: OnFailure,
        id: String,
        input: String,
        select: EdgeSelector,
        length: Scalar,
    },
    Shell {
        id: String,
        input: String,
        thickness: Scalar,
        #[serde(default)]
        open_faces: Vec<FaceSelector>,
    },
    Boss {
        id: String,
        origin: [Scalar; 3],
        outer_radius: Scalar,
        height: Scalar,
        hole_radius: Scalar,
        hole_depth: Scalar,
    },
    Pattern {
        id: String,
        input: String,
        positions: Vec<[Scalar; 3]>,
    },
    ComponentCad {
        id: String,
        asset: String,
        size: [Scalar; 3],
        #[serde(default = "operations::zero_vector")]
        frame_rotation_deg: [Scalar; 3],
    },
    ImportStep {
        id: String,
        asset: String,
        #[serde(default)]
        body_index: usize,
        #[serde(default)]
        assembly: bool,
        #[serde(default)]
        definition: bool,
    },
    Loft {
        id: String,
        sections: Vec<LoftSection>,
        degree: usize,
    },
    Box {
        id: String,
        origin: [Scalar; 3],
        size: [Scalar; 3],
    },
    Cone {
        id: String,
        origin: [Scalar; 3],
        base_radius: Scalar,
        top_radius: Scalar,
        height: Scalar,
    },
    Cylinder {
        id: String,
        origin: [Scalar; 3],
        radius: Scalar,
        height: Scalar,
    },
    Sphere {
        id: String,
        center: [Scalar; 3],
        radius: Scalar,
    },
    Cut {
        id: String,
        left: String,
        right: String,
    },
    Union {
        id: String,
        left: String,
        right: String,
    },
    Intersect {
        id: String,
        left: String,
        right: String,
    },
    Translate {
        id: String,
        input: String,
        offset: [Scalar; 3],
    },
    HoleGrid {
        id: String,
        input: String,
        origin: [Scalar; 3],
        radius: Scalar,
        depth: Scalar,
        count: [Scalar; 2],
        spacing: [Scalar; 2],
    },
}
impl Feature {
    pub fn id(&self) -> &str {
        match self {
            Self::DraftExtrude { id, .. }
            | Self::NameTopology { id, .. }
            | Self::SurfaceNetwork { id, .. }
            | Self::SurfacePatch { id, .. }
            | Self::SurfaceBridge { id, .. }
            | Self::SurfaceEdit { id, .. }
            | Self::Thicken { id, .. }
            | Self::Sew { id, .. }
            | Self::InspectionRegion { id, .. }
            | Self::ShellBody { id, .. }
            | Self::Hollow { id, .. }
            | Self::LidFor { id, .. }
            | Self::FaceCutout { id, .. }
            | Self::FaceHoles { id, .. }
            | Self::FaceHoleGrid { id, .. }
            | Self::FaceTool { id, .. }
            | Self::Cutout { id, .. }
            | Self::Holes { id, .. }
            | Self::Split { id, .. }
            | Self::Extrude { id, .. }
            | Self::Revolve { id, .. }
            | Self::Sweep { id, .. }
            | Self::CurveSweep { id, .. }
            | Self::Pipe { id, .. }
            | Self::ProfileLoft { id, .. }
            | Self::Place { id, .. }
            | Self::Rotate { id, .. }
            | Self::Mirror { id, .. }
            | Self::Fillet { id, .. }
            | Self::Chamfer { id, .. }
            | Self::Shell { id, .. }
            | Self::Boss { id, .. }
            | Self::Pattern { id, .. }
            | Self::ComponentCad { id, .. }
            | Self::ImportStep { id, .. }
            | Self::Box { id, .. }
            | Self::Cone { id, .. }
            | Self::Cylinder { id, .. }
            | Self::Sphere { id, .. }
            | Self::Cut { id, .. }
            | Self::Union { id, .. }
            | Self::Intersect { id, .. }
            | Self::Translate { id, .. }
            | Self::HoleGrid { id, .. }
            | Self::Loft { id, .. } => id,
        }
    }
    pub(crate) fn inputs(&self) -> Vec<&str> {
        match self {
            Self::SurfaceBridge { start, end, .. } => vec![start, end],
            Self::Sew { inputs, .. } => inputs.iter().map(String::as_str).collect(),
            Self::NameTopology { input, .. }
            | Self::SurfaceEdit { input, .. }
            | Self::Thicken { input, .. } => vec![input],
            Self::Cut { left, right, .. }
            | Self::Union { left, right, .. }
            | Self::Intersect { left, right, .. } => vec![left, right],
            Self::Hollow { input, .. }
            | Self::InspectionRegion { input, .. }
            | Self::FaceCutout { input, .. }
            | Self::FaceHoles { input, .. }
            | Self::FaceHoleGrid { input, .. }
            | Self::FaceTool { input, .. }
            | Self::Cutout { input, .. }
            | Self::Holes { input, .. }
            | Self::Split { input, .. }
            | Self::Place { input, .. }
            | Self::Rotate { input, .. }
            | Self::Mirror { input, .. }
            | Self::Fillet { input, .. }
            | Self::Chamfer { input, .. }
            | Self::Shell { input, .. }
            | Self::Pattern { input, .. }
            | Self::Translate { input, .. }
            | Self::HoleGrid { input, .. } => vec![input],
            _ => vec![],
        }
    }
}

pub(crate) fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
pub(crate) fn point(v: &[Scalar; 3], p: &BTreeMap<String, f64>) -> Result<Vec3> {
    Ok(Vec3::new(
        v[0].resolve(p)?,
        v[1].resolve(p)?,
        v[2].resolve(p)?,
    ))
}
pub(crate) fn positive(n: f64) -> Result<f64> {
    if n < 1e-4 {
        return Err(CadError::new(
            "INVALID_DIMENSION",
            "size must be at least 0.0001 mm",
        ));
    }
    Ok(n)
}
pub(crate) fn grid_counts(v: &[Scalar; 2], p: &BTreeMap<String, f64>) -> Result<[usize; 2]> {
    let mut out = [0; 2];
    for (i, n) in v.iter().enumerate() {
        let n = n.resolve(p)?;
        if n < 1. || n.fract() != 0. || n > MAX_CUTTERS as f64 {
            return Err(CadError::new(
                "RESOURCE_LIMIT",
                "hole counts must be integers in 1..1024",
            ));
        }
        out[i] = n as usize;
    }
    if out[0] * out[1] > MAX_CUTTERS {
        return Err(CadError::new(
            "RESOURCE_LIMIT",
            "at most 1024 cutters per model",
        ));
    }
    Ok(out)
}

pub fn model_hash(model: &Model) -> String {
    let bytes = serde_json::to_vec(model).expect("validated model is serializable");
    let mut h = Sha256::new();
    h.update(b"agent-cad-ir-v1/kernel-default-tolerances/");
    h.update(KERNEL_REV);
    h.update(EXECUTOR_VERSION);
    h.update(bytes);
    format!("{:x}", h.finalize())
}
