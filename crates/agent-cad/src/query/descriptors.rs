//! Trim/topology metadata, preserving the distinction between support surfaces and faces.
use super::engine::{arr, vec, Context};
use crate::*;
use solidcore::nurbs::{
    b_spline_surface::p1::SurfParamType,
    b_spline_surface_n_lib::p2::{EvaluateFlags, OrientType},
};
use solidcore::surface::surface::SurfaceKind;
use solidcore::topology::{
    edgeuse::{p1 as eu, p3 as trims},
    face::p1 as face_ops,
};
use solidcore::{EntityRef, Point2};

impl Context {
    pub(crate) fn edge_incidents(
        &mut self,
        edge: solidcore::EdgeId,
    ) -> Result<Vec<(solidcore::FaceId, solidcore::LoopId, bool)>> {
        if self.incidents.is_none() {
            let mut incidents: BTreeMap<_, Vec<_>> = BTreeMap::new();
            for id in self.loaded.body.loops().ids() {
                let l = self.loaded.body.loop_(id)?;
                for e in &l.edges {
                    self.visited += 1;
                    if self.visited > 200_000 {
                        return Err(CadError::new("QUERY_LIMIT", "adjacency budget exceeded"));
                    }
                    incidents
                        .entry(e.edge)
                        .or_default()
                        .push((l.face, id, e.forward));
                }
            }
            self.incidents = Some(incidents);
        }
        let result = self
            .incidents
            .as_ref()
            .unwrap()
            .get(&edge)
            .cloned()
            .unwrap_or_default();
        self.visited += result.len();
        if self.visited > 200_000 {
            return Err(CadError::new("QUERY_LIMIT", "adjacency budget exceeded"));
        }
        Ok(result)
    }
    pub(crate) fn loop_uses(&mut self, id: solidcore::LoopId) -> Result<Vec<Value>> {
        let l = self.loaded.body.loop_(id)?.clone();
        let Some(lu) = l.loopuse else {
            return Err(CadError::new(
                "QUERY_UNSUPPORTED",
                "loop has no oriented native uses",
            ));
        };
        let mut out = vec![];
        for u in eu::loopuse_edgeuses(&self.loaded.body, lu)? {
            trims::get_or_create_uv_trim_curve(
                &self.kernel,
                &mut self.loaded.body,
                u,
                false,
                false,
                true,
            )?
            .status?;
            let e = self.loaded.body.edgeuse(u)?;
            let id = e
                .edge
                .ok_or_else(|| CadError::new("ARTIFACT_INTEGRITY", "edge use has no edge"))?;
            let edge = self.loaded.body.edge(id)?;
            let Some(curve) = eu::uv_trim_curve(&self.loaded.body, u)?.0 else {
                return Err(CadError::new(
                    "QUERY_UNSUPPORTED",
                    "edge use has no face-local trim",
                ));
            };
            let interval = curve.natural_interval();
            let bounds = curve.calculate_tight_bounding_box(&self.kernel, &interval)?;
            out.push(json!({"edge":self.reference(self.token(EntityRef::Edge(id))?),"orientation":match e.orientation{OrientType::Same=>"same",OrientType::Opposite=>"opposite",_=>"unknown"},"edge_parameter_range":[edge.interval.min(),edge.interval.max()],"trim_parameter_range":[interval.min(),interval.max()],"uv_bounds":[[bounds.min().x,bounds.min().y],[bounds.max().x,bounds.max().y]],"trim_domain_semantics":"native_face_local_pcurve"}));
        }
        Ok(out)
    }
    pub(crate) fn enrich(&mut self, e: EntityRef, v: &mut Value) -> Result<()> {
        match e {
            EntityRef::Face(id) => {
                let f = self.loaded.body.face(id)?.clone();
                let surface = f
                    .surface
                    .as_ref()
                    .ok_or_else(|| CadError::new("QUERY_UNSUPPORTED", "face without surface"))?;
                let mut neighbors = BTreeSet::new();
                let mut counts = BTreeMap::new();
                for l in &f.loops {
                    for edge in &self.loaded.body.loop_(*l)?.edges {
                        *counts.entry(edge.edge).or_insert(0) += 1;
                    }
                }
                for edge in counts.keys() {
                    for (other, _, _) in self.edge_incidents(*edge)? {
                        if other != id {
                            neighbors.insert(self.token(EntityRef::Face(other))?);
                        }
                    }
                }
                v["adjacent_faces"] = json!(neighbors
                    .into_iter()
                    .map(|t| self.reference(t))
                    .collect::<Vec<_>>());
                v["seam_edges"] = json!(counts
                    .iter()
                    .filter(|(_, n)| **n > 1)
                    .map(|(e, _)| self.token(EntityRef::Edge(*e)).map(|t| self.reference(t)))
                    .collect::<Result<Vec<_>>>()?);
                v["kernel_tolerance_mm"] = json!(f.tolerance);
                let natural = surface.natural_uv_domain_with_nurb(&self.kernel)?;
                let periodic = if matches!(surface.kind(), SurfaceKind::Plane(_)) {
                    [false, false]
                } else {
                    [
                        surface.is_periodic(&self.kernel, &natural, SurfParamType::U)?,
                        surface.is_periodic(&self.kernel, &natural, SurfParamType::V)?,
                    ]
                };
                let periods = [
                    periodic[0].then(|| natural.max().x - natural.min().x),
                    periodic[1].then(|| natural.max().y - natural.min().y),
                ];
                let singular = if matches!(
                    surface.kind(),
                    SurfaceKind::Plane(_) | SurfaceKind::Torus(_)
                ) {
                    json!({"status":"analytic","side_mask":0,"approximate_side_mask":0})
                } else {
                    match surface.get_singularities_of_face(&self.kernel, f.tolerance, false, false)
                    {
                        Ok(s) => {
                            json!({"status":"native_boundary_test","side_mask":s.singular,"approximate_side_mask":s.approx_singular})
                        }
                        Err(e) if e.is_unported() => {
                            json!({"status":"unsupported","reason":e.to_string()})
                        }
                        Err(e) => return Err(e.into()),
                    }
                };
                v["parameterization"] = json!({"periodic":periodic,"periods":periods,"natural_uv_domain":[[natural.min().x,natural.min().y],[natural.max().x,natural.max().y]],"singularities":singular,"singularity_scope":"natural_surface_boundary; evaluate_face also checks the requested UV"});
                if let SurfaceKind::Cone(c) = surface.kind() {
                    if c.is_cylinder() {
                        let axis =
                            vec(serde_json::from_value(v["axis"]["direction"].clone()).unwrap());
                        let origin =
                            vec(serde_json::from_value(v["axis"]["point_mm"].clone()).unwrap());
                        let eval = |u, fv| {
                            surface.evaluate(
                                &self.kernel,
                                Point2::new(u, fv),
                                1,
                                1,
                                EvaluateFlags {
                                    non_zero_tangents: false,
                                    ..Default::default()
                                },
                            )
                        };
                        let mid = f.uv_domain.evaluate(0.5, 0.5);
                        let p0 = eval(mid.x, f.uv_domain.min().y)?;
                        let p1 = eval(mid.x, f.uv_domain.max().y)?;
                        let mut limits = [
                            (self.pose.point(p0[0][0]) - origin).dot(axis),
                            (self.pose.point(p1[0][0]) - origin).dot(axis),
                        ];
                        limits.sort_by(f64::total_cmp);
                        v["axial_interval_mm"] = json!(limits);
                        let key = |e: EntityRef| solidcore::topology::loopuse::notify_ref(e);
                        let rect = face_ops::find_if_rectangular_trim(
                            &self.kernel,
                            &mut self.loaded.body,
                            id,
                            &key,
                        )?;
                        v["trim_coverage_status"] = json!(if rect.rectangular {
                            "rectangular"
                        } else {
                            "nonrectangular"
                        });
                        if let Some(period) = periods[0] {
                            v["angular_bounds_rad"] = json!([
                                (f.uv_domain.min().x - natural.min().x) / period
                                    * std::f64::consts::TAU,
                                (f.uv_domain.max().x - natural.min().x) / period
                                    * std::f64::consts::TAU
                            ]);
                            if rect.rectangular {
                                v["angular_coverage_rad"] = json!(
                                    (rect.uv_domain.max().x - rect.uv_domain.min().x) / period
                                        * std::f64::consts::TAU
                                );
                            }
                        }
                        let d = eval(mid.x, mid.y)?;
                        let radial = self.pose.point(d[0][0]) - origin;
                        let radial = radial - axis * radial.dot(axis);
                        let cross = self.pose.vector(d[1][0].cross(d[0][1]));
                        v["material_side"] = json!(match self.oriented_sign(id)? {
                            Some(sign) if radial.length() > 1e-12 && cross.length() > 1e-12 =>
                                if cross.dot(radial) * sign < 0. {
                                    "internal"
                                } else {
                                    "external"
                                },
                            _ => "unknown",
                        });
                    }
                }
            }
            EntityRef::Edge(id) => {
                let incidents = self.edge_incidents(id)?;
                let mut faces = BTreeMap::new();
                for (f, _, _) in &incidents {
                    *faces.entry(*f).or_insert(0) += 1;
                }
                v["incident_face_uses"]=json!(incidents.iter().map(|(f,l,forward)|Ok(json!({"face":self.reference(self.token(EntityRef::Face(*f))?),"loop":self.reference(self.token(EntityRef::Loop(*l))?),"orientation":if *forward{"same"}else{"opposite"}}))).collect::<Result<Vec<_>>>()?);
                v["seam"] = json!(faces.values().any(|n| *n > 1));
                let edge = self.loaded.body.edge(id)?;
                let curve = edge.curve.as_ref().unwrap();
                v["kernel_tolerance_mm"] = json!(edge.tolerance);
                v["periodic"] = json!(curve.is_periodic(&self.kernel, &curve.natural_interval())?);
                if v["curve_type"] == "circle" {
                    v["arc_coverage_rad"] =
                        json!(v["length_mm"].as_f64().unwrap() / v["radius_mm"].as_f64().unwrap());
                    v["arc_start_mm"] = json!(arr(self
                        .pose
                        .point(curve.evaluate_point(&self.kernel, edge.interval.min())?)));
                    v["arc_end_mm"] = json!(arr(self
                        .pose
                        .point(curve.evaluate_point(&self.kernel, edge.interval.max())?)));
                }
            }
            EntityRef::Vertex(id) => {
                v["kernel_tolerance_mm"] = json!(self.loaded.body.vertex(id)?.tolerance);
                v["incident_edges"] = json!(self
                    .loaded
                    .body
                    .edge_list()
                    .iter()
                    .filter(|e| self.loaded.body.edge(**e).unwrap().vertices.contains(&id))
                    .map(|e| self.token(EntityRef::Edge(*e)).map(|t| self.reference(t)))
                    .collect::<Result<Vec<_>>>()?);
            }
            EntityRef::Loop(id) => {
                v["edge_uses"] = json!(self.loop_uses(id)?);
            }
            _ => {}
        }
        Ok(())
    }
}
