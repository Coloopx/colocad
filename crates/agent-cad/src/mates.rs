//! Local six-DOF rigid assembly solving, followed by exact B-rep checks.
use crate::*;
use operations::Frame;
use placement::Rigid;
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MateSystem {
    pub anchors: Vec<String>,
    pub constraints: Vec<Mate>,
    #[serde(default)]
    pub expected_degrees_of_freedom: usize,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Attachment {
    pub instance: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<Frame>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub datum: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interface: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orientation: Option<query::interfaces::AttachmentOrientation>,
}
/// Surface normals remain outward; a mate separately chooses the axis relation.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AxisAlignment {
    #[default]
    Aligned,
    Opposed,
    Unoriented,
}
impl AxisAlignment {
    fn is_aligned(&self) -> bool {
        *self == Self::Aligned
    }
    fn sign(self) -> Result<f64> {
        match self {
            Self::Aligned => Ok(1.),
            Self::Opposed => Ok(-1.),
            Self::Unoriented => Err(CadError::new(
                "MATE_DIRECTION",
                "unresolved numerical axis branch",
            )),
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FrameAlignment {
    #[default]
    Aligned,
    Opposed,
}
impl FrameAlignment {
    fn is_aligned(&self) -> bool {
        *self == Self::Aligned
    }
}
fn zero_twist(s: &Scalar) -> bool {
    matches!(s, Scalar::Number(n) if *n == 0.)
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Mate {
    Fixed {
        id: String,
        a: Attachment,
        b: Attachment,
        #[serde(default, skip_serializing_if = "FrameAlignment::is_aligned")]
        alignment: FrameAlignment,
        #[serde(default = "operations::zero", skip_serializing_if = "zero_twist")]
        twist_deg: Scalar,
    },
    Coincident {
        id: String,
        a: Attachment,
        b: Attachment,
    },
    Concentric {
        id: String,
        a: Attachment,
        b: Attachment,
        #[serde(default, skip_serializing_if = "AxisAlignment::is_aligned")]
        alignment: AxisAlignment,
    },
    Revolute {
        id: String,
        a: Attachment,
        b: Attachment,
        #[serde(default, skip_serializing_if = "AxisAlignment::is_aligned")]
        alignment: AxisAlignment,
    },
    Prismatic {
        id: String,
        a: Attachment,
        b: Attachment,
        #[serde(default, skip_serializing_if = "FrameAlignment::is_aligned")]
        alignment: FrameAlignment,
        #[serde(default = "operations::zero", skip_serializing_if = "zero_twist")]
        twist_deg: Scalar,
    },
    Plane {
        id: String,
        a: Attachment,
        b: Attachment,
        distance: Scalar,
        #[serde(default)]
        opposed: bool,
    },
    Distance {
        id: String,
        a: Attachment,
        b: Attachment,
        distance: Scalar,
    },
}
impl Mate {
    pub(crate) fn fields(&self) -> (&str, &Attachment, &Attachment) {
        match self {
            Self::Fixed { id, a, b, .. }
            | Self::Coincident { id, a, b }
            | Self::Concentric { id, a, b, .. }
            | Self::Revolute { id, a, b, .. }
            | Self::Prismatic { id, a, b, .. }
            | Self::Plane { id, a, b, .. }
            | Self::Distance { id, a, b, .. } => (id, a, b),
        }
    }
}
fn exp(v: Vec3) -> [Vec3; 3] {
    let a = v.length();
    if a < 1e-16 {
        return [Vec3::X, Vec3::Y, Vec3::Z];
    }
    let u = v / a;
    let (s, c) = a.sin_cos();
    let f = |p| p * c + u.cross(p) * s + u * (u.dot(p) * (1. - c));
    [f(Vec3::X), f(Vec3::Y), f(Vec3::Z)]
}
fn frame(
    a: &Attachment,
    poses: &BTreeMap<String, Rigid>,
    p: &BTreeMap<String, f64>,
) -> Result<Rigid> {
    let pose = poses
        .get(&a.instance)
        .ok_or_else(|| CadError::new("MATE_REFERENCE", &a.instance))?;
    let (o, x, y) = a.frame.as_ref().unwrap_or(&Frame::default()).resolve(p)?;
    Ok(pose.compose(Rigid {
        origin: o,
        axes: [x, y, x.cross(y)],
    }))
}
fn push(out: &mut Vec<f64>, v: Vec3) {
    out.extend([v.x, v.y, v.z]);
}
fn target_axes(
    a: &Rigid,
    alignment: FrameAlignment,
    twist: &Scalar,
    p: &BTreeMap<String, f64>,
) -> Result<[Vec3; 3]> {
    let (s, c) = twist.resolve(p)?.to_radians().sin_cos();
    let x = a.axes[0] * c + a.axes[1] * s;
    let z = a.axes[2]
        * if alignment == FrameAlignment::Opposed {
            -1.
        } else {
            1.
        };
    Ok([x, z.cross(x), z])
}
fn residual(
    m: &Mate,
    poses: &BTreeMap<String, Rigid>,
    p: &BTreeMap<String, f64>,
) -> Result<Vec<f64>> {
    let (_, a, b) = m.fields();
    let a = frame(a, poses, p)?;
    let b = frame(b, poses, p)?;
    let d = b.origin - a.origin;
    let mut out = vec![];
    match m {
        Mate::Fixed {
            alignment,
            twist_deg,
            ..
        } => {
            push(&mut out, d);
            let target = target_axes(&a, *alignment, twist_deg, p)?;
            for i in 0..3 {
                push(&mut out, (b.axes[i] - target[i]) * 10.);
            }
        }
        Mate::Coincident { .. } => push(&mut out, d),
        Mate::Concentric { alignment, .. } => {
            push(&mut out, b.axes[2] - a.axes[2] * alignment.sign()?);
            push(&mut out, d.cross(a.axes[2]));
        }
        Mate::Revolute { alignment, .. } => {
            push(&mut out, d);
            push(&mut out, (b.axes[2] - a.axes[2] * alignment.sign()?) * 10.);
        }
        Mate::Prismatic {
            alignment,
            twist_deg,
            ..
        } => {
            push(&mut out, d.cross(a.axes[2]));
            let target = target_axes(&a, *alignment, twist_deg, p)?;
            for i in 0..3 {
                push(&mut out, (b.axes[i] - target[i]) * 10.);
            }
        }
        Mate::Plane {
            distance, opposed, ..
        } => {
            let normal = if *opposed { -a.axes[2] } else { a.axes[2] };
            push(&mut out, (b.axes[2] - normal) * 10.);
            out.push(d.dot(a.axes[2]) - distance.resolve(p)?);
        }
        Mate::Distance { distance, .. } => {
            out.push(d.length() - positive(distance.resolve(p)?)?);
        }
    }
    Ok(out)
}
fn physical_residual(
    m: &Mate,
    poses: &BTreeMap<String, Rigid>,
    p: &BTreeMap<String, f64>,
) -> Result<Value> {
    let (_, aa, bb) = m.fields();
    let a = frame(aa, poses, p)?;
    let b = frame(bb, poses, p)?;
    let d = b.origin - a.origin;
    let angle = |u: Vec3, v: Vec3| u.dot(v).clamp(-1., 1.).acos();
    let (linear, angular) = match m {
        Mate::Fixed {
            alignment,
            twist_deg,
            ..
        }
        | Mate::Prismatic {
            alignment,
            twist_deg,
            ..
        } => {
            let target = target_axes(&a, *alignment, twist_deg, p)?;
            let trace = (0..3).map(|i| target[i].dot(b.axes[i])).sum::<f64>();
            let linear = if matches!(m, Mate::Fixed { .. }) {
                d.length()
            } else {
                d.cross(a.axes[2]).length()
            };
            (linear, Some(((trace - 1.) / 2.).clamp(-1., 1.).acos()))
        }
        Mate::Concentric { alignment, .. } | Mate::Revolute { alignment, .. } => {
            let linear = if matches!(m, Mate::Revolute { .. }) {
                d.length()
            } else {
                d.cross(a.axes[2]).length()
            };
            (
                linear,
                Some(angle(a.axes[2] * alignment.sign()?, b.axes[2])),
            )
        }
        Mate::Plane {
            distance, opposed, ..
        } => (
            (d.dot(a.axes[2]) - distance.resolve(p)?).abs(),
            Some(angle(
                a.axes[2] * if *opposed { -1. } else { 1. },
                b.axes[2],
            )),
        ),
        Mate::Coincident { .. } => (d.length(), None),
        Mate::Distance { distance, .. } => ((d.length() - distance.resolve(p)?).abs(), None),
    };
    Ok(json!({"linear_mm":linear,"angular_rad":angular}))
}
pub(crate) fn solve(
    m: &Model,
    initial: BTreeMap<String, Rigid>,
    datums: &query::datums::Definitions,
    interfaces: &query::interfaces::Definitions,
) -> Result<(BTreeMap<String, Rigid>, Value)> {
    let Some(system) = &m.mates else {
        return Ok((initial, Value::Null));
    };
    if system.anchors.is_empty() || system.constraints.is_empty() || system.constraints.len() > 64 {
        return Err(CadError::new(
            "MATE_BUDGET",
            "requires anchors and 1..64 constraints",
        ));
    }
    let mut anchors = BTreeSet::new();
    for id in &system.anchors {
        if !initial.contains_key(id) || !anchors.insert(id) {
            return Err(CadError::new("MATE_REFERENCE", id));
        }
    }
    let mut constraints = system.constraints.clone();
    for mate in &mut constraints {
        let allowed: &[&str] = match mate {
            Mate::Fixed { .. } | Mate::Prismatic { .. } | Mate::Revolute { .. } => &["frame"],
            Mate::Concentric { .. } => &["axis", "frame"],
            Mate::Plane { .. } => &["plane", "frame"],
            Mate::Coincident { .. } | Mate::Distance { .. } => &["point", "frame"],
        };
        let (a, b) = match mate {
            Mate::Fixed { a, b, .. }
            | Mate::Prismatic { a, b, .. }
            | Mate::Revolute { a, b, .. }
            | Mate::Concentric { a, b, .. }
            | Mate::Plane { a, b, .. }
            | Mate::Coincident { a, b, .. }
            | Mate::Distance { a, b, .. } => (a, b),
        };
        for attachment in [a, b] {
            if let Some(id) = &attachment.interface {
                let instance = m
                    .assembly
                    .iter()
                    .find(|i| i.id == attachment.instance)
                    .ok_or_else(|| CadError::new("MATE_REFERENCE", &attachment.instance))?;
                let d = interfaces
                    .get(&instance.input)
                    .and_then(|ds| ds.get(id))
                    .ok_or_else(|| CadError::new("INTERFACE_NOT_FOUND", id))?;
                attachment.frame = Some(d.attachment(&attachment.orientation, allowed)?);
                attachment.interface = None;
                attachment.orientation = None;
            }
            if let Some(id) = &attachment.datum {
                let instance = m
                    .assembly
                    .iter()
                    .find(|i| i.id == attachment.instance)
                    .ok_or_else(|| CadError::new("MATE_REFERENCE", &attachment.instance))?;
                let d = datums
                    .get(&instance.input)
                    .and_then(|ds| ds.get(id))
                    .ok_or_else(|| CadError::new("DATUM_REFERENCE", id))?;
                if !allowed.contains(&d.kind.as_str()) {
                    return Err(CadError::new(
                        "DATUM_KIND_MISMATCH",
                        format!("{id}: {} cannot supply {:?}", d.kind, allowed),
                    ));
                }
                attachment.frame = Some(d.numerical_frame()?);
                attachment.datum = None;
            }
        }
    }
    let mut branches = BTreeMap::new();
    for mate in &mut constraints {
        if let Mate::Concentric {
            id,
            a,
            b,
            alignment,
        }
        | Mate::Revolute {
            id,
            a,
            b,
            alignment,
        } = mate
        {
            if *alignment == AxisAlignment::Unoriented {
                let aa = frame(a, &initial, &m.parameters)?;
                let bb = frame(b, &initial, &m.parameters)?;
                *alignment = if aa.axes[2].dot(bb.axes[2]) < 0. {
                    AxisAlignment::Opposed
                } else {
                    AxisAlignment::Aligned
                };
                branches.insert(id.clone(), *alignment);
            }
        }
    }
    let mut ids = BTreeSet::new();
    for mate in &constraints {
        let (id, a, b) = mate.fields();
        if !valid_id(id) || !ids.insert(id) || a.instance == b.instance {
            return Err(CadError::new("MATE_REFERENCE", id));
        }
        residual(mate, &initial, &m.parameters)?;
    }
    let floating = initial
        .keys()
        .filter(|id| !anchors.contains(id))
        .cloned()
        .collect::<Vec<_>>();
    if floating.len() > 21 {
        return Err(CadError::new("MATE_BUDGET", "at most 21 movable instances"));
    }
    let poses = |values: &[f64]| {
        let mut poses = initial.clone();
        for (i, id) in floating.iter().enumerate() {
            let x = &values[i * 6..i * 6 + 6];
            let delta = Rigid {
                axes: exp(Vec3::new(x[3], x[4], x[5])),
                origin: Vec3::ZERO,
            };
            let base = initial[id];
            poses.insert(
                id.clone(),
                Rigid {
                    axes: base.axes.map(|v| delta.vector(v)),
                    origin: base.origin + Vec3::new(x[0], x[1], x[2]),
                },
            );
        }
        poses
    };
    let f = |x: &[f64]| {
        let poses = poses(x);
        constraints
            .iter()
            .map(|c| residual(c, &poses, &m.parameters))
            .collect::<Result<Vec<_>>>()
            .map(|v| v.into_iter().flatten().collect::<Vec<_>>())
    };
    let solution = if floating.is_empty() {
        let r = f(&[])?;
        crate::solve::Solution {
            values: vec![],
            iterations: 0,
            rank: 0,
            degrees_of_freedom: 0,
            redundant_equations: r.len(),
            max_residual: r.iter().fold(0_f64, |a, b| a.max(b.abs())),
        }
    } else {
        crate::solve::least_squares(vec![0.; floating.len() * 6], f, 1e-7)?
    };
    let solved = poses(&solution.values);
    let constraint_reports = constraints
        .iter()
        .map(|c| Ok(json!({"id":c.fields().0,"residuals":residual(c,&solved,&m.parameters)?,"physical_residual":physical_residual(c,&solved,&m.parameters)?,"unoriented_branch":branches.get(c.fields().0)})))
        .collect::<Result<Vec<_>>>()?;
    let report = json!({"converged":solution.max_residual<=1e-6,"max_residual":solution.max_residual,"degrees_of_freedom":solution.degrees_of_freedom,"expected_degrees_of_freedom":system.expected_degrees_of_freedom,"rank":solution.rank,"redundant_equations":solution.redundant_equations,"iterations":solution.iterations,"local_solution":true,"constraints":constraint_reports,"placements":solved.iter().map(|(id,t)|(id.clone(),t.json())).collect::<BTreeMap<_,_>>()});
    if solution.max_residual > 1e-6 {
        return Err(CadError::new("MATE_INCONSISTENT", report.to_string()));
    }
    if solution.degrees_of_freedom != system.expected_degrees_of_freedom {
        return Err(CadError::new("MATE_DEGREES_OF_FREEDOM", report.to_string()));
    }
    Ok((solved, report))
}
