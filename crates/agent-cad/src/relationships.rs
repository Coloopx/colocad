//! Exact solid relationships. Boundary distance and material overlap are independent predicates.
use crate::*;
use query::contract::Tolerance;
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "RelationshipTarget")]
pub struct Target {
    pub instance: String,
    #[serde(default)]
    pub region: Option<operations::FaceSelector>,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContactPair {
    pub a: operations::FaceSelector,
    pub b: operations::FaceSelector,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PoseSample {
    pub id: String,
    #[serde(default)]
    pub a: placement::Pose,
    #[serde(default)]
    pub b: placement::Pose,
}
pub(crate) fn validate(
    m: &Model,
    a: &Target,
    b: &Target,
    t: Tolerance,
    max: &Scalar,
    min: &Option<Scalar>,
    contacts: &[ContactPair],
    samples: &[PoseSample],
) -> Result<()> {
    query::engine::validate_tolerance(t)?;
    if a.instance == b.instance
        || [a, b]
            .iter()
            .any(|a| !m.assembly.iter().any(|i| i.id == a.instance))
    {
        return Err(CadError::new(
            "INVALID_REFERENCE",
            "relationship requires two different occurrences",
        ));
    }
    if max.resolve(&m.parameters)? < 0.
        || min
            .as_ref()
            .map(|s| s.resolve(&m.parameters))
            .transpose()?
            .is_some_and(|x| x < 0.)
    {
        return Err(CadError::new(
            "INVALID_DIMENSION",
            "negative relationship threshold",
        ));
    }
    if samples.len() > 32 || contacts.len() > 32 {
        return Err(CadError::new(
            "RESOURCE_LIMIT",
            "at most 32 poses/contact pairs",
        ));
    }
    let mut ids = BTreeSet::new();
    for s in samples {
        if !valid_id(&s.id) || !ids.insert(&s.id) {
            return Err(CadError::new(
                "INVALID_POSE",
                "pose IDs must be unique and valid",
            ));
        }
        s.a.resolve(&m.parameters)?;
        s.b.resolve(&m.parameters)?;
    }
    Ok(())
}
fn volume(k: &mut Kernel, b: &mut Brep) -> Result<f64> {
    if b.face_list().is_empty() {
        Ok(0.)
    } else {
        measured_volume(k, b)
    }
}
pub(crate) fn measure(k: &mut Kernel, a: &Brep, b: &Brep, t: Tolerance) -> Result<Value> {
    if !brep_is_manifold_solid(a)? || !brep_is_manifold_solid(b)? {
        return Err(CadError::new(
            "QUERY_UNSUPPORTED",
            "material relationships require closed manifold solids",
        ));
    }
    let ac = solidcore::topology::brep::p3::brep_copy(k, a)?;
    let bc = solidcore::topology::brep::p3::brep_copy(k, b)?;
    let mut common = boolean_intersection(k, ac, bc)?;
    let overlap = volume(k, &mut common)?;
    let mut ac = solidcore::topology::brep::p3::brep_copy(k, a)?;
    let mut bc = solidcore::topology::brep::p3::brep_copy(k, b)?;
    let av = volume(k, &mut ac)?;
    let bv = volume(k, &mut bc)?;
    let boundary = crate::distance::boundary_measurement(k, a, b)?;
    if !boundary.distance.is_finite()
        || boundary.distance < 0.
        || !overlap.is_finite()
        || overlap < 0.
    {
        return Err(CadError::new(
            "QUERY_UNCERTAIN",
            "invalid kernel relationship measurement",
        ));
    }
    // Retain numerical uncertainty separately from the caller's permitted overlap.
    let eps = 1e-9_f64.max(av.max(bv) * 1e-10);
    let relation = if overlap > eps {
        if (overlap - av.min(bv)).abs() <= eps {
            "contained"
        } else {
            "overlapping"
        }
    } else if overlap > 0. {
        "indeterminate"
    } else if boundary.distance > t.linear_mm {
        "separated"
    } else {
        "touching"
    };
    Ok(
        json!({"relationship":relation,"boundary_distance_mm":boundary.distance,"overlap_mm3":overlap,"a_volume_mm3":av,"b_volume_mm3":bv,"contained_side":if relation=="contained"{Some(if av<=bv{"a"}else{"b"})}else{None},"witness":boundary.witness,"predicate_tolerance":t,"volume_uncertainty_mm3":eps,"distance_semantics":"unsigned_boundary_distance_not_penetration_depth"}),
    )
}
pub(crate) fn require_exact(m: &Model, definition: &str, trace: &[Value]) -> Result<()> {
    let mut ancestry = BTreeSet::new();
    let mut todo = vec![definition];
    while let Some(id) = todo.pop() {
        if ancestry.insert(id) {
            if let Some(f) = m.features.iter().find(|f| f.id() == id) {
                todo.extend(f.inputs());
            }
        }
    }
    let degraded = trace
        .iter()
        .filter(|r| {
            !r["degradation"].is_null() && r["id"].as_str().is_some_and(|id| ancestry.contains(id))
        })
        .cloned()
        .collect::<Vec<_>>();
    if !degraded.is_empty() {
        let mut e = CadError::new("QUERY_UNCERTAIN", "required geometry has degraded features");
        e.details = Some(json!({"definition_id":definition,"degradations":degraded}));
        return Err(e);
    }
    Ok(())
}
// A deliberately bounded proof: a polyhedral solid's entire contact with a
// separating plane consists of its coplanar faces, edges and vertices. Verify
// all of those belong to the selected face; a single closest witness is insufficient.
fn support(
    k: &Kernel,
    b: &Brep,
    selector: &operations::FaceSelector,
    p: &BTreeMap<String, f64>,
    t: Tolerance,
) -> Result<(Vec3, Vec3, f64)> {
    use solidcore::surface::surface::SurfaceKind;
    let selected = match selector {
        operations::FaceSelector::OnPlane {
            axis,
            value,
            tolerance,
            expected_count,
        } => {
            let tolerance = tolerance.resolve(p)?;
            if !tolerance.is_finite() || tolerance <= 0. {
                return Err(CadError::new(
                    "QUERY_ARGUMENT",
                    "contact selector tolerance must be positive",
                ));
            }
            let value = value.resolve(p)?;
            let i = axis.index();
            let mut matches = vec![];
            for id in b.face_list() {
                let bounds = solidcore::query::api_queries::face_bounding_box(k, b, *id, true)?;
                if (bounds.min_at(i) - value).abs() <= tolerance
                    && (bounds.max_at(i) - value).abs() <= tolerance
                {
                    matches.push(*id);
                }
            }
            if matches.len() != *expected_count {
                return Err(CadError::new(
                    "SELECTION_CARDINALITY",
                    "contact face count differs from expectation",
                ));
            }
            matches
        }
        _ => operations::select_faces(k, b, std::slice::from_ref(selector), p)?,
    };
    if selected.len() != 1 {
        return Err(CadError::new(
            "SELECTION_CARDINALITY",
            "contact region requires one face",
        ));
    }
    let id = selected[0];
    let face = b.face(id)?;
    let Some(surface) = &face.surface else {
        return Err(CadError::new(
            "QUERY_UNSUPPORTED",
            "contact face has no surface",
        ));
    };
    let SurfaceKind::Plane(plane) = surface.kind() else {
        return Err(CadError::new(
            "QUERY_UNSUPPORTED",
            "contact certification requires a planar region",
        ));
    };
    let o = plane.position().origin();
    let n = plane.evaluate_normal();
    let mut edges = BTreeSet::new();
    let mut vertices = BTreeSet::new();
    for l in &face.loops {
        for e in &b.loop_(*l)?.edges {
            edges.insert(e.edge);
            let (a, z) = solidcore::topology::edge::start_end_vertices(b, e.edge)?;
            vertices.extend(a);
            vertices.extend(z);
        }
    }
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for v in b.vertices().ids() {
        let h = (b.vertex(v)?.point - o).dot(n);
        lo = lo.min(h);
        hi = hi.max(h);
        if h.abs() <= t.linear_mm && !vertices.contains(&v) {
            return Err(CadError::new(
                "QUERY_UNSUPPORTED",
                "contact outside selected face vertices",
            ));
        }
    }
    let side = if hi <= t.linear_mm && lo < -t.linear_mm {
        -1.
    } else if lo >= -t.linear_mm && hi > t.linear_mm {
        1.
    } else {
        return Err(CadError::new(
            "QUERY_UNSUPPORTED",
            "region plane does not separate the complete solid",
        ));
    };
    for f in b.face_list() {
        let sf = b
            .face(*f)?
            .surface
            .as_ref()
            .ok_or_else(|| CadError::new("QUERY_UNSUPPORTED", "missing surface"))?;
        let SurfaceKind::Plane(pl) = sf.kind() else {
            return Err(CadError::new(
                "QUERY_UNSUPPORTED",
                "contact completeness proof currently requires polyhedral solids",
            ));
        };
        if *f != id
            && pl.evaluate_normal().cross(n).length() <= t.angular_rad
            && (pl.position().origin() - o).dot(n).abs() <= t.linear_mm
        {
            return Err(CadError::new(
                "QUERY_UNSUPPORTED",
                "another coplanar face is outside the contact region",
            ));
        }
    }
    for e in b.edges().ids() {
        let edge = b.edge(e)?;
        if !matches!(
            edge.curve.as_ref().map(|c| c.kind()),
            Some(solidcore::curve::curve::CurveKind::Line(_))
        ) {
            return Err(CadError::new(
                "QUERY_UNSUPPORTED",
                "contact proof requires straight polyhedral edges",
            ));
        }
        let (a, z) = solidcore::topology::edge::start_end_vertices(b, e)?;
        let (a, z) = (
            a.ok_or_else(|| CadError::new("QUERY_UNSUPPORTED", "missing edge endpoint"))?,
            z.ok_or_else(|| CadError::new("QUERY_UNSUPPORTED", "missing edge endpoint"))?,
        );
        if (b.vertex(a)?.point - o).dot(n).abs() <= t.linear_mm
            && (b.vertex(z)?.point - o).dot(n).abs() <= t.linear_mm
            && !edges.contains(&e)
        {
            return Err(CadError::new(
                "QUERY_UNSUPPORTED",
                "contact edge is outside selected face",
            ));
        }
    }
    Ok((o, n, side))
}
fn certify_contacts(
    k: &Kernel,
    a: &Brep,
    b: &Brep,
    contacts: &[ContactPair],
    p: &BTreeMap<String, f64>,
    t: Tolerance,
) -> Result<Value> {
    let mut failures = vec![];
    for (index, pair) in contacts.iter().enumerate() {
        let proof = (|| {
            let (ao, an, sa) = support(k, a, &pair.a, p, t)?;
            let (bo, bn, sb) = support(k, b, &pair.b, p, t)?;
            if an.cross(bn).length().atan2(an.dot(bn).abs()) > t.angular_rad
                || (bo - ao).dot(an).abs() > t.linear_mm
                || sa * sb * an.dot(bn) >= 0.
            {
                return Err(CadError::new(
                    "QUERY_UNSUPPORTED",
                    "allowed faces do not define opposing contact halfspaces",
                ));
            }
            Ok(())
        })();
        match proof {
            Ok(()) => {
                return Ok(
                    json!({"status":"certified_separating_plane","allowed_pair_index":index,"scope":"complete_polyhedral_contact_locus_in_selected_regions","tolerance":t}),
                )
            }
            Err(e) => failures.push(json!({"pair_index":index,"error":e})),
        }
    }
    let mut e = CadError::new(
        "QUERY_UNSUPPORTED",
        "cannot certify the complete contact locus in allowed regions",
    );
    e.details = Some(json!({"attempts":failures}));
    Err(e)
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn check(
    m: &Model,
    k: &mut Kernel,
    parts: &BTreeMap<String, Brep>,
    trace: &[Value],
    a: &Target,
    b: &Target,
    t: Tolerance,
    max: &Scalar,
    min: &Option<Scalar>,
    contacts: &[ContactPair],
    samples: &[PoseSample],
) -> Result<Value> {
    for target in [a, b] {
        let i = m.assembly.iter().find(|i| i.id == target.instance).unwrap();
        if !matches!(i.representation, assembly::Representation::Exact) {
            return Err(CadError::new(
                "QUERY_UNCERTAIN",
                "required relationship cannot be certified from reduced geometry",
            ));
        }
        require_exact(m, &i.input, trace)?;
    }
    if a.region.is_some() || b.region.is_some() {
        return Err(CadError::new(
            "QUERY_UNSUPPORTED",
            "region-restricted contact certification not available for this region",
        ));
    }
    let max = max.resolve(&m.parameters)?;
    let min = min.as_ref().map(|s| s.resolve(&m.parameters)).transpose()?;
    let fallback = [PoseSample {
        id: "solved".into(),
        a: Default::default(),
        b: Default::default(),
    }];
    let samples = if samples.is_empty() {
        &fallback[..]
    } else {
        samples
    };
    let mut rows = vec![];
    let mut passed = true;
    for s in samples {
        let mut ac = solidcore::topology::brep::p3::brep_copy(k, &parts[&a.instance])?;
        let mut bc = solidcore::topology::brep::p3::brep_copy(k, &parts[&b.instance])?;
        let ap = s.a.resolve(&m.parameters)?;
        let bp = s.b.resolve(&m.parameters)?;
        ap.apply(k, &mut ac)?;
        bp.apply(k, &mut bc)?;
        let mut r = measure(k, &ac, &bc, t)?;
        let relation = r["relationship"].as_str().unwrap().to_string();
        let contact = if relation == "touching" && !contacts.is_empty() {
            Some(certify_contacts(k, &ac, &bc, contacts, &m.parameters, t)?)
        } else {
            None
        };
        let ok = relation != "indeterminate"
            && r["overlap_mm3"].as_f64().unwrap() <= max
            && min.is_none_or(|min| {
                (relation == "separated" && r["boundary_distance_mm"].as_f64().unwrap() >= min)
                    || contact.is_some()
            });
        r["contact_evidence"]=contact.unwrap_or_else(||json!({"status":"not_applied","reason":if relation=="touching"{"no_region_exception_requested"}else{"not_a_touching_relationship"}}));
        r["pose_id"] = json!(s.id);
        r["applied_assembly_space_delta"] = json!({"a":ap.json(),"b":bp.json()});
        r["passed"] = json!(ok);
        passed &= ok;
        rows.push(r);
    }
    let r = json!({"kind":"geometric_relationship","a":a,"b":b,"samples":rows,"passed":passed,"max_overlap_mm3":max,"min_clearance_mm":min,"allowed_contacts":contacts,"pose_scope":"explicit_samples_only_no_continuous_motion_claim"});
    if !passed {
        let mut e = CadError::new(
            "GEOMETRIC_RELATIONSHIP_FAILED",
            "required geometric relationship failed",
        );
        e.details = Some(r);
        return Err(e);
    }
    Ok(r)
}
