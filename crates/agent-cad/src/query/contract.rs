use crate::*;
#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Face,
    Edge,
    Vertex,
    Loop,
}
impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Face => "face",
            Self::Edge => "edge",
            Self::Vertex => "vertex",
            Self::Loop => "loop",
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TopologyRef {
    pub query_revision_id: String,
    pub definition_id: String,
    pub definition_sha256: String,
    pub kind: Kind,
    pub topology_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_path: Option<Vec<String>>,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
pub enum Target {
    Definition { definition_id: String },
    Instance { instance_path: Vec<String> },
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Space {
    Definition,
    Assembly,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Tolerance {
    pub linear_mm: f64,
    pub angular_rad: f64,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Range {
    pub min: Option<f64>,
    pub max: Option<f64>,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceClass {
    Plane,
    Cylinder,
    Cone,
    Sphere,
    Torus,
    Nurbs,
    Other,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CurveClass {
    Line,
    Circle,
    Ellipse,
    Nurbs,
    Other,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LoopRole {
    Outer,
    Inner,
    Unknown,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Filter {
    pub name: Option<String>,
    pub surface_type: Option<SurfaceClass>,
    pub curve_type: Option<CurveClass>,
    pub area_mm2: Option<Range>,
    pub length_mm: Option<Range>,
    pub radius_mm: Option<Range>,
    pub planar_normal_parallel_to: Option<[f64; 3]>,
    pub planar_normal_aligned_with: Option<[f64; 3]>,
    pub loop_role: Option<LoopRole>,
    #[serde(default)]
    pub all: Vec<Filter>,
    #[serde(default)]
    pub any: Vec<Filter>,
    pub not: Option<Box<Filter>>,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "property", rename_all = "snake_case", deny_unknown_fields)]
pub enum RankBy {
    AreaMm2,
    LengthMm,
    CenterProjection { direction: [f64; 3] },
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Selection {
    Entities {
        entity: Kind,
    },
    Refs {
        refs: Vec<TopologyRef>,
    },
    Filter {
        input: Box<Selection>,
        r#where: Filter,
    },
    Children {
        input: Box<Selection>,
        entity: Kind,
    },
    Ancestors {
        input: Box<Selection>,
        entity: Kind,
    },
    Siblings {
        input: Box<Selection>,
        entity: Kind,
        via: Kind,
    },
    Union {
        inputs: Vec<Selection>,
    },
    Intersection {
        inputs: Vec<Selection>,
    },
    Difference {
        left: Box<Selection>,
        right: Box<Selection>,
    },
    Rank {
        input: Box<Selection>,
        by: RankBy,
        #[serde(default)]
        descending: bool,
        rank: usize,
        tie_tolerance: f64,
    },
    Expect {
        input: Box<Selection>,
        count: usize,
    },
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Derive {
    Point,
    Plane,
    Axis,
    Direction,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    Faces {
        #[serde(default)]
        r#where: Filter,
    },
    Edges {
        #[serde(default)]
        r#where: Filter,
    },
    Vertices {
        #[serde(default)]
        r#where: Filter,
    },
    Loops {
        #[serde(default)]
        r#where: Filter,
    },
    Select {
        selection: Selection,
    },
    Describe {
        refs: Vec<TopologyRef>,
    },
    Datums {
        selection: Selection,
        derive: Derive,
    },
    NamedDatums {
        ids: Vec<String>,
    },
    EvaluateFace {
        r#ref: TopologyRef,
        uv: [f64; 2],
    },
}
impl Operation {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Faces { .. } => "faces",
            Self::Edges { .. } => "edges",
            Self::Vertices { .. } => "vertices",
            Self::Loops { .. } => "loops",
            Self::Select { .. } => "select",
            Self::Describe { .. } => "describe",
            Self::Datums { .. } => "datums",
            Self::EvaluateFace { .. } => "evaluate_face",
            Self::NamedDatums { .. } => "named_datums",
        }
    }
}
fn limit() -> usize {
    64
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub struct Query {
    pub id: String,
    #[serde(flatten)]
    pub operation: Operation,
    #[serde(default = "limit")]
    pub limit: usize,
    #[serde(default)]
    pub cursor: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: String,
    pub query_revision_id: String,
    pub target: Target,
    pub space: Space,
    pub tolerance: Tolerance,
    pub queries: Vec<Query>,
}
