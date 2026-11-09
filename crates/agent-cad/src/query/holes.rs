//! Connected trimmed cylinders, planar ends and material checks define the supported hole domain.
use super::{
    contract::*,
    engine::{arr, unit, vec, Context},
    selection,
};
use crate::*;
use solidcore::{EntityRef, Vec3};
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "HoleFilter")]
pub struct Filter {
    pub diameter_mm: Option<Range>,
    pub termination: Option<Termination>,
    pub axis_parallel_to: Option<DirectionInput>,
    #[serde(default)]
    pub opening_on: Vec<TopologyRef>,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Termination {
    Through,
    Blind,
    Unknown,
}
#[derive(Clone, Debug, Default)]
pub(crate) struct Recognition {
    pub holes: Vec<Value>,
    pub uncertain: Vec<Value>,
    pub excluded: Vec<Value>,
}
#[derive(Clone)]
struct Endpoint {
    plane: u64,
    loop_: u64,
    role: String,
    point: Vec3,
    normal: Vec3,
    edges: Vec<u64>,
    radius: f64,
}
#[derive(Clone)]
struct Segment {
    faces: Vec<u64>,
    origin: Vec3,
    axis: Vec3,
    radius: f64,
    interval: [f64; 2],
    ends: Vec<Endpoint>,
}
fn xyz(v: &Value) -> Vec3 {
    vec(serde_json::from_value(v.clone()).unwrap())
}
fn token(v: &Value) -> u64 {
    v["topology_id"].as_str().unwrap()[1..].parse().unwrap()
}
fn parallel(a: Vec3, b: Vec3, t: f64) -> bool {
    a.cross(b).length().atan2(a.dot(b).abs()) <= t
}
fn coaxis(a: &Value, b: &Value, t: Tolerance) -> bool {
    let aa = xyz(&a["axis"]["direction"]);
    let bb = xyz(&b["axis"]["direction"]);
    parallel(aa, bb, t.angular_rad)
        && (xyz(&a["axis"]["point_mm"]) - xyz(&b["axis"]["point_mm"]))
            .cross(aa)
            .length()
            <= t.linear_mm
}
fn material_witness(c: &mut Context, t: u64) -> Result<bool> {
    if c.probes + 2 > 20_000 {
        return Err(CadError::new(
            "QUERY_LIMIT",
            "material probe budget exceeded",
        ));
    }
    c.probes += 2;
    let EntityRef::Face(id) = c.loaded.entities[&t] else {
        unreachable!()
    };
    let f = c.loaded.body.face(id)?;
    let uv = f.uv_domain.evaluate(0.5, 0.5);
    let s = f.surface.as_ref().unwrap();
    let values = s.evaluate(
        &c.kernel,
        uv,
        1,
        1,
        solidcore::nurbs::b_spline_surface_n_lib::p2::EvaluateFlags {
            non_zero_tangents: false,
            ..Default::default()
        },
    )?;
    let Some(sign) = c.oriented_sign(id)? else {
        return Ok(false);
    };
    let normal = unit(arr(values[1][0].cross(values[0][1]) * sign))?;
    let distance = 10. * c.tolerance.linear_mm.max(f.tolerance);
    let mut classifier = solidcore::query::brep::BrepClassifier::of_brep(&c.loaded.body);
    let mut classified = vec![];
    for p in [
        values[0][0] - normal * distance,
        values[0][0] + normal * distance,
    ] {
        let pc = classifier.point_3d_classify(&c.kernel, p, c.tolerance.linear_mm, false)?;
        classified.push(if let Some(EntityRef::Region(r)) = pc.object() {
            Some(classifier.brep().region(r)?.is_void() == 0)
        } else {
            None
        });
    }
    Ok(classified == [Some(true), Some(false)])
}
fn build_segment(
    c: &mut Context,
    parts: &[u64],
    faces: &BTreeMap<u64, Value>,
) -> Result<std::result::Result<Segment, String>> {
    let first = &faces[&parts[0]];
    let axis = xyz(&first["axis"]["direction"]);
    let origin = xyz(&first["axis"]["point_mm"]);
    let radius = first["radius_mm"].as_f64().unwrap();
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    let mut area = 0.;
    let mut edges = BTreeSet::new();
    for t in parts {
        let f = &faces[t];
        if f["trim_coverage_status"] != "rectangular" {
            return Ok(Err("nonrectangular_cylindrical_patch".into()));
        }
        let o = xyz(&f["axis"]["point_mm"]);
        let a = xyz(&f["axis"]["direction"]);
        for v in f["axial_interval_mm"].as_array().unwrap() {
            let h = (o + a * v.as_f64().unwrap() - origin).dot(axis);
            lo = lo.min(h);
            hi = hi.max(h);
        }
        area += f["area_mm2"].as_f64().unwrap();
        edges.extend(c.children(*t, Kind::Edge)?);
    }
    let expected = std::f64::consts::TAU * radius * (hi - lo);
    let area_tol = c.tolerance.linear_mm * std::f64::consts::TAU * (radius + hi - lo);
    if hi - lo <= c.tolerance.linear_mm || (area - expected).abs() > area_tol {
        return Ok(Err("incomplete_or_overlapping_angular_coverage".into()));
    }
    let mut end_edges: [Vec<(u64, u64, u64)>; 2] = Default::default();
    for e in edges {
        let EntityRef::Edge(id) = c.loaded.entities[&e] else {
            unreachable!()
        };
        let incidents = c.edge_incidents(id)?;
        let outsiders = incidents
            .iter()
            .filter_map(|(f, l, _)| {
                let t = c.token(EntityRef::Face(*f)).ok()?;
                (!parts.contains(&t)).then_some((t, c.token(EntityRef::Loop(*l)).ok()?))
            })
            .collect::<BTreeSet<_>>();
        if outsiders.is_empty() {
            if incidents.len() < 2 {
                return Ok(Err("open_cylindrical_boundary".into()));
            }
            continue;
        }
        let d = c.describe(e)?;
        if d["curve_type"] != "circle"
            || (d["radius_mm"].as_f64().unwrap() - radius).abs() > c.tolerance.linear_mm
        {
            return Ok(Err("noncircular_opening_or_intersection".into()));
        }
        let h = (xyz(&d["center_mm"]) - origin).dot(axis);
        let end = if (h - lo).abs() <= c.tolerance.linear_mm {
            0
        } else if (h - hi).abs() <= c.tolerance.linear_mm {
            1
        } else {
            return Ok(Err("interior_cylindrical_boundary".into()));
        };
        if outsiders.len() != 1 {
            return Ok(Err("ambiguous_opening_adjacency".into()));
        }
        let (f, l) = *outsiders.first().unwrap();
        end_edges[end].push((e, f, l));
    }
    let mut ends = vec![];
    for (i, rim) in end_edges.iter().enumerate() {
        if rim.is_empty() {
            return Ok(Err("missing_end_boundary".into()));
        }
        let f = rim[0].1;
        let l = rim[0].2;
        let face = &faces[&f];
        if rim.iter().any(|(_, ff, ll)| *ff != f || *ll != l)
            || face["surface_type"] != "plane"
            || face["normal"].is_null()
        {
            return Ok(Err("nonplanar_or_composite_entrance".into()));
        }
        let normal = xyz(&face["normal"]);
        if !parallel(axis, normal, c.tolerance.angular_rad) {
            return Ok(Err("oblique_entrance".into()));
        }
        let EntityRef::Loop(loop_id) = c.loaded.entities[&l] else {
            unreachable!()
        };
        let loop_edges = c
            .loaded
            .body
            .loop_(loop_id)?
            .edges
            .iter()
            .map(|e| c.token(EntityRef::Edge(e.edge)))
            .collect::<Result<BTreeSet<_>>>()?;
        let boundary = rim.iter().map(|(e, _, _)| *e).collect::<BTreeSet<_>>();
        if boundary != loop_edges {
            return Ok(Err("side_open_or_nonclosed_rim".into()));
        }
        let mut angle = 0.;
        for e in &boundary {
            angle += c.describe(*e)?["arc_coverage_rad"].as_f64().unwrap();
        }
        if (angle - std::f64::consts::TAU).abs() * radius > c.tolerance.linear_mm {
            return Ok(Err("incomplete_rim".into()));
        }
        let ld = c.describe(l)?;
        ends.push(Endpoint {
            plane: f,
            loop_: l,
            role: ld["role"].as_str().unwrap().into(),
            point: origin + axis * if i == 0 { lo } else { hi },
            normal,
            edges: boundary.into_iter().collect(),
            radius,
        });
    }
    for t in parts {
        if !material_witness(c, *t)? {
            return Ok(Err("material_side_unresolved".into()));
        }
    }
    Ok(Ok(Segment {
        faces: parts.into(),
        origin,
        axis,
        radius,
        interval: [lo, hi],
        ends,
    }))
}
pub(crate) fn recognize(c: &mut Context) -> Result<Recognition> {
    if let Some((tolerance, cache)) = &c.holes {
        if *tolerance == c.tolerance {
            return Ok(cache.clone());
        }
    }
    let mut out = Recognition::default();
    if !c.solid {
        out.uncertain
            .push(json!({"reason":"requires_closed_oriented_solid"}));
        return Ok(out);
    }
    let mut faces = BTreeMap::new();
    let mut cylinders = BTreeSet::new();
    for t in c.tokens(Kind::Face) {
        let d = c.describe(t)?;
        if d["surface_type"] == "cylinder" {
            if d["material_side"] == "internal" {
                cylinders.insert(t);
            } else if d["material_side"] == "external" {
                out.excluded
                    .push(json!({"face":c.reference(t),"reason":"external_cylindrical_material"}));
            } else {
                out.uncertain
                    .push(json!({"face":c.reference(t),"reason":"unknown_material_orientation"}));
            }
        } else if ["cone", "nurbs", "other"]
            .iter()
            .any(|s| d["surface_type"] == *s)
        {
            out.uncertain.push(
                json!({"face":c.reference(t),"reason":"unsupported_non_cylindrical_surface"}),
            );
        }
        faces.insert(t, d);
    }
    let mut segments = vec![];
    while let Some(seed) = cylinders.pop_first() {
        let mut group = vec![seed];
        let mut i = 0;
        while i < group.len() {
            let t = group[i];
            let a = &faces[&t];
            for reference in a["adjacent_faces"].as_array().unwrap() {
                let b = token(reference);
                if cylinders.contains(&b)
                    && coaxis(a, &faces[&b], c.tolerance)
                    && (a["radius_mm"].as_f64().unwrap() - faces[&b]["radius_mm"].as_f64().unwrap())
                        .abs()
                        <= c.tolerance.linear_mm
                {
                    cylinders.remove(&b);
                    group.push(b);
                }
            }
            i += 1;
        }
        group.sort();
        match build_segment(c,&group,&faces)?{Ok(s)=>segments.push(s),Err(reason)=>out.uncertain.push(json!({"source_refs":group.iter().map(|t|c.reference(*t)).collect::<Vec<_>>(),"reason":reason}))}
    }
    let mut links: BTreeMap<(usize, usize), (usize, usize)> = BTreeMap::new();
    for a in 0..segments.len() {
        for b in a + 1..segments.len() {
            c.visited += 1;
            if c.visited > 200_000 {
                return Err(CadError::new(
                    "QUERY_LIMIT",
                    "segment adjacency budget exceeded",
                ));
            }
            for ea in 0..2 {
                for eb in 0..2 {
                    let sa = &segments[a];
                    let sb = &segments[b];
                    let aa = &sa.ends[ea];
                    let bb = &sb.ends[eb];
                    if aa.plane == bb.plane
                        && aa.role != bb.role
                        && parallel(sa.axis, sb.axis, c.tolerance.angular_rad)
                        && (aa.point - bb.point).length() <= c.tolerance.linear_mm
                        && faces[&aa.plane]["loops"].as_array().unwrap().len() == 2
                    {
                        let expected =
                            std::f64::consts::PI * (aa.radius.powi(2) - bb.radius.powi(2)).abs();
                        let actual = faces[&aa.plane]["area_mm2"].as_f64().unwrap();
                        if (expected - actual).abs()
                            <= c.tolerance.linear_mm
                                * std::f64::consts::TAU
                                * (aa.radius + bb.radius)
                        {
                            links.insert((a, ea), (b, eb));
                            links.insert((b, eb), (a, ea));
                        }
                    }
                }
            }
        }
    }
    let mut unused = (0..segments.len()).collect::<BTreeSet<_>>();
    while let Some(seed) = unused.pop_first() {
        let mut indices = vec![seed];
        let mut i = 0;
        while i < indices.len() {
            for end in 0..2 {
                if let Some((other, _)) = links.get(&(indices[i], end)) {
                    if unused.remove(other) {
                        indices.push(*other);
                    }
                }
            }
            i += 1;
        }
        let mut openings = vec![];
        let mut caps = vec![];
        let mut bad = false;
        let mut refs = vec![];
        for &index in &indices {
            let s = &segments[index];
            refs.extend(&s.faces);
            for end in 0..2 {
                if links.contains_key(&(index, end)) {
                    continue;
                }
                let e = &s.ends[end];
                if e.role == "inner" {
                    openings.push(e.clone());
                } else if faces[&e.plane]["loops"].as_array().unwrap().len() == 1
                    && (faces[&e.plane]["area_mm2"].as_f64().unwrap()
                        - std::f64::consts::PI * s.radius * s.radius)
                        .abs()
                        <= c.tolerance.linear_mm * std::f64::consts::TAU * s.radius
                {
                    caps.push(e.clone());
                } else {
                    bad = true;
                }
            }
        }
        refs.sort();
        refs.dedup();
        if bad
            || !((openings.len() == 2 && caps.is_empty())
                || (openings.len() == 1 && caps.len() == 1))
        {
            out.uncertain.push(json!({"source_refs":refs.iter().map(|t|c.reference(*t)).collect::<Vec<_>>(),"reason":"unresolved_termination_or_branch"}));
            continue;
        }
        openings.sort_by_key(|e| (e.plane, e.loop_));
        let entry = &openings[0];
        let inward = -entry.normal;
        let mut rows = vec![];
        let mut depth = 0_f64;
        for &index in &indices {
            let s = &segments[index];
            let mut interval = s
                .interval
                .map(|t| (s.origin + s.axis * t - entry.point).dot(inward));
            interval.sort_by(f64::total_cmp);
            depth = depth.max(interval[1]);
            rows.push(json!({"radius_mm":s.radius,"diameter_mm":2.*s.radius,"axial_interval_mm":interval,"length_mm":interval[1]-interval[0],"source_refs":s.faces.iter().map(|t|c.reference(*t)).collect::<Vec<_>>()}));
        }
        rows.sort_by(|a, b| {
            a["axial_interval_mm"][0]
                .as_f64()
                .unwrap()
                .total_cmp(&b["axial_interval_mm"][0].as_f64().unwrap())
        });
        out.holes.push(json!({"id":format!("hole-{}",refs.iter().map(|t|format!("t{t}")).collect::<Vec<_>>().join("-")),"source_refs":refs.iter().map(|t|c.reference(*t)).collect::<Vec<_>>(),"axis":{"point_mm":arr(entry.point),"direction":arr(inward),"direction_semantics":"inward_from_first_reported_opening"},"termination":if caps.is_empty(){"through"}else{"blind"},"depth_mm":depth,"depth_semantics":"first_opening_to_opposite_end_along_inward_axis","segments":rows,"openings":openings.iter().map(|e|json!({"support_face":c.reference(e.plane),"loop":c.reference(e.loop_),"edge_refs":e.edges.iter().map(|t|c.reference(*t)).collect::<Vec<_>>(),"center_mm":arr(e.point),"normal":arr(e.normal),"inward_direction":arr(-e.normal),"diameter_mm":2.*e.radius})).collect::<Vec<_>>(),"evidence":{"connected_topology":true,"full_angular_coverage":true,"closed_planar_rims":true,"material_witnesses":true},"thread":"unknown","provenance":"recognized"}));
    }
    out.holes
        .sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    c.holes = Some((c.tolerance, out.clone()));
    Ok(out)
}
pub(crate) fn query(c: &mut Context, filter: &Filter) -> Result<Recognition> {
    if filter.opening_on.len() > 256 {
        return Err(CadError::new("QUERY_LIMIT", "at most 256 support refs"));
    }
    if let Some(r) = &filter.diameter_mm {
        selection::check_range(r)?;
    }
    let wanted = filter
        .opening_on
        .iter()
        .map(|r| {
            let t = c.resolve(r)?;
            if c.kind(t) != Kind::Face {
                return Err(CadError::new(
                    "QUERY_ARGUMENT",
                    "opening_on requires face refs",
                ));
            }
            Ok(t)
        })
        .collect::<Result<BTreeSet<_>>>()?;
    let axis = filter
        .axis_parallel_to
        .as_ref()
        .map(|a| selection::direction_vector(c, a))
        .transpose()?;
    let mut out = recognize(c)?;
    out.holes.retain_mut(|h| {
        if filter
            .termination
            .as_ref()
            .is_some_and(|t| json!(t) != h["termination"])
        {
            return false;
        }
        if axis.is_some_and(|a| !parallel(a, xyz(&h["axis"]["direction"]), c.tolerance.angular_rad))
        {
            return false;
        }
        let matching =
            h["openings"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .filter_map(|(i, o)| {
                    if !wanted.is_empty() && !wanted.contains(&token(&o["support_face"])) {
                        return None;
                    }
                    let d = o["diameter_mm"].as_f64().unwrap();
                    if filter.diameter_mm.as_ref().is_some_and(|r| {
                        r.min.is_some_and(|v| d < v) || r.max.is_some_and(|v| d > v)
                    }) {
                        return None;
                    }
                    Some(i)
                })
                .collect::<Vec<_>>();
        h["matching_openings"] = json!(matching);
        !matching.is_empty()
    });
    Ok(out)
}
