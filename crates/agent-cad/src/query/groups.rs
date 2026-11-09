//! Plane-local pattern hypotheses. Subsets retain every excluded opening and symmetry.
use super::{
    contract::*,
    engine::{arr, unit, vec, Context},
};
use crate::*;
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "HolePattern")]
pub enum Pattern {
    Linear,
    Rectangular,
    Circular,
    Unstructured,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "HoleGroupFilter")]
pub struct Filter {
    pub pattern: Option<Pattern>,
    pub member_count: Option<usize>,
    #[serde(default)]
    pub equal_diameter: bool,
}
#[derive(Clone, Debug, Default)]
pub(crate) struct Recognition {
    pub groups: Vec<Value>,
    pub diagnostics: Vec<Value>,
    pub partial: bool,
}
fn xyz(v: &Value) -> Vec3 {
    vec(serde_json::from_value(v.clone()).unwrap())
}
fn tok(v: &Value) -> u64 {
    v["topology_id"].as_str().unwrap()[1..].parse().unwrap()
}
fn tick(c: &mut Context) -> Result<()> {
    c.hypotheses += 1;
    if c.hypotheses > 10_000 {
        Err(CadError::new(
            "QUERY_LIMIT",
            "pattern hypotheses exceed 10000",
        ))
    } else {
        Ok(())
    }
}
fn unique_near(points: &[Vec3], p: Vec3, tol: f64) -> Option<usize> {
    let matched = points
        .iter()
        .enumerate()
        .filter(|(_, v)| (**v - p).length() <= tol)
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    (matched.len() == 1).then(|| matched[0])
}
fn same_diameter(members: &[Value], indices: &[usize], tol: f64) -> bool {
    let d = members[indices[0]]["diameter_mm"].as_f64().unwrap();
    indices
        .iter()
        .all(|i| (members[*i]["diameter_mm"].as_f64().unwrap() - d).abs() <= tol)
}
#[allow(clippy::too_many_arguments)]
fn add(
    out: &mut BTreeMap<String, Value>,
    plane: &Value,
    members: &[Value],
    points: &[Vec3],
    mut indices: Vec<usize>,
    pattern: Pattern,
    parameters: Value,
    filter: &Filter,
    tol: f64,
    residual: f64,
) {
    indices.sort();
    indices.dedup();
    if indices.is_empty()
        || filter.pattern.as_ref().is_some_and(|p| *p != pattern)
        || filter.member_count.is_some_and(|n| n != indices.len())
        || (filter.equal_diameter && !same_diameter(members, &indices, tol))
    {
        return;
    }
    let residual = if pattern == Pattern::Linear {
        let u = xyz(&parameters["axis_direction"]);
        let pitch = parameters["pitch_mm"].as_f64().unwrap();
        indices
            .iter()
            .map(|i| {
                let d = points[*i] - points[indices[0]];
                (d - u * (d.dot(u) / pitch).round() * pitch).length()
            })
            .fold(residual, f64::max)
    } else {
        residual
    };
    if residual > tol {
        return;
    }
    let kind = serde_json::to_value(&pattern).unwrap();
    let key = format!(
        "{}-{}",
        kind.as_str().unwrap(),
        indices
            .iter()
            .map(|i| members[*i]["hole_id"].as_str().unwrap())
            .collect::<Vec<_>>()
            .join("-")
    );
    if out.contains_key(&key) {
        return;
    }
    let center = indices.iter().fold(Vec3::ZERO, |s, i| s + points[*i]) / indices.len() as f64;
    let mut distances = vec![];
    for (ii, i) in indices.iter().enumerate() {
        for j in indices.iter().skip(ii + 1) {
            distances.push(json!({"a":members[*i]["hole_id"],"b":members[*j]["hole_id"],"distance_mm":(points[*i]-points[*j]).length()}));
        }
    }
    let rotations = match pattern {
        Pattern::Linear => vec![0., std::f64::consts::PI],
        Pattern::Rectangular => {
            let pitch = parameters["pitch_mm"].as_array().unwrap();
            let count = parameters["count"].as_array().unwrap();
            if count[0] == count[1]
                && (pitch[0].as_f64().unwrap() - pitch[1].as_f64().unwrap()).abs() <= tol
            {
                vec![
                    0.,
                    std::f64::consts::FRAC_PI_2,
                    std::f64::consts::PI,
                    3. * std::f64::consts::FRAC_PI_2,
                ]
            } else {
                vec![0., std::f64::consts::PI]
            }
        }
        Pattern::Circular => (0..indices.len())
            .map(|i| i as f64 * std::f64::consts::TAU / indices.len() as f64)
            .collect(),
        Pattern::Unstructured => vec![],
    };
    out.insert(key.clone(),json!({"id":format!("group-t{}-{key}",tok(&plane["ref"])),"pattern":kind,"support_face":plane["ref"],"normal":plane["normal"],"center_mm":arr(center),"members":indices.iter().map(|i|members[*i].clone()).collect::<Vec<_>>(),"excluded_members":members.iter().enumerate().filter(|(i,_)|!indices.contains(i)).map(|(_,m)|m.clone()).collect::<Vec<_>>(),"parameters":parameters,"pairwise_distances":distances,"max_fit_residual_mm":residual,"equal_diameter":same_diameter(members,&indices,tol),"orientation_unique":false,"continuous_freedoms":if indices.len()==1{vec!["rotation_about_normal"]}else{vec![]},"symmetry":{"rotations_about_normal_rad":rotations,"orientation_semantics":"requires_explicit_orientation_datum_for_fixed_attachment"},"provenance":"recognized"}));
}
pub(crate) fn recognize(c: &mut Context, support: &[u64], filter: &Filter) -> Result<Recognition> {
    if filter.member_count.is_some_and(|n| n == 0 || n > 64) {
        return Err(CadError::new(
            "QUERY_ARGUMENT",
            "member_count must be 1..64",
        ));
    }
    let holes = super::holes::recognize(c)?;
    let mut diagnostics = vec![];
    let mut partial = false;
    for mut diagnostic in holes.uncertain {
        let mut refs = diagnostic["source_refs"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        if !diagnostic["face"].is_null() {
            refs.push(diagnostic["face"].clone());
        }
        let mut relevant = support.is_empty() || refs.is_empty();
        for reference in refs {
            let face = tok(&reference);
            let descriptor = c.describe(face)?;
            relevant |= support.contains(&face)
                || descriptor["adjacent_faces"]
                    .as_array()
                    .is_some_and(|neighbors| neighbors.iter().any(|r| support.contains(&tok(r))));
        }
        diagnostic["scope"] = json!(if relevant {
            "affects_requested_support"
        } else {
            "outside_requested_support_adjacency"
        });
        partial |= relevant;
        diagnostics.push(diagnostic);
    }
    let mut out = Recognition {
        partial,
        diagnostics,
        groups: vec![],
    };
    let mut planes: BTreeMap<u64, Vec<Value>> = BTreeMap::new();
    for h in holes.holes {
        for (index, opening) in h["openings"].as_array().unwrap().iter().enumerate() {
            let t = tok(&opening["support_face"]);
            if support.is_empty() || support.contains(&t) {
                let mut m = opening.clone();
                m["hole_id"] = h["id"].clone();
                m["opening_index"] = json!(index);
                m["termination"] = h["termination"].clone();
                m["source_refs"] = h["source_refs"].clone();
                planes.entry(t).or_default().push(m);
            }
        }
    }
    for (t, members) in planes {
        if members.len() > 64 {
            return Err(CadError::new(
                "QUERY_LIMIT",
                "at most 64 recognized openings per support plane",
            ));
        }
        let plane = c.describe(t)?;
        if plane["surface_type"] != "plane" || plane["normal"].is_null() {
            return Err(CadError::new(
                "QUERY_ARGUMENT",
                "hole group support requires an oriented plane",
            ));
        }
        let points = members
            .iter()
            .map(|m| xyz(&m["center_mm"]))
            .collect::<Vec<_>>();
        let normal = xyz(&plane["normal"]);
        let tol = c.tolerance.linear_mm;
        let n = points.len();
        let mut found = BTreeMap::new();
        let origin = xyz(&plane["plane_point_mm"]);
        if points.iter().any(|p| (*p - origin).dot(normal).abs() > tol) {
            return Err(CadError::new(
                "QUERY_UNSUPPORTED",
                "recognized openings are not coplanar",
            ));
        }
        add(
            &mut found,
            &plane,
            &members,
            &points,
            (0..n).collect(),
            Pattern::Unstructured,
            json!({}),
            filter,
            tol,
            0.,
        );
        for i in 0..n {
            for j in i + 1..n {
                let delta = points[j] - points[i];
                let pitch = delta.length();
                if pitch <= tol {
                    continue;
                }
                let u = delta / pitch;
                if filter.pattern.is_none() || filter.pattern == Some(Pattern::Linear) {
                    tick(c)?;
                    let mut nodes = points
                        .iter()
                        .enumerate()
                        .filter_map(|(k, p)| {
                            let d = *p - points[i];
                            let x = d.dot(u) / pitch;
                            let rounded = x.round();
                            (d.cross(u).length() <= tol && (x - rounded).abs() * pitch <= tol)
                                .then_some((rounded as i64, k))
                        })
                        .collect::<Vec<_>>();
                    nodes.sort();
                    let mut runs = vec![];
                    let mut run = vec![];
                    let mut previous = None;
                    for (x, k) in nodes {
                        if previous.is_some_and(|v| x != v + 1) {
                            runs.push(std::mem::take(&mut run));
                        }
                        run.push(k);
                        previous = Some(x);
                    }
                    runs.push(run);
                    for run in runs {
                        let count = filter.member_count.unwrap_or(run.len());
                        if count < 2 || count > run.len() {
                            continue;
                        }
                        for subset in run.windows(count) {
                            tick(c)?;
                            add(
                                &mut found,
                                &plane,
                                &members,
                                &points,
                                subset.to_vec(),
                                Pattern::Linear,
                                json!({"axis_direction":arr(u),"pitch_mm":pitch}),
                                filter,
                                tol,
                                0.,
                            );
                        }
                    }
                }
            }
        }
        if filter.pattern.is_none() || filter.pattern == Some(Pattern::Rectangular) {
            for i in 0..n {
                for j in 0..n {
                    if i == j {
                        continue;
                    }
                    let dx = points[j] - points[i];
                    let lx = dx.length();
                    if lx <= tol {
                        continue;
                    }
                    let u = dx / lx;
                    for k in 0..n {
                        if k == i || k == j {
                            continue;
                        }
                        tick(c)?;
                        let dy = points[k] - points[i];
                        let ly = dy.length();
                        if ly <= tol || u.dot(dy).abs() > tol {
                            continue;
                        }
                        let v = dy / ly;
                        let fourth = points[i] + dx + dy;
                        let Some(l) = unique_near(&points, fourth, tol) else {
                            continue;
                        };
                        let residual = (points[l] - fourth).length();
                        add(
                            &mut found,
                            &plane,
                            &members,
                            &points,
                            vec![i, j, k, l],
                            Pattern::Rectangular,
                            json!({"count":[2,2],"pitch_mm":[lx,ly],"axes":[arr(u),arr(v)]}),
                            filter,
                            tol,
                            residual,
                        );
                        let px = points
                            .iter()
                            .filter_map(|p| {
                                let d = *p - points[i];
                                let x = d.dot(u);
                                (x > tol && x <= lx + tol && d.dot(v).abs() <= tol).then_some(x)
                            })
                            .min_by(f64::total_cmp)
                            .unwrap_or(lx);
                        let py = points
                            .iter()
                            .filter_map(|p| {
                                let d = *p - points[i];
                                let y = d.dot(v);
                                (y > tol && y <= ly + tol && d.dot(u).abs() <= tol).then_some(y)
                            })
                            .min_by(f64::total_cmp)
                            .unwrap_or(ly);
                        let nx = ((lx / px).round() as usize).saturating_add(1);
                        let ny = ((ly / py).round() as usize).saturating_add(1);
                        if nx > 64
                            || ny > 64
                            || nx * ny <= 4
                            || nx * ny > 64
                            || (px * (nx - 1) as f64 - lx).abs() > tol
                            || (py * (ny - 1) as f64 - ly).abs() > tol
                        {
                            continue;
                        }
                        let mut grid = vec![];
                        let mut worst = 0_f64;
                        for ix in 0..nx {
                            for iy in 0..ny {
                                let p = points[i] + u * (ix as f64 * px) + v * (iy as f64 * py);
                                if let Some(q) = unique_near(&points, p, tol) {
                                    grid.push(q);
                                    worst = worst.max((points[q] - p).length());
                                }
                            }
                        }
                        if grid.len() == nx * ny {
                            add(
                                &mut found,
                                &plane,
                                &members,
                                &points,
                                grid,
                                Pattern::Rectangular,
                                json!({"count":[nx,ny],"pitch_mm":[px,py],"axes":[arr(u),arr(v)]}),
                                filter,
                                tol,
                                worst,
                            );
                        }
                    }
                }
            }
        }
        if filter.pattern.is_none() || filter.pattern == Some(Pattern::Circular) {
            let mut centers: Vec<(Vec3, f64)> = vec![];
            for i in 0..n {
                for j in i + 1..n {
                    for k in j + 1..n {
                        tick(c)?;
                        let a = points[j] - points[i];
                        let b = points[k] - points[i];
                        let cross = a.cross(b);
                        let denom = 2. * cross.length_squared();
                        if denom <= tol.powi(4) {
                            continue;
                        }
                        let offset = (b.cross(cross) * a.length_squared()
                            + cross.cross(a) * b.length_squared())
                            / denom;
                        let center = points[i] + offset;
                        let radius = offset.length();
                        if radius <= tol || !radius.is_finite() {
                            continue;
                        }
                        if centers.iter().any(|(p, r)| {
                            (*p - center).length() <= tol && (*r - radius).abs() <= tol
                        }) {
                            continue;
                        }
                        centers.push((center, radius));
                        let ring = (0..n)
                            .filter(|q| ((points[*q] - center).length() - radius).abs() <= tol)
                            .collect::<Vec<_>>();
                        let counts: Vec<usize> = if let Some(count) = filter.member_count {
                            vec![count]
                        } else {
                            (3..=ring.len()).collect()
                        };
                        for count in counts {
                            if count < 3 || count > ring.len() {
                                continue;
                            }
                            for &seed in &ring {
                                tick(c)?;
                                let u = unit(arr(points[seed] - center))?;
                                let v = normal.cross(u);
                                let mut members_on_ring = vec![];
                                let mut worst = 0_f64;
                                for q in 0..count {
                                    let angle = q as f64 * std::f64::consts::TAU / count as f64;
                                    let expected =
                                        center + (u * angle.cos() + v * angle.sin()) * radius;
                                    if let Some(p) = unique_near(&points, expected, tol) {
                                        members_on_ring.push(p);
                                        worst = worst.max((points[p] - expected).length());
                                    }
                                }
                                if members_on_ring.len() == count {
                                    add(
                                        &mut found,
                                        &plane,
                                        &members,
                                        &points,
                                        members_on_ring,
                                        Pattern::Circular,
                                        json!({"center_mm":arr(center),"radius_mm":radius,"pcd_mm":2.*radius,"count":count,"angular_spacing_rad":std::f64::consts::TAU/count as f64}),
                                        filter,
                                        tol,
                                        worst,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        out.groups.extend(found.into_values());
    }
    out.groups
        .sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    Ok(out)
}
pub(crate) fn query(
    c: &mut Context,
    support: &[TopologyRef],
    filter: &Filter,
) -> Result<Recognition> {
    if support.len() > 256 {
        return Err(CadError::new("QUERY_LIMIT", "at most 256 support refs"));
    }

    let ids = support
        .iter()
        .map(|r| {
            let t = c.resolve(r)?;
            if c.kind(t) != Kind::Face {
                return Err(CadError::new(
                    "QUERY_ARGUMENT",
                    "support_faces requires faces",
                ));
            }
            Ok(t)
        })
        .collect::<Result<Vec<_>>>()?;
    recognize(c, &ids, filter)
}
