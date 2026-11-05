//! Parametric 2D constraints. Initial coordinates select a local solution branch.
use crate::*;
use operations::{Profile, Segment, V2};
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Circle {
    pub center: String,
    pub radius: Scalar,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Sketch {
    pub points: BTreeMap<String, V2>,
    #[serde(default)]
    pub circles: BTreeMap<String, Circle>,
    pub constraints: Vec<Constraint>,
    pub boundary: Boundary,
    #[serde(default)]
    pub allow_underconstrained: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Boundary {
    Polygon {
        points: Vec<String>,
    },
    Outline {
        start: String,
        segments: Vec<SketchSegment>,
    },
    Circle {
        circle: String,
    },
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SketchSegment {
    Line { to: String },
    Arc { via: String, to: String },
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Constraint {
    Fixed {
        id: String,
        point: String,
        at: V2,
    },
    Coincident {
        id: String,
        a: String,
        b: String,
    },
    Horizontal {
        id: String,
        a: String,
        b: String,
    },
    Vertical {
        id: String,
        a: String,
        b: String,
    },
    Distance {
        id: String,
        a: String,
        b: String,
        value: Scalar,
    },
    Dx {
        id: String,
        a: String,
        b: String,
        value: Scalar,
    },
    Dy {
        id: String,
        a: String,
        b: String,
        value: Scalar,
    },
    Parallel {
        id: String,
        line_a: [String; 2],
        line_b: [String; 2],
    },
    Perpendicular {
        id: String,
        line_a: [String; 2],
        line_b: [String; 2],
    },
    Angle {
        id: String,
        line_a: [String; 2],
        line_b: [String; 2],
        degrees: Scalar,
    },
    EqualLength {
        id: String,
        line_a: [String; 2],
        line_b: [String; 2],
    },
    Midpoint {
        id: String,
        point: String,
        line: [String; 2],
    },
    PointOnLine {
        id: String,
        point: String,
        line: [String; 2],
    },
    Radius {
        id: String,
        circle: String,
        value: Scalar,
    },
    EqualRadius {
        id: String,
        a: String,
        b: String,
    },
    PointOnCircle {
        id: String,
        point: String,
        circle: String,
    },
    Tangent {
        id: String,
        line: [String; 2],
        circle: String,
    },
    CircleTangent {
        id: String,
        a: String,
        b: String,
        #[serde(default)]
        internal: bool,
    },
    Symmetric {
        id: String,
        a: String,
        b: String,
        axis: [String; 2],
    },
}
impl Constraint {
    fn id(&self) -> &str {
        match self {
            Self::Fixed { id, .. }
            | Self::Coincident { id, .. }
            | Self::Horizontal { id, .. }
            | Self::Vertical { id, .. }
            | Self::Distance { id, .. }
            | Self::Dx { id, .. }
            | Self::Dy { id, .. }
            | Self::Parallel { id, .. }
            | Self::Perpendicular { id, .. }
            | Self::Angle { id, .. }
            | Self::EqualLength { id, .. }
            | Self::Midpoint { id, .. }
            | Self::PointOnLine { id, .. }
            | Self::Radius { id, .. }
            | Self::EqualRadius { id, .. }
            | Self::PointOnCircle { id, .. }
            | Self::Tangent { id, .. }
            | Self::CircleTangent { id, .. }
            | Self::Symmetric { id, .. } => id,
        }
    }
}
#[derive(Serialize)]
pub struct SolvedSketch {
    pub points: BTreeMap<String, [f64; 2]>,
    pub radii: BTreeMap<String, f64>,
    pub diagnostics: Value,
    pub profile: Profile,
}
fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}
fn norm(v: [f64; 2]) -> f64 {
    v[0].hypot(v[1])
}
fn dot(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn cross(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
struct Context<'a> {
    sketch: &'a Sketch,
    params: &'a BTreeMap<String, f64>,
    points: BTreeMap<String, usize>,
    radii: BTreeMap<String, usize>,
}
impl Context<'_> {
    fn point(&self, x: &[f64], id: &str) -> Result<[f64; 2]> {
        let i = *self
            .points
            .get(id)
            .ok_or_else(|| CadError::new("SKETCH_REFERENCE", id))?;
        Ok([x[i], x[i + 1]])
    }
    fn radius(&self, x: &[f64], id: &str) -> Result<f64> {
        self.radii
            .get(id)
            .map(|&i| x[i])
            .ok_or_else(|| CadError::new("SKETCH_REFERENCE", id))
    }
    fn center(&self, x: &[f64], id: &str) -> Result<[f64; 2]> {
        let c = self
            .sketch
            .circles
            .get(id)
            .ok_or_else(|| CadError::new("SKETCH_REFERENCE", id))?;
        self.point(x, &c.center)
    }
    fn line(&self, x: &[f64], ids: &[String; 2]) -> Result<[f64; 2]> {
        let d = sub(self.point(x, &ids[1])?, self.point(x, &ids[0])?);
        if norm(d) < 1e-10 {
            return Err(CadError::new(
                "SKETCH_DEGENERATE",
                "zero length line in constraint",
            ));
        }
        Ok(d)
    }
    fn residual(&self, x: &[f64], c: &Constraint) -> Result<Vec<f64>> {
        let p = |s: &str| self.point(x, s);
        let r = |s: &str| self.radius(x, s);
        let center = |s: &str| self.center(x, s);
        let l = |a: &[String; 2]| self.line(x, a);
        let s = |v: &Scalar| v.resolve(self.params);
        Ok(match c {
            Constraint::Fixed { point, at, .. } => {
                let q = p(point)?;
                vec![q[0] - s(&at[0])?, q[1] - s(&at[1])?]
            }
            Constraint::Coincident { a, b, .. } => sub(p(a)?, p(b)?).to_vec(),
            Constraint::Horizontal { a, b, .. } => vec![p(b)?[1] - p(a)?[1]],
            Constraint::Vertical { a, b, .. } => vec![p(b)?[0] - p(a)?[0]],
            Constraint::Distance { a, b, value, .. } => {
                let d = s(value)?;
                if d <= 0. {
                    return Err(CadError::new(
                        "SKETCH_DIMENSION",
                        "distance must be positive; use coincident for zero",
                    ));
                }
                vec![norm(sub(p(a)?, p(b)?)) - d]
            }
            Constraint::Dx { a, b, value, .. } => vec![p(b)?[0] - p(a)?[0] - s(value)?],
            Constraint::Dy { a, b, value, .. } => vec![p(b)?[1] - p(a)?[1] - s(value)?],
            Constraint::Parallel { line_a, line_b, .. } => {
                let (a, b) = (l(line_a)?, l(line_b)?);
                vec![cross(a, b) / (norm(a) * norm(b))]
            }
            Constraint::Perpendicular { line_a, line_b, .. } => {
                let (a, b) = (l(line_a)?, l(line_b)?);
                vec![dot(a, b) / (norm(a) * norm(b))]
            }
            Constraint::Angle {
                line_a,
                line_b,
                degrees,
                ..
            } => {
                let (a, b) = (l(line_a)?, l(line_b)?);
                let angle = cross(a, b).atan2(dot(a, b)) - s(degrees)?.to_radians();
                vec![angle.sin().atan2(angle.cos())]
            }
            Constraint::EqualLength { line_a, line_b, .. } => {
                vec![norm(l(line_a)?) - norm(l(line_b)?)]
            }
            Constraint::Midpoint { point, line, .. } => {
                let (q, a, b) = (p(point)?, p(&line[0])?, p(&line[1])?);
                vec![q[0] - (a[0] + b[0]) / 2., q[1] - (a[1] + b[1]) / 2.]
            }
            Constraint::PointOnLine { point, line, .. } => {
                let d = l(line)?;
                vec![cross(d, sub(p(point)?, p(&line[0])?)) / norm(d)]
            }
            Constraint::Radius { circle, value, .. } => vec![r(circle)? - positive(s(value)?)?],
            Constraint::EqualRadius { a, b, .. } => vec![r(a)? - r(b)?],
            Constraint::PointOnCircle { point, circle, .. } => {
                vec![norm(sub(p(point)?, center(circle)?)) - r(circle)?]
            }
            Constraint::Tangent { line, circle, .. } => {
                let d = l(line)?;
                vec![cross(d, sub(center(circle)?, p(&line[0])?)).abs() / norm(d) - r(circle)?]
            }
            Constraint::CircleTangent { a, b, internal, .. } => vec![
                norm(sub(center(a)?, center(b)?))
                    - if *internal {
                        (r(a)? - r(b)?).abs()
                    } else {
                        r(a)? + r(b)?
                    },
            ],
            Constraint::Symmetric { a, b, axis, .. } => {
                let d = l(axis)?;
                let (a, b) = (p(a)?, p(b)?);
                let mid = [(a[0] + b[0]) / 2., (a[1] + b[1]) / 2.];
                vec![
                    cross(d, sub(mid, p(&axis[0])?)) / norm(d),
                    dot(d, sub(b, a)) / norm(d),
                ]
            }
        })
    }
}
pub fn solve(sketch: &Sketch, params: &BTreeMap<String, f64>) -> Result<SolvedSketch> {
    if sketch.points.is_empty()
        || 2 * sketch.points.len() + sketch.circles.len() > 128
        || sketch.constraints.len() > 256
    {
        return Err(CadError::new(
            "SOLVER_BUDGET",
            "sketch: 1..128 variables, at most 256 constraints",
        ));
    }
    let mut ids = BTreeSet::new();
    for c in &sketch.constraints {
        if !valid_id(c.id()) || !ids.insert(c.id()) {
            return Err(CadError::new("SKETCH_CONSTRAINT_ID", c.id()));
        }
    }
    let mut initial = vec![];
    let mut points = BTreeMap::new();
    let mut radii = BTreeMap::new();
    for (id, v) in &sketch.points {
        if !valid_id(id) {
            return Err(CadError::new("SKETCH_REFERENCE", id));
        }
        points.insert(id.clone(), initial.len());
        initial.extend([v[0].resolve(params)?, v[1].resolve(params)?]);
    }
    for (id, c) in &sketch.circles {
        if !valid_id(id) || !points.contains_key(&c.center) {
            return Err(CadError::new("SKETCH_REFERENCE", id));
        }
        radii.insert(id.clone(), initial.len());
        initial.push(positive(c.radius.resolve(params)?)?);
    }
    let ctx = Context {
        sketch,
        params,
        points,
        radii,
    };
    let result = crate::solve::least_squares(
        initial,
        |x| {
            sketch
                .constraints
                .iter()
                .map(|c| ctx.residual(x, c))
                .collect::<Result<Vec<_>>>()
                .map(|v| v.into_iter().flatten().collect())
        },
        1e-7,
    )?;
    let residuals = sketch
        .constraints
        .iter()
        .map(|c| {
            let r = ctx.residual(&result.values, c)?;
            Ok(json!({"id":c.id(),"residuals":r}))
        })
        .collect::<Result<Vec<_>>>()?;
    let diagnostics = json!({"converged":result.max_residual<=1e-6,"max_residual":result.max_residual,"rank":result.rank,"degrees_of_freedom":result.degrees_of_freedom,"redundant_equations":result.redundant_equations,"iterations":result.iterations,"local_solution":true,"constraints":residuals});
    if result.max_residual > 1e-6 {
        return Err(CadError::new(
            "SKETCH_INCONSISTENT",
            diagnostics.to_string(),
        ));
    }
    if result.degrees_of_freedom > 0 && !sketch.allow_underconstrained {
        return Err(CadError::new(
            "SKETCH_UNDERCONSTRAINED",
            diagnostics.to_string(),
        ));
    }
    let points = ctx
        .points
        .keys()
        .map(|id| Ok((id.clone(), ctx.point(&result.values, id)?)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let radii = ctx
        .radii
        .keys()
        .map(|id| Ok((id.clone(), positive(ctx.radius(&result.values, id)?)?)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let p = |id: &str| -> Result<V2> {
        let q = points
            .get(id)
            .ok_or_else(|| CadError::new("SKETCH_REFERENCE", id))?;
        Ok(q.map(Scalar::Number))
    };
    let profile = match &sketch.boundary {
        Boundary::Polygon { points } => Profile::Polygon {
            points: points.iter().map(|id| p(id)).collect::<Result<_>>()?,
        },
        Boundary::Outline { start, segments } => Profile::Outline {
            start: p(start)?,
            segments: segments
                .iter()
                .map(|s| match s {
                    SketchSegment::Line { to } => Ok(Segment::Line { to: p(to)? }),
                    SketchSegment::Arc { via, to } => Ok(Segment::Arc {
                        via: p(via)?,
                        to: p(to)?,
                    }),
                })
                .collect::<Result<_>>()?,
        },
        Boundary::Circle { circle } => {
            let c = ctx.center(&result.values, circle)?;
            let r = ctx.radius(&result.values, circle)?;
            let v = |x, y| [Scalar::Number(x), Scalar::Number(y)];
            Profile::Outline {
                start: v(c[0] + r, c[1]),
                segments: vec![
                    Segment::Arc {
                        via: v(c[0], c[1] + r),
                        to: v(c[0] - r, c[1]),
                    },
                    Segment::Arc {
                        via: v(c[0], c[1] - r),
                        to: v(c[0] + r, c[1]),
                    },
                ],
            }
        }
    };
    Ok(SolvedSketch {
        points,
        radii,
        diagnostics,
        profile,
    })
}
