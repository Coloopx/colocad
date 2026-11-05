//! Rigid placement shared by model instances, STEP and constraint solving.
use crate::*;
use operations::{Frame, V3};
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RotationOrder {
    #[default]
    Xyz,
    Zyx,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Rigid {
    pub axes: [Vec3; 3],
    pub origin: Vec3,
}
impl Default for Rigid {
    fn default() -> Self {
        Self {
            axes: [Vec3::X, Vec3::Y, Vec3::Z],
            origin: Vec3::ZERO,
        }
    }
}
impl Rigid {
    pub fn rotation(translation: Vec3, r: Vec3, order: RotationOrder) -> Self {
        let apply = |mut p: Vec3| {
            for axis in match order {
                RotationOrder::Xyz => [0, 1, 2],
                RotationOrder::Zyx => [2, 1, 0],
            } {
                let a = [r.x, r.y, r.z][axis].to_radians();
                let (s, c) = a.sin_cos();
                p = match axis {
                    0 => Vec3::new(p.x, c * p.y - s * p.z, s * p.y + c * p.z),
                    1 => Vec3::new(c * p.x + s * p.z, p.y, -s * p.x + c * p.z),
                    _ => Vec3::new(c * p.x - s * p.y, s * p.x + c * p.y, p.z),
                };
            }
            p
        };
        Self {
            axes: [apply(Vec3::X), apply(Vec3::Y), apply(Vec3::Z)],
            origin: translation,
        }
    }
    pub fn vector(self, p: Vec3) -> Vec3 {
        self.axes[0] * p.x + self.axes[1] * p.y + self.axes[2] * p.z
    }
    pub fn point(self, p: Vec3) -> Vec3 {
        self.vector(p) + self.origin
    }
    pub fn compose(self, local: Self) -> Self {
        Self {
            axes: local.axes.map(|v| self.vector(v)),
            origin: self.point(local.origin),
        }
    }
    pub fn inverse(self) -> Self {
        let [x, y, z] = self.axes;
        let mut out = Self {
            axes: [
                Vec3::new(x.x, y.x, z.x),
                Vec3::new(x.y, y.y, z.y),
                Vec3::new(x.z, y.z, z.z),
            ],
            origin: Vec3::ZERO,
        };
        out.origin = out.vector(-self.origin);
        out
    }
    pub fn apply(self, k: &mut Kernel, b: &mut Brep) -> Result<()> {
        operations::matrix(k, b, self.axes, self.origin)
    }
    pub fn json(self) -> Value {
        json!({"origin_mm":[self.origin.x,self.origin.y,self.origin.z],"axes":self.axes.map(|v|[v.x,v.y,v.z])})
    }
}
fn zero() -> V3 {
    [Scalar::Number(0.), Scalar::Number(0.), Scalar::Number(0.)]
}
fn yes() -> bool {
    true
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Pose {
    #[serde(default)]
    pub frame: Option<Frame>,
    #[serde(default = "zero")]
    pub translation: V3,
    #[serde(default = "zero")]
    pub rotation_deg: V3,
    #[serde(default)]
    pub rotation_order: RotationOrder,
    #[serde(default = "yes")]
    pub visible: bool,
}
impl Default for Pose {
    fn default() -> Self {
        Self {
            frame: None,
            translation: zero(),
            rotation_deg: zero(),
            rotation_order: RotationOrder::default(),
            visible: true,
        }
    }
}
impl Pose {
    pub(crate) fn resolve(&self, p: &BTreeMap<String, f64>) -> Result<Rigid> {
        if let Some(f) = &self.frame {
            if point(&self.translation, p)?.length() > 1e-12
                || point(&self.rotation_deg, p)?.length() > 1e-12
            {
                return Err(CadError::new(
                    "INVALID_POSE",
                    "frame and nonzero Euler pose cannot be combined",
                ));
            }
            let (o, x, y) = f.resolve(p)?;
            Ok(Rigid {
                origin: o,
                axes: [x, y, x.cross(y)],
            })
        } else {
            Ok(Rigid::rotation(
                point(&self.translation, p)?,
                point(&self.rotation_deg, p)?,
                self.rotation_order,
            ))
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Poses {
    pub normal: Pose,
    pub internal: Pose,
    pub exploded: Pose,
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PoseMode {
    #[default]
    Normal,
    Internal,
    Exploded,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub id: String,
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub pose: Pose,
}
pub(crate) fn groups(m: &Model) -> Result<BTreeMap<String, Rigid>> {
    if m.groups.len() > 512 {
        return Err(CadError::new(
            "RESOURCE_LIMIT",
            "at most 512 assembly groups",
        ));
    }
    let mut out = BTreeMap::new();
    let mut left = m.groups.iter().collect::<Vec<_>>();
    let mut ids = BTreeSet::new();
    for g in &left {
        if !valid_id(&g.id) || !ids.insert(g.id.clone()) || m.assembly.iter().any(|i| i.id == g.id)
        {
            return Err(CadError::new("INVALID_GROUP", &g.id));
        }
        g.pose.resolve(&m.parameters)?;
    }
    while !left.is_empty() {
        let before = left.len();
        let mut pending = vec![];
        for g in left {
            let parent = match &g.parent {
                None => Rigid::default(),
                Some(id) => match out.get(id) {
                    Some(p) => *p,
                    None => {
                        pending.push(g);
                        continue;
                    }
                },
            };
            out.insert(g.id.clone(), parent.compose(g.pose.resolve(&m.parameters)?));
        }
        left = pending;
        if left.len() == before {
            return Err(CadError::new(
                "ASSEMBLY_CYCLE",
                "cyclic or missing group parent",
            ));
        }
    }
    Ok(out)
}
pub(crate) fn local(
    i: &assembly::Instance,
    p: &BTreeMap<String, f64>,
    mode: PoseMode,
) -> Result<(Rigid, bool)> {
    match &i.poses {
        Some(v) => {
            let pose = match mode {
                PoseMode::Normal => &v.normal,
                PoseMode::Internal => &v.internal,
                PoseMode::Exploded => &v.exploded,
            };
            Ok((pose.resolve(p)?, pose.visible))
        }
        None => Ok((
            Rigid::rotation(
                point(&i.translation, p)?,
                point(&i.rotation_deg, p)?,
                i.rotation_order,
            ),
            true,
        )),
    }
}
pub(crate) fn world(
    m: &Model,
    i: &assembly::Instance,
    mode: PoseMode,
    groups: &BTreeMap<String, Rigid>,
) -> Result<(Rigid, bool)> {
    let (t, v) = local(i, &m.parameters, mode)?;
    let parent = match &i.parent {
        None => Rigid::default(),
        Some(id) => *groups
            .get(id)
            .ok_or_else(|| CadError::new("INVALID_GROUP", id))?,
    };
    Ok((parent.compose(t), v))
}
