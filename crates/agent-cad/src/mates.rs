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
    #[serde(default)]
    pub frame: Frame,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Mate {
    Fixed {
        id: String,
        a: Attachment,
        b: Attachment,
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
    },
    Revolute {
        id: String,
        a: Attachment,
        b: Attachment,
    },
    Prismatic {
        id: String,
        a: Attachment,
        b: Attachment,
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
    fn fields(&self) -> (&str, &Attachment, &Attachment) {
        match self {
            Self::Fixed { id, a, b }
            | Self::Coincident { id, a, b }
            | Self::Concentric { id, a, b }
            | Self::Revolute { id, a, b }
            | Self::Prismatic { id, a, b }
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
    let (o, x, y) = a.frame.resolve(p)?;
    Ok(pose.compose(Rigid {
        origin: o,
        axes: [x, y, x.cross(y)],
    }))
}
fn push(out: &mut Vec<f64>, v: Vec3) {
    out.extend([v.x, v.y, v.z]);
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
        Mate::Fixed { .. } => {
            push(&mut out, d);
            for i in 0..3 {
                push(&mut out, (b.axes[i] - a.axes[i]) * 10.);
            }
        }
        Mate::Coincident { .. } => push(&mut out, d),
        Mate::Concentric { .. } => {
            push(&mut out, b.axes[2] - a.axes[2]);
            push(&mut out, d.cross(a.axes[2]));
        }
        Mate::Revolute { .. } => {
            push(&mut out, d);
            push(&mut out, (b.axes[2] - a.axes[2]) * 10.);
        }
        Mate::Prismatic { .. } => {
            push(&mut out, d.cross(a.axes[2]));
            for i in 0..3 {
                push(&mut out, (b.axes[i] - a.axes[i]) * 10.);
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
pub(crate) fn solve(
    m: &Model,
    initial: BTreeMap<String, Rigid>,
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
    let mut ids = BTreeSet::new();
    for mate in &system.constraints {
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
        system
            .constraints
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
    let constraints = system
        .constraints
        .iter()
        .map(|c| Ok(json!({"id":c.fields().0,"residuals":residual(c,&solved,&m.parameters)?})))
        .collect::<Result<Vec<_>>>()?;
    let report = json!({"converged":solution.max_residual<=1e-6,"max_residual":solution.max_residual,"degrees_of_freedom":solution.degrees_of_freedom,"expected_degrees_of_freedom":system.expected_degrees_of_freedom,"rank":solution.rank,"redundant_equations":solution.redundant_equations,"iterations":solution.iterations,"local_solution":true,"constraints":constraints,"placements":solved.iter().map(|(id,t)|(id.clone(),t.json())).collect::<BTreeMap<_,_>>()});
    if solution.max_residual > 1e-6 {
        return Err(CadError::new("MATE_INCONSISTENT", report.to_string()));
    }
    if solution.degrees_of_freedom != system.expected_degrees_of_freedom {
        return Err(CadError::new("MATE_DEGREES_OF_FREEDOM", report.to_string()));
    }
    Ok((solved, report))
}
