use super::{artifact::Manifest, contract::*, snapshot::LoadedDefinition};
use crate::*;
use solidcore::curve::curve::CurveKind;
use solidcore::mass::integrator::properties::{edge_length, face_centroid};
use solidcore::nurbs::b_spline_surface_n_lib::p2::OrientType;
use solidcore::query::api_queries::*;
use solidcore::surface::surface::SurfaceKind;
use solidcore::EntityRef;
use std::path::Path;

pub(crate) fn vec(v: [f64; 3]) -> Vec3 {
    Vec3::new(v[0], v[1], v[2])
}
pub(crate) fn arr(v: Vec3) -> [f64; 3] {
    [v.x, v.y, v.z]
}
pub(crate) fn unit(v: [f64; 3]) -> Result<Vec3> {
    let v = vec(v);
    if ![v.x, v.y, v.z].iter().all(|x| x.is_finite()) || v.length() < 1e-12 {
        return Err(CadError::new(
            "QUERY_ARGUMENT",
            "direction must be finite and nonzero",
        ));
    }
    Ok(v / v.length())
}
fn canonical(v: Vec3) -> Vec3 {
    if [v.x, v.y, v.z]
        .into_iter()
        .find(|x| x.abs() > 1e-12)
        .unwrap_or(1.)
        < 0.
    {
        -v
    } else {
        v
    }
}
pub(crate) struct Context {
    pub kernel: Kernel,
    pub loaded: LoadedDefinition,
    pub manifest: Manifest,
    pub definition: String,
    pub occurrence: Option<Vec<String>>,
    pub pose: placement::Rigid,
    pub tolerance: Tolerance,
    pub cache: BTreeMap<u64, Value>,
    pub visited: usize,
    pub reverse: BTreeMap<EntityRef, u64>,
    pub solid: bool,
    pub probes: usize,
    pub hypotheses: usize,
    pub holes: Option<(Tolerance, super::holes::Recognition)>,
    pub incidents:
        Option<BTreeMap<solidcore::EdgeId, Vec<(solidcore::FaceId, solidcore::LoopId, bool)>>>,
}
impl Context {
    pub fn kind(&self, token: u64) -> Kind {
        match self.loaded.entities[&token] {
            EntityRef::Face(_) => Kind::Face,
            EntityRef::Edge(_) => Kind::Edge,
            EntityRef::Vertex(_) => Kind::Vertex,
            EntityRef::Loop(_) => Kind::Loop,
            _ => unreachable!(),
        }
    }
    pub fn tokens(&self, kind: Kind) -> Vec<u64> {
        self.loaded
            .entities
            .keys()
            .copied()
            .filter(|t| self.kind(*t) == kind)
            .collect()
    }
    pub fn token(&self, e: EntityRef) -> Result<u64> {
        self.reverse
            .get(&e)
            .copied()
            .ok_or_else(|| CadError::new("ARTIFACT_INTEGRITY", "unbound topology"))
    }
    pub fn reference(&self, t: u64) -> TopologyRef {
        TopologyRef {
            query_revision_id: self.manifest.query_revision_id.clone(),
            definition_id: self.definition.clone(),
            definition_sha256: self.manifest.body.definitions[&self.definition]
                .definition_sha256
                .clone(),
            kind: self.kind(t),
            topology_id: format!("t{t}"),
            instance_path: self.occurrence.clone(),
        }
    }
    pub fn resolve(&self, r: &TopologyRef) -> Result<u64> {
        if r.query_revision_id != self.manifest.query_revision_id {
            return Err(CadError::new(
                "QUERY_REVISION_MISMATCH",
                "topology ref belongs to another revision",
            ));
        }
        if r.definition_id != self.definition
            || r.definition_sha256
                != self.manifest.body.definitions[&self.definition].definition_sha256
            || r.instance_path
                .as_ref()
                .is_some_and(|p| Some(p) != self.occurrence.as_ref())
        {
            return Err(CadError::new(
                "TOPOLOGY_REF_INVALID",
                "reference owner does not match query target",
            ));
        }
        let t = r
            .topology_id
            .strip_prefix('t')
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|t| self.loaded.entities.contains_key(t))
            .ok_or_else(|| CadError::new("TOPOLOGY_REF_INVALID", "unknown topology ID"))?;
        if r.topology_id != format!("t{t}") || self.kind(t) != r.kind {
            return Err(CadError::new(
                "TOPOLOGY_REF_INVALID",
                "topology kind/ID mismatch",
            ));
        }
        Ok(t)
    }
    pub fn children(&mut self, t: u64, kind: Kind) -> Result<Vec<u64>> {
        self.visited += 1;
        if self.visited > 200_000 {
            return Err(CadError::new(
                "QUERY_LIMIT",
                "adjacency traversal budget exceeded",
            ));
        }
        let b = &self.loaded.body;
        let edges = match self.loaded.entities[&t] {
            EntityRef::Face(f) => {
                let f = b.face(f)?;
                if kind == Kind::Loop {
                    return f
                        .loops
                        .iter()
                        .map(|l| self.token(EntityRef::Loop(*l)))
                        .collect();
                }
                f.loops
                    .iter()
                    .flat_map(|l| b.loop_(*l).unwrap().edges.iter().map(|e| e.edge))
                    .collect::<Vec<_>>()
            }
            EntityRef::Loop(l) => b.loop_(l)?.edges.iter().map(|e| e.edge).collect(),
            EntityRef::Edge(e) => vec![e],
            _ => vec![],
        };
        let entities = match kind {
            Kind::Edge if self.kind(t) != Kind::Edge => {
                edges.into_iter().map(EntityRef::Edge).collect::<Vec<_>>()
            }
            Kind::Vertex => edges
                .iter()
                .flat_map(|e| b.edge(*e).unwrap().vertices)
                .map(EntityRef::Vertex)
                .collect(),
            _ => vec![],
        };
        self.visited += entities.len();
        if self.visited > 200_000 {
            return Err(CadError::new(
                "QUERY_LIMIT",
                "adjacency traversal budget exceeded",
            ));
        }
        entities
            .into_iter()
            .map(|e| self.token(e))
            .collect::<Result<BTreeSet<_>>>()
            .map(|s| s.into_iter().collect())
    }
    pub(crate) fn oriented_sign(&self, id: solidcore::FaceId) -> Result<Option<f64>> {
        let b = &self.loaded.body;
        if !self.solid {
            return Ok(None);
        }
        let Some(fu) = b.face(id)?.faceuse else {
            return Ok(None);
        };
        for f in [Some(fu), b.faceuse(fu)?.mate].into_iter().flatten() {
            let use_ = b.faceuse(f)?;
            if let Some(shell) = use_.shell {
                if let Some(region) = b.shell(shell)?.region {
                    if b.region(region)?.is_void() == 0 {
                        return Ok(match use_.orientation {
                            // A native face-use normal points into its owning
                            // region. On the material side this is inward;
                            // engineering outward is the opposite direction.
                            OrientType::Same => Some(-1.),
                            OrientType::Opposite => Some(1.),
                            _ => None,
                        });
                    }
                }
            }
        }
        Ok(None)
    }
    fn bounds(&self, min: Vec3, max: Vec3) -> Value {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for x in [min.x, max.x] {
            for y in [min.y, max.y] {
                for z in [min.z, max.z] {
                    for (i, v) in arr(self.pose.point(Vec3::new(x, y, z)))
                        .into_iter()
                        .enumerate()
                    {
                        lo[i] = lo[i].min(v);
                        hi[i] = hi[i].max(v);
                    }
                }
            }
        }
        json!([lo, hi])
    }
    pub fn evaluate_face(&mut self, reference: &TopologyRef, uv: [f64; 2]) -> Result<Value> {
        let token = self.resolve(reference)?;
        let EntityRef::Face(id) = self.loaded.entities[&token] else {
            return Err(CadError::new(
                "QUERY_ARGUMENT",
                "evaluate_face requires a face reference",
            ));
        };
        if uv.iter().any(|v| !v.is_finite() || v.abs() > 1e8) {
            return Err(CadError::new(
                "QUERY_ARGUMENT",
                "UV must be finite and within ±1e8",
            ));
        }
        let f = self.loaded.body.face(id)?.clone();
        let uv = solidcore::Point2::new(uv[0], uv[1]);
        if !f.uv_domain.contains_point(uv, 0.) {
            return Ok(
                json!({"ref":self.reference(token),"trim_status":"outside_trim","point_mm":null,"normal":null,"evaluation_status":"outside_parameter_domain"}),
            );
        }
        let status = super::face::classify_uv(&self.kernel, &mut self.loaded.body, id, uv)?;
        let surface = f
            .surface
            .as_ref()
            .ok_or_else(|| CadError::new("QUERY_UNSUPPORTED", "face has no surface"))?;
        let values = surface.evaluate(
            &self.kernel,
            uv,
            1,
            1,
            solidcore::nurbs::b_spline_surface_n_lib::p2::EvaluateFlags {
                non_zero_tangents: false,
                ..Default::default()
            },
        )?;
        let point = values[0][0];
        let du = values[1][0];
        let dv = values[0][1];
        let cross = du.cross(dv);
        let length = cross.length();
        if arr(point).iter().any(|v| !v.is_finite()) {
            return Err(CadError::new(
                "QUERY_UNSUPPORTED",
                "non-finite surface evaluation",
            ));
        }
        let sign = self.oriented_sign(id)?;
        let singular =
            !length.is_finite() || length <= 1e-12 * du.length().max(1.) * dv.length().max(1.);
        let normal = if singular {
            None
        } else {
            sign.map(|s| arr(self.pose.vector(cross / length * s)))
        };
        Ok(
            json!({"ref":self.reference(token),"uv":[uv.x,uv.y],"trim_status":status,"point_mm":arr(self.pose.point(point)),"normal":normal,"orientation":if sign.is_some(){"material_outward"}else{"unknown"},"evaluation_status":if singular{"singular_normal"}else{"evaluated"},"point_on_material_face":status!=super::face::TrimStatus::OutsideTrim}),
        )
    }
    pub fn describe(&mut self, t: u64) -> Result<Value> {
        if let Some(v) = self.cache.get(&t) {
            return Ok(v.clone());
        }
        let e = self.loaded.entities[&t];
        let mut v = json!({"ref":self.reference(t),"names":crate::naming::names(&self.loaded.body,e),"kind":self.kind(t)});
        match e {
            EntityRef::Face(id) => {
                let f = self.loaded.body.face(id)?.clone();
                let surf = f
                    .surface
                    .as_ref()
                    .ok_or_else(|| CadError::new("QUERY_UNSUPPORTED", "face has no surface"))?;
                let (area, center) = face_centroid(&self.kernel, &mut self.loaded.body, id, 1e-8)?;
                let bb = face_bounding_box(&self.kernel, &self.loaded.body, id, true)?;
                v["area_mm2"] = json!(area.abs());
                v["center_mm"] = json!(arr(self.pose.point(center)));
                v["bbox_mm"] = self.bounds(bb.min(), bb.max());
                v["bounds_semantics"] = json!("rigidly_transformed_definition_aabb");
                v["loops"] = json!(f
                    .loops
                    .iter()
                    .map(|l| self.token(EntityRef::Loop(*l)).map(|t| self.reference(t)))
                    .collect::<Result<Vec<_>>>()?);
                v["edges"] = json!(self
                    .children(t, Kind::Edge)?
                    .into_iter()
                    .map(|t| self.reference(t))
                    .collect::<Vec<_>>());
                let sign = self.oriented_sign(id)?;
                v["orientation"] = json!(if sign.is_some() {
                    "material_outward"
                } else {
                    "unknown"
                });
                v["uv_domain"] = json!([
                    [f.uv_domain.min().x, f.uv_domain.min().y],
                    [f.uv_domain.max().x, f.uv_domain.max().y]
                ]);
                v["surface_type"] = json!(match surf.kind() {
                    SurfaceKind::Plane(_) => "plane",
                    SurfaceKind::Cone(c) if c.is_cylinder() => "cylinder",
                    SurfaceKind::Cone(_) => "cone",
                    SurfaceKind::BSpline(_) => "nurbs",
                    SurfaceKind::Sphere(_) => "sphere",
                    SurfaceKind::Torus(_) => "torus",
                    _ => "other",
                });
                match surf.kind() {
                    SurfaceKind::Plane(p) => {
                        v["plane_point_mm"] = json!(arr(self.pose.point(p.position().origin())));
                        v["normal"] =
                            json!(sign.map(|s| arr(self.pose.vector(p.evaluate_normal() * s))));
                    }
                    SurfaceKind::Cone(c) => {
                        let canon = c.canonical();
                        v["axis"] = json!({"point_mm":arr(self.pose.point(canon.origin.origin())),"direction":arr(self.pose.vector(canonical(canon.origin.z_axis()))),"direction_semantics":"unoriented_definition_canonical"});
                        v["radius_mm"] = json!(c.base_radius());
                        v["trim_coverage_status"] = json!("not_computed");
                    }
                    _ => {
                        v["specialized_parameters_status"] = json!("unsupported");
                    }
                }
            }
            EntityRef::Edge(id) => {
                let edge = self.loaded.body.edge(id)?;
                let curve = edge
                    .curve
                    .as_ref()
                    .ok_or_else(|| CadError::new("QUERY_UNSUPPORTED", "edge has no curve"))?;
                let bb = edge_bounding_box(&self.kernel, &self.loaded.body, id, true)?;
                v["bbox_mm"] = self.bounds(bb.min(), bb.max());
                v["bounds_semantics"] = json!("rigidly_transformed_definition_aabb");
                v["length_mm"] = json!(edge_length(&self.kernel, &self.loaded.body, id, 1e-8)?);
                v["center_mm"] = json!(arr(self.pose.point((bb.min() + bb.max()) / 2.)));
                v["center_semantics"] = json!("bounding_box_midpoint");
                v["vertices"] = json!(edge
                    .vertices
                    .iter()
                    .map(|i| self.token(EntityRef::Vertex(*i)).map(|t| self.reference(t)))
                    .collect::<Result<Vec<_>>>()?);
                v["parameter_range"] = json!([edge.interval.min(), edge.interval.max()]);
                v["closed"] = json!(edge.vertices[0] == edge.vertices[1]);
                v["curve_type"] = json!(match curve.kind() {
                    CurveKind::Line(_) => "line",
                    CurveKind::Ellipse(c) if c.is_circle_class() => "circle",
                    CurveKind::Ellipse(_) => "ellipse",
                    CurveKind::BSpline(_) => "nurbs",
                    _ => "other",
                });
                match curve.kind() {
                    CurveKind::Ellipse(c) if c.is_circle_class() => {
                        v["radius_mm"] = json!(c.radius());
                        v["center_mm"] = json!(arr(self.pose.point(c.position().origin())));
                        v["center_semantics"] = json!("analytic_circle_center");
                        v["axis_direction"] =
                            json!(arr(self.pose.vector(canonical(c.position().z_axis()))));
                    }
                    CurveKind::Line(_) => {
                        let a = vertex_get_point(&self.loaded.body, edge.vertices[0])?;
                        let b = vertex_get_point(&self.loaded.body, edge.vertices[1])?;
                        v["direction"] = json!(arr(self.pose.vector(unit(arr(b - a))?)));
                    }
                    _ => {}
                }
            }
            EntityRef::Vertex(id) => {
                v["point_mm"] = json!(arr(self
                    .pose
                    .point(vertex_get_point(&self.loaded.body, id)?)));
                v["center_mm"] = v["point_mm"].clone();
            }
            EntityRef::Loop(id) => {
                let l = self.loaded.body.loop_(id)?;
                let face = self.loaded.body.face(l.face)?;
                v["face"] = json!(self.reference(self.token(EntityRef::Face(l.face))?));
                v["role"] = json!(if face.loops.first() == Some(&id) {
                    "outer"
                } else {
                    "inner"
                });
                v["edge_uses"]=json!(l.edges.iter().map(|e|Ok(json!({"edge":self.reference(self.token(EntityRef::Edge(e.edge))?),"orientation":if e.forward{"same"}else{"opposite"}}))).collect::<Result<Vec<_>>>()?);
            }
            _ => unreachable!(),
        }
        self.enrich(e, &mut v)?;
        self.cache.insert(t, v.clone());
        Ok(v)
    }
}

pub(crate) fn validate_tolerance(tol: Tolerance) -> Result<()> {
    if !tol.linear_mm.is_finite()
        || !tol.angular_rad.is_finite()
        || tol.linear_mm <= 0.
        || tol.angular_rad <= 0.
        || tol.angular_rad > std::f64::consts::PI
    {
        return Err(CadError::new("QUERY_ARGUMENT", "invalid query tolerance"));
    }
    if tol.linear_mm < 1e-8 || tol.angular_rad < 1e-10 {
        return Err(CadError::new(
            "QUERY_PRECISION_UNSUPPORTED",
            "requested tolerance below supported numerical floor; source accuracy remains unknown",
        ));
    }
    Ok(())
}

pub fn execute(root: &Path, request: Request) -> Result<Value> {
    let start = Instant::now();
    if request.schema != "colocad.query/1" {
        return Err(CadError::new(
            "QUERY_UNSUPPORTED",
            "query schema is not supported",
        ));
    }
    if request.queries.is_empty() || request.queries.len() > 64 {
        return Err(CadError::new(
            "QUERY_LIMIT",
            "queries must contain 1..64 entries",
        ));
    }
    let tol = request.tolerance;
    validate_tolerance(tol)?;
    let manifest = Manifest::open(root, &request.query_revision_id)?;
    let (definition, occurrence, pose) = match &request.target {
        Target::Definition { definition_id } => {
            if request.space == Space::Assembly {
                return Err(CadError::new(
                    "QUERY_ARGUMENT",
                    "assembly space requires an occurrence",
                ));
            }
            (definition_id.clone(), None, placement::Rigid::default())
        }
        Target::Instance { instance_path } => {
            let o = manifest
                .body
                .occurrences
                .values()
                .find(|o| &o.instance_path == instance_path)
                .ok_or_else(|| {
                    CadError::new("TOPOLOGY_REF_INVALID", "instance path not in snapshot")
                })?;
            (
                o.definition_id.clone(),
                Some(instance_path.clone()),
                if request.space == Space::Assembly {
                    o.placement()?
                } else {
                    placement::Rigid::default()
                },
            )
        }
    };
    let mut kernel = Kernel::default();
    kernel.fixes.precise_properties_body_length_scale = true;
    kernel.fixes.precise_properties_split_at_surface_knots = true;
    kernel.fixes.precise_properties_span_end_from_below = true;
    kernel.fixes.precise_properties_trim_base_v = true;
    kernel.fixes.integrator_propagates_failure = true;
    let loaded = manifest.load(root, &definition, &mut kernel)?;
    let load_ms = start.elapsed().as_secs_f64() * 1000.;
    let indexing = Instant::now();
    let solid = brep_is_manifold_solid(&loaded.body)?;
    let reverse = loaded.entities.iter().map(|(t, e)| (*e, *t)).collect();
    let mut c = Context {
        kernel,
        loaded,
        manifest,
        definition,
        occurrence,
        pose,
        tolerance: tol,
        cache: BTreeMap::new(),
        visited: 0,
        reverse,
        solid,
        probes: 0,
        hypotheses: 0,
        holes: None,
        incidents: None,
    };
    let index_ms = indexing.elapsed().as_secs_f64() * 1000.;
    let mut ids = BTreeSet::new();
    let mut results = Vec::new();
    let mut extra_definitions = BTreeMap::new();
    let mut measurement_calls = 0;
    let mut result_bytes = 0usize;
    for q in &request.queries {
        if !valid_id(&q.id) || !ids.insert(&q.id) || q.limit == 0 || q.limit > 256 {
            return Err(CadError::new(
                "QUERY_ARGUMENT",
                "query IDs must be unique and limit must be 1..256",
            ));
        }
        let operation = if let Operation::Measure { a, b } = &q.operation {
            if q.cursor.is_some() {
                return Err(CadError::new("QUERY_ARGUMENT", "measure does not paginate"));
            }
            measurement_calls += 1;
            super::measure::run(&mut c, root, &mut extra_definitions, request.space, a, b).map(|r| {
                let complete = r["relationship"] != "indeterminate";
                json!({"id":q.id,"op":"measure","status":if complete{"complete"}else{"partial"},
                    "items":[r],"returned_count":1,"total_count":1,"next_cursor":null,
                    "coverage":{"relationship_complete":complete,"scope":"explicit_whole_part_pair"},
                    "diagnostics":if complete{json!([])}else{json!([{"code":"QUERY_UNCERTAIN","message":"relationship is numerically indeterminate"}])},
                    "measurement":r})
            })
        } else {
            super::selection::run(&mut c, &request, q)
        };
        match operation {
            Ok(result) => {
                result_bytes += serde_json::to_vec(&result)
                    .map_err(|e| CadError::new("QUERY_ARGUMENT", e.to_string()))?
                    .len();
                if result_bytes > 8 * 1024 * 1024 {
                    return Err(CadError::new(
                        "QUERY_LIMIT",
                        "query receipt exceeds 8 MiB; request smaller pages",
                    ));
                }
                results.push(result)
            }
            Err(mut error) => {
                error.query_id = Some(q.id.clone());
                error.details = Some(json!({"cause":error.details,"completed_results":results}));
                return Err(error);
            }
        }
    }
    let receipt = json!({"schema":request.schema,"query_revision_id":request.query_revision_id,"target":request.target,"space":request.space,"tolerance":request.tolerance,"source_accuracy":"unknown","results":results,"degradations":c.manifest.body.degradations,"cache":{"geometry":"exact_snapshot","descriptors":"request_local"},"execution":{"geometry_build_calls":0,"step_export_calls":0,"tessellation_calls":0,"definitions_loaded":1+extra_definitions.len(),"measurement_calls":measurement_calls,"measurement_boolean_intersection_calls":measurement_calls,"material_probes":c.probes,"pattern_hypotheses":c.hypotheses},"timing_ms":{"load":load_ms,"index":index_ms,"query":start.elapsed().as_secs_f64()*1000.-load_ms-index_ms,"total":start.elapsed().as_secs_f64()*1000.}});
    if serde_json::to_vec(&receipt)
        .map_err(|e| CadError::new("QUERY_ARGUMENT", e.to_string()))?
        .len()
        > 8 * 1024 * 1024
    {
        return Err(CadError::new(
            "QUERY_LIMIT",
            "query receipt exceeds 8 MiB; request smaller pages",
        ));
    }
    Ok(receipt)
}
