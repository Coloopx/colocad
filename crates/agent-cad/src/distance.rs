//! Exact topology distance adapter; no sampled-mesh distance fallback.
use solidcore::query::brep::{BrepFaceServices, SideObject, SidedFaceServices};
use solidcore::query::brep::{BrepIdentity, ProvenBrepIdentity};
use solidcore::surface::surface::p1::SurfaceSolveContext;
use solidcore::surface::surface_cache::{face_context::FaceQueries, SurfaceCache};
use solidcore::topology::brep_cache::{BrepCache, BrepObjects, CacheObject, CacheSource};
use solidcore::{
    error::Result,
    kernel::Kernel,
    topology::{Brep, BrepId, FaceId},
};
use std::cell::{RefCell, RefMut};
struct Context<'a, 'b>(RefMut<'a, SidedFaceServices<'b>>);
impl SurfaceSolveContext<SideObject> for Context<'_, '_> {
    fn build_surface_cache(
        &mut self,
        k: &Kernel,
        surface: &Surface,
        handle: Option<SideObject>,
    ) -> Result<SurfaceCache<SideObject>> {
        self.0.build_surface_cache(k, surface, handle)
    }
    fn surface_cache_state(
        &mut self,
        k: &Kernel,
        surface: &Surface,
        handle: Option<SideObject>,
    ) -> Result<SurfaceCache<SideObject>> {
        self.0.surface_cache_state(k, surface, handle)
    }
    fn face_queries(&self) -> Option<&dyn FaceQueries<SideObject>> {
        Some(&*self.0)
    }
}
use solidcore::geom::{extent1d::Extent1d, extent2d::Extent2d, object::ShapeMember};
use solidcore::intersect::topology_solver::p1::TopologyObjects;
use solidcore::{curve::curve::Curve, math::Point3, surface::surface::Surface};
struct Pair<'a> {
    services: RefCell<SidedFaceServices<'a>>,
    b: [&'a Brep; 2],
    ids: [ProvenBrepIdentity<'static>; 2],
    cache: [BrepCache<SideObject>; 2],
}
impl Pair<'_> {
    fn objects(&self, i: usize) -> BrepObjects<'_> {
        let identity = self.ids[i].identity();
        BrepObjects {
            preparation: None,
            brep: self.b[i],
            brep_id: identity.brep_id,
            cache: None,
            keys: identity.keys,
            owner_of: identity.owner_of,
        }
    }
}
impl TopologyObjects<SideObject> for Pair<'_> {
    fn surface_solve_context(&self) -> Option<Box<dyn SurfaceSolveContext<SideObject> + '_>> {
        Some(Box::new(Context(self.services.borrow_mut())))
    }
    fn brep_cache(&self, o: SideObject) -> Result<&BrepCache<SideObject>> {
        Ok(&self.cache[o.0])
    }
    fn curve(&self, o: SideObject) -> Result<&Curve> {
        if let CacheObject::Curve(e) = o.1 {
            self.b[o.0]
                .edge(e)?
                .curve
                .as_ref()
                .ok_or(solidcore::error::SolidError::null_pointer("distance"))
        } else {
            Err(solidcore::error::SolidError::null_pointer("distance"))
        }
    }
    fn analytic_curve_handle(&self, o: SideObject) -> Option<SideObject> {
        Some(o)
    }
    fn vertex_point(&self, o: SideObject) -> Result<Point3> {
        self.objects(o.0).vertex_point(o.1)
    }
    fn vertex_tolerance(&self, o: SideObject) -> Result<f64> {
        self.objects(o.0).vertex_tolerance(o.1)
    }
    fn curve_edge(&self, o: SideObject) -> Result<Option<SideObject>> {
        Ok(self
            .objects(o.0)
            .curve_edge(o.1)?
            .map(|v| SideObject(o.0, v)))
    }
    fn edge_curve(&self, o: SideObject) -> Result<Option<SideObject>> {
        Ok(self
            .objects(o.0)
            .edge_curve(o.1)?
            .map(|v| SideObject(o.0, v)))
    }
    fn edge_interval(&self, o: SideObject) -> Result<Extent1d> {
        self.objects(o.0).edge_interval(o.1)
    }
    fn edge_tolerance(&self, o: SideObject) -> Result<f64> {
        self.objects(o.0).edge_tolerance(o.1)
    }
    fn surface_face(&self, o: SideObject) -> Result<Option<SideObject>> {
        Ok(self
            .objects(o.0)
            .surface_face(o.1)?
            .map(|v| SideObject(o.0, v)))
    }
    fn face_surface(&self, o: SideObject) -> Result<SideObject> {
        Ok(SideObject(o.0, self.objects(o.0).face_surface(o.1)?))
    }
    fn surface(&self, o: SideObject) -> Result<&Surface> {
        if let CacheObject::Surface(f) = o.1 {
            self.b[o.0]
                .face(f)?
                .surface
                .as_ref()
                .ok_or(solidcore::error::SolidError::null_pointer("distance"))
        } else {
            Err(solidcore::error::SolidError::null_pointer("distance"))
        }
    }
    fn face_uv_domain(&self, o: SideObject) -> Result<Extent2d> {
        self.objects(o.0).face_uv_domain(o.1)
    }
    fn face_tolerance(&self, o: SideObject) -> Result<f64> {
        self.objects(o.0).face_tolerance(o.1)
    }
    fn face_entity(&self, o: SideObject) -> Option<(&Brep, FaceId)> {
        if let CacheObject::Face(f) = o.1 {
            Some((self.b[o.0], f))
        } else {
            None
        }
    }
    fn surface_owner(&self, o: SideObject) -> Result<Option<SideObject>> {
        Ok(self
            .objects(o.0)
            .surface_owner(o.1)?
            .map(|v| SideObject(o.0, v)))
    }
    fn shape_member(&self, o: SideObject) -> ShapeMember {
        self.objects(o.0).shape_member(o.1)
    }
    fn object_zone_tol_3d(&self, k: &Kernel, o: SideObject) -> Result<f64> {
        self.objects(o.0).object_zone_tol_3d(k, o.1)
    }
}
pub(crate) struct Measurement {
    pub distance: f64,
    pub witness: serde_json::Value,
}
pub(crate) fn boundary_distance(k: &mut Kernel, a: &Brep, b: &Brep) -> Result<f64> {
    Ok(boundary_measurement(k, a, b)?.distance)
}
pub(crate) fn boundary_measurement(k: &mut Kernel, a: &Brep, b: &Brep) -> Result<Measurement> {
    let mut prepared_a = solidcore::topology::brep::p3::brep_copy(k, a)?;
    let mut prepared_b = solidcore::topology::brep::p3::brep_copy(k, b)?;
    for body in [&mut prepared_a, &mut prepared_b] {
        let uses = body.edgeuses().ids().collect::<Vec<_>>();
        for eu in uses {
            if solidcore::topology::edgeuse::p1::face(body, eu)?.is_some() {
                solidcore::topology::edgeuse::p3::get_or_create_uv_trim_curve(
                    k, body, eu, false, false, true,
                )?
                .status?;
            }
        }
    }
    let (a, b) = (&prepared_a, &prepared_b);
    let ids = [
        BrepIdentity::of_brep_with_id(BrepId::from_index(0), a),
        BrepIdentity::of_brep_with_id(BrepId::from_index(1), b),
    ];
    let mut cache = [
        BrepCache::new(solidcore::topology::brep::p1::TYPE)?,
        BrepCache::new(solidcore::topology::brep::p1::TYPE)?,
    ];
    for (i, body) in [a, b].into_iter().enumerate() {
        cache[i].build_trees(
            k,
            CacheSource::Brep(body),
            |v| SideObject(i, v),
            ids[i].identity().keys,
        )?;
    }
    let mut ca = solidcore::topology::brep::p3::brep_copy(k, a)?;
    let mut cb = solidcore::topology::brep::p3::brep_copy(k, b)?;
    let services = RefCell::new(SidedFaceServices(vec![
        BrepFaceServices::new(&mut ca),
        BrepFaceServices::new(&mut cb),
    ]));
    let pair = Pair {
        b: [a, b],
        ids,
        cache,
        services,
    };
    let solution = solidcore::query::api_queries::brep_relationship_solve(
        k,
        &pair,
        SideObject(0, CacheObject::Brep),
        SideObject(1, CacheObject::Brep),
    )?;
    let mut points = [None, None];
    let mut parameter = 0;
    for object in solution.objects.iter().take(solution.num_objects).flatten() {
        let (point, advance) = match object.1 {
            CacheObject::Vertex(id) => (Some(pair.b[object.0].vertex(id)?.point), 0),
            CacheObject::Edge(id) | CacheObject::Curve(id) => (
                Some(
                    pair.b[object.0]
                        .edge(id)?
                        .curve
                        .as_ref()
                        .ok_or(solidcore::SolidError::null_pointer("distance witness"))?
                        .evaluate_point(k, solution.start.param(parameter)?)?,
                ),
                1,
            ),
            CacheObject::Face(id) | CacheObject::Surface(id) => (
                Some(
                    pair.b[object.0]
                        .face(id)?
                        .surface
                        .as_ref()
                        .ok_or(solidcore::SolidError::null_pointer("distance witness"))?
                        .evaluate_point(
                            k,
                            solidcore::Point2::new(
                                solution.start.param(parameter)?,
                                solution.start.param(parameter + 1)?,
                            ),
                        )?,
                ),
                2,
            ),
            _ => (None, 0),
        };
        parameter += advance;
        if let Some(p) = point {
            if [p.x, p.y, p.z].into_iter().all(f64::is_finite) {
                points[object.0] = Some([p.x, p.y, p.z]);
            }
        }
    }
    Ok(Measurement {
        distance: solution.start.solution_value,
        witness: serde_json::json!({"a_point_mm":points[0],"b_point_mm":points[1],"status":if points.iter().all(Option::is_some){"exact_boundary_solver"}else{"witness_unavailable"},"support_refs":null,"support_ref_status":"not_retained_by_copying_solver_adapter"}),
    })
}
