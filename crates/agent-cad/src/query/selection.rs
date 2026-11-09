//! Bounded topology selection, cardinality and primitive datum derivation.
use super::{
    contract::*,
    engine::{unit, Context},
};
use crate::*;
pub(crate) fn check_range(r: &Range) -> Result<()> {
    if r.min.is_none() && r.max.is_none()
        || r.min.into_iter().chain(r.max).any(|n| !n.is_finite())
        || r.min.zip(r.max).is_some_and(|(a, b)| a > b)
    {
        return Err(CadError::new(
            "QUERY_ARGUMENT",
            "range must be finite, nonempty and ordered",
        ));
    }
    Ok(())
}
pub(crate) fn filter_budget(f: &Filter, depth: usize, nodes: &mut usize) -> Result<()> {
    *nodes += 1;
    if depth > 8 || *nodes > 128 {
        return Err(CadError::new(
            "QUERY_LIMIT",
            "selection/filter exceeds depth 8 or 128 nodes",
        ));
    }
    for r in [&f.area_mm2, &f.length_mm, &f.radius_mm]
        .into_iter()
        .flatten()
    {
        check_range(r)?;
    }
    for v in [&f.planar_normal_parallel_to, &f.planar_normal_aligned_with]
        .into_iter()
        .flatten()
    {
        validate_direction(v)?;
    }
    if let Some(d) = &f.planar_coplanar_with {
        if !valid_id(&d.datum) {
            return Err(CadError::new(
                "DATUM_REFERENCE",
                "invalid coplanar datum name",
            ));
        }
    }
    for child in f.all.iter().chain(&f.any).chain(f.not.iter().map(|v| &**v)) {
        filter_budget(child, depth + 1, nodes)?;
    }
    Ok(())
}
pub(crate) fn matches(c: &Context, f: &Filter, v: &Value) -> Result<bool> {
    if f.name.as_ref().is_some_and(|name| {
        !v["names"]
            .as_array()
            .is_some_and(|n| n.iter().any(|n| n == name))
    }) {
        return Ok(false);
    }
    for (test, key) in [
        (f.surface_type.as_ref().map(|s| json!(s)), "surface_type"),
        (f.curve_type.as_ref().map(|s| json!(s)), "curve_type"),
        (f.loop_role.as_ref().map(|s| json!(s)), "role"),
    ] {
        if test.is_some_and(|x| v[key] != x) {
            return Ok(false);
        }
    }
    for (r, key) in [
        (&f.area_mm2, "area_mm2"),
        (&f.length_mm, "length_mm"),
        (&f.radius_mm, "radius_mm"),
    ] {
        if let Some(r) = r {
            let Some(n) = v[key].as_f64() else {
                return Ok(false);
            };
            if r.min.is_some_and(|x| n < x) || r.max.is_some_and(|x| n > x) {
                return Ok(false);
            }
        }
    }
    for (direction, parallel) in [
        (&f.planar_normal_parallel_to, true),
        (&f.planar_normal_aligned_with, false),
    ] {
        if let Some(direction) = direction {
            let Some(a) = v["normal"].as_array() else {
                return Ok(false);
            };
            let n = unit([
                a[0].as_f64().unwrap(),
                a[1].as_f64().unwrap(),
                a[2].as_f64().unwrap(),
            ])?;
            let axis = direction_vector(c, direction)?;
            let dot = n.dot(axis);
            let dot = if parallel { dot.abs() } else { dot };
            if n.cross(axis).length().atan2(dot) > c.tolerance.angular_rad {
                return Ok(false);
            }
        }
    }
    if let Some(input) = &f.planar_coplanar_with {
        let datum = c
            .manifest
            .body
            .datums
            .get(&c.definition)
            .and_then(|ds| ds.get(&input.datum))
            .ok_or_else(|| CadError::new("DATUM_REFERENCE", &input.datum))?;
        if datum.kind != "plane" {
            return Err(CadError::new(
                "DATUM_KIND_MISMATCH",
                "coplanar predicate requires a plane datum",
            ));
        }
        if v["surface_type"] != "plane" || v["normal"].is_null() {
            return Ok(false);
        }
        let vector = |v: &Value| -> Result<Vec3> {
            Ok(super::engine::vec(
                serde_json::from_value(v.clone())
                    .map_err(|_| CadError::new("DATUM_GEOMETRY", "invalid plane vector"))?,
            ))
        };
        let n = c.pose.vector(vector(&datum.geometry["normal"])?);
        let o = c.pose.point(vector(&datum.geometry["point_mm"])?);
        let normal = vector(&v["normal"])?;
        let point = vector(&v["plane_point_mm"])?;
        if n.cross(normal).length().atan2(n.dot(normal).abs()) > c.tolerance.angular_rad
            || (point - o).dot(n).abs() > c.tolerance.linear_mm
        {
            return Ok(false);
        }
    }
    for child in &f.all {
        if !matches(c, child, v)? {
            return Ok(false);
        }
    }
    if !f.any.is_empty() {
        let mut any = false;
        for child in &f.any {
            any |= matches(c, child, v)?;
        }
        if !any {
            return Ok(false);
        }
    }
    if let Some(child) = &f.not {
        if matches(c, child, v)? {
            return Ok(false);
        }
    }
    Ok(true)
}
pub(crate) fn selection_budget(s: &Selection, depth: usize, nodes: &mut usize) -> Result<()> {
    *nodes += 1;
    if depth > 8 || *nodes > 128 {
        return Err(CadError::new(
            "QUERY_LIMIT",
            "selection exceeds depth 8 or 128 nodes",
        ));
    }
    match s {
        Selection::Entities { .. } => {}
        Selection::Refs { refs } => {
            if refs.is_empty() || refs.len() > 256 {
                return Err(CadError::new(
                    "QUERY_LIMIT",
                    "refs must contain 1..256 entries",
                ));
            }
        }
        Selection::Filter { input, r#where } => {
            selection_budget(input, depth + 1, nodes)?;
            filter_budget(r#where, depth + 1, nodes)?;
        }
        Selection::Children { input, .. }
        | Selection::Ancestors { input, .. }
        | Selection::Siblings { input, .. }
        | Selection::Expect { input, .. } => selection_budget(input, depth + 1, nodes)?,
        Selection::Rank {
            input,
            by,
            tie_tolerance,
            ..
        } => {
            if !tie_tolerance.is_finite() || *tie_tolerance < 0. {
                return Err(CadError::new(
                    "QUERY_ARGUMENT",
                    "tie tolerance must be finite/nonnegative",
                ));
            }
            if let RankBy::CenterProjection { direction } = by {
                validate_direction(direction)?;
            }
            selection_budget(input, depth + 1, nodes)?;
        }
        Selection::Union { inputs } | Selection::Intersection { inputs } => {
            if inputs.is_empty() {
                return Err(CadError::new(
                    "QUERY_ARGUMENT",
                    "set operation needs inputs",
                ));
            }
            for input in inputs {
                selection_budget(input, depth + 1, nodes)?;
            }
        }
        Selection::Difference { left, right } => {
            selection_budget(left, depth + 1, nodes)?;
            selection_budget(right, depth + 1, nodes)?;
        }
    }
    Ok(())
}
fn set(v: Vec<u64>) -> BTreeSet<u64> {
    v.into_iter().collect()
}
pub(crate) fn select(c: &mut Context, s: &Selection) -> Result<BTreeSet<u64>> {
    match s {
        Selection::Entities { entity } => Ok(set(c.tokens(*entity))),
        Selection::Refs { refs } => refs.iter().map(|r| c.resolve(r)).collect(),
        Selection::Filter { input, r#where } => {
            let mut result = BTreeSet::new();
            for t in select(c, input)? {
                let v = c.describe(t)?;
                if matches(c, r#where, &v)? {
                    result.insert(t);
                }
            }
            Ok(result)
        }
        Selection::Children { input, entity } => {
            let mut result = BTreeSet::new();
            for t in select(c, input)? {
                result.extend(c.children(t, *entity)?);
            }
            Ok(result)
        }
        Selection::Ancestors { input, entity } => {
            let selected = select(c, input)?;
            let mut result = BTreeSet::new();
            for parent in c.tokens(*entity) {
                for kind in [Kind::Face, Kind::Edge, Kind::Vertex, Kind::Loop] {
                    if c.children(parent, kind)?
                        .iter()
                        .any(|t| selected.contains(t))
                    {
                        result.insert(parent);
                        break;
                    }
                }
            }
            Ok(result)
        }
        Selection::Siblings { input, entity, via } => {
            let selected = select(c, input)?;
            let mut shared = BTreeSet::new();
            for t in &selected {
                shared.extend(c.children(*t, *via)?);
            }
            let mut result = BTreeSet::new();
            for candidate in c.tokens(*entity) {
                if !selected.contains(&candidate)
                    && c.children(candidate, *via)?
                        .iter()
                        .any(|t| shared.contains(t))
                {
                    result.insert(candidate);
                }
            }
            Ok(result)
        }
        Selection::Union { inputs } => {
            let mut result = BTreeSet::new();
            for input in inputs {
                result.extend(select(c, input)?);
            }
            Ok(result)
        }
        Selection::Intersection { inputs } => {
            let mut result = select(c, &inputs[0])?;
            for input in &inputs[1..] {
                let next = select(c, input)?;
                result = result.intersection(&next).copied().collect();
            }
            Ok(result)
        }
        Selection::Difference { left, right } => {
            let left = select(c, left)?;
            let right = select(c, right)?;
            Ok(left.difference(&right).copied().collect())
        }
        Selection::Expect { input, count } => {
            let result = select(c, input)?;
            if result.len() != *count {
                return Err(CadError::new(
                    "SELECTION_CARDINALITY",
                    format!(
                        "expected {count}, complete selection contains {} entities",
                        result.len()
                    ),
                ));
            }
            Ok(result)
        }
        Selection::Rank {
            input,
            by,
            descending,
            rank,
            tie_tolerance,
        } => {
            let mut values = vec![];
            for t in select(c, input)? {
                let v = c.describe(t)?;
                let n = match by {
                    RankBy::AreaMm2 => v["area_mm2"].as_f64(),
                    RankBy::LengthMm => v["length_mm"].as_f64(),
                    RankBy::CenterProjection { direction } => {
                        let axis = direction_vector(c, direction)?;
                        v["center_mm"].as_array().map(|a| {
                            axis.dot(super::engine::vec([
                                a[0].as_f64().unwrap(),
                                a[1].as_f64().unwrap(),
                                a[2].as_f64().unwrap(),
                            ]))
                        })
                    }
                }
                .ok_or_else(|| {
                    CadError::new(
                        "QUERY_ARGUMENT",
                        "ranking property is not defined for selected entity kind",
                    )
                })?;
                if !n.is_finite() {
                    return Err(CadError::new(
                        "QUERY_UNSUPPORTED",
                        "non-finite ranking quantity",
                    ));
                }
                values.push((t, n));
            }
            values.sort_by(|a, b| {
                if *descending {
                    b.1.total_cmp(&a.1)
                } else {
                    a.1.total_cmp(&b.1)
                }
                .then(a.0.cmp(&b.0))
            });
            let mut groups: Vec<(f64, BTreeSet<u64>)> = vec![];
            for (t, n) in values {
                if let Some((anchor, items)) = groups.last_mut() {
                    if (n - *anchor).abs() <= *tie_tolerance {
                        items.insert(t);
                        continue;
                    }
                }
                groups.push((n, BTreeSet::from([t])));
            }
            Ok(groups
                .get(*rank)
                .map(|(_, v)| v.clone())
                .unwrap_or_default())
        }
    }
}
pub(crate) fn datum(c: &mut Context, t: u64, derive: &Derive) -> Result<Value> {
    let v = c.describe(t)?;
    let (kind, geometry, freedoms) = match derive {
        Derive::Point => {
            let p = if v["kind"] == "vertex" {
                v["point_mm"].clone()
            } else if v["curve_type"] == "circle" {
                v["center_mm"].clone()
            } else {
                return Err(CadError::new(
                    "DATUM_UNSUPPORTED",
                    "point requires a vertex or analytic circle center",
                ));
            };
            (
                "point",
                json!({"point_mm":p}),
                json!(["rotation_x", "rotation_y", "rotation_z"]),
            )
        }
        Derive::Plane => {
            if v["surface_type"] != "plane" || v["normal"].is_null() {
                return Err(CadError::new(
                    "DATUM_UNSUPPORTED",
                    "plane requires an analytic face with known material normal",
                ));
            }
            (
                "plane",
                json!({"point_mm":v["plane_point_mm"],"normal":v["normal"]}),
                json!([
                    "in_plane_translation_1",
                    "in_plane_translation_2",
                    "rotation_about_normal"
                ]),
            )
        }
        Derive::Axis => {
            let axis = if v["surface_type"] == "cylinder" {
                v["axis"].clone()
            } else if v["curve_type"] == "circle" {
                json!({"point_mm":v["center_mm"],"direction":v["axis_direction"],"direction_semantics":"unoriented_definition_canonical"})
            } else {
                return Err(CadError::new(
                    "DATUM_UNSUPPORTED",
                    "axis requires an analytic cylinder or circle",
                ));
            };
            (
                "axis",
                axis,
                json!(["translation_along_axis", "rotation_about_axis"]),
            )
        }
        Derive::Direction => {
            if v["curve_type"] != "line" || v["direction"].is_null() {
                return Err(CadError::new(
                    "DATUM_UNSUPPORTED",
                    "direction requires a nondegenerate straight edge",
                ));
            }
            (
                "direction",
                json!({"direction":v["direction"],"direction_semantics":"edge_parameter_direction"}),
                json!([
                    "translation_x",
                    "translation_y",
                    "translation_z",
                    "rotation_about_direction"
                ]),
            )
        }
    };
    Ok(
        json!({"kind":kind,"geometry":geometry,"source_refs":[c.reference(t)],"provenance":"derived","remaining_rigid_freedoms":freedoms,"engineering_fit":"not_asserted"}),
    )
}
pub(crate) fn run(c: &mut Context, request: &Request, q: &Query) -> Result<Value> {
    if let Operation::Interfaces {
        ids,
        profile,
        support_faces,
        r#where,
    } = &q.operation
    {
        let (items, partial, diagnostics) =
            super::interfaces::query(c, ids, profile, support_faces, r#where)?;
        let (offset, key) = page(request, q, items.len())?;
        let end = (offset + q.limit).min(items.len());
        return Ok(
            json!({"id":q.id,"op":"interfaces","status":if partial{"partial"}else{"complete"},"coverage":{"recognition_complete":!partial,"scope":if ids.is_none(){"supplied_support_faces_or_all_recognized_openings"}else{"named_declarations"}},"items":&items[offset..end],"total_count":items.len(),"returned_count":end-offset,"next_cursor":(end<items.len()).then(||format!("{key}:{end}")),"diagnostics":diagnostics}),
        );
    }
    if let Operation::HoleGroups {
        support_faces,
        r#where,
    } = &q.operation
    {
        let r = super::groups::query(c, support_faces, r#where)?;
        let (offset, key) = page(request, q, r.groups.len())?;
        let end = (offset + q.limit).min(r.groups.len());
        return Ok(
            json!({"id":q.id,"op":"hole_groups","status":if r.partial{"partial"}else{"complete"},"items":&r.groups[offset..end],"total_count":r.groups.len(),"returned_count":end-offset,"next_cursor":(end<r.groups.len()).then(||format!("{key}:{end}")),"coverage":{"recognition_complete":!r.partial,"scope":"supplied_support_faces_or_all_recognized_openings"},"diagnostics":r.diagnostics}),
        );
    }
    if let Operation::Holes { r#where } = &q.operation {
        let recognized = super::holes::query(c, r#where)?;
        let (offset, key) = page(request, q, recognized.holes.len())?;
        let end = (offset + q.limit).min(recognized.holes.len());
        return Ok(
            json!({"id":q.id,"op":"holes","status":if recognized.uncertain.is_empty(){"complete"}else{"partial"},"items":&recognized.holes[offset..end],"returned_count":end-offset,"total_count":recognized.holes.len(),"next_cursor":(end<recognized.holes.len()).then(||format!("{key}:{end}")),"coverage":{"domain":"connected_analytic_cylinders_planar_ends","uncertain":recognized.uncertain,"excluded":recognized.excluded},"diagnostics":[]}),
        );
    }
    if let Operation::NamedDatums { ids } = &q.operation {
        let unique: BTreeSet<_> = ids.iter().collect();
        if unique.is_empty() || unique.len() != ids.len() || ids.len() > 128 {
            return Err(CadError::new(
                "QUERY_ARGUMENT",
                "named_datums requires 1..128 unique IDs",
            ));
        }
        let datums =
            c.manifest.body.datums.get(&c.definition).ok_or_else(|| {
                CadError::new("DATUM_REFERENCE", "definition has no named datums")
            })?;
        let mut values = Vec::new();
        for id in unique {
            let datum = datums
                .get(id)
                .ok_or_else(|| CadError::new("DATUM_REFERENCE", id))?;
            let mut value =
                datum.observation(id, &c.definition, &c.manifest, c.pose, c.occurrence.clone())?;
            value["space"] = json!(request.space);
            values.push(value);
        }
        let (offset, key) = page(request, q, values.len())?;
        let end = (offset + q.limit).min(values.len());
        return Ok(
            json!({"id":q.id,"op":"named_datums","status":"complete","items":&values[offset..end],"total_count":values.len(),"returned_count":end-offset,"next_cursor":(end<values.len()).then(||format!("{key}:{end}")),"coverage":{"declarations":"requested_ids"},"diagnostics":[]}),
        );
    }
    if let Operation::EvaluateFace { r#ref, uv } = &q.operation {
        if q.cursor.is_some() {
            return Err(CadError::new(
                "QUERY_ARGUMENT",
                "face evaluation does not paginate",
            ));
        }
        let item = c.evaluate_face(r#ref, *uv)?;
        return Ok(
            json!({"id":q.id,"op":"evaluate_face","status":"complete","items":[item],"returned_count":1,"total_count":1,"next_cursor":null,"coverage":{"evaluation":"requested_uv_only"},"diagnostics":[]}),
        );
    }
    let mut nodes = 0;
    let selection = match &q.operation {
        Operation::Faces { r#where }
        | Operation::Edges { r#where }
        | Operation::Vertices { r#where }
        | Operation::Loops { r#where } => Selection::Filter {
            input: Box::new(Selection::Entities {
                entity: match q.operation {
                    Operation::Faces { .. } => Kind::Face,
                    Operation::Edges { .. } => Kind::Edge,
                    Operation::Vertices { .. } => Kind::Vertex,
                    _ => Kind::Loop,
                },
            }),
            r#where: r#where.clone(),
        },
        Operation::Select { selection } | Operation::Datums { selection, .. } => selection.clone(),
        Operation::Describe { refs } => Selection::Refs { refs: refs.clone() },
        Operation::EvaluateFace { .. }
        | Operation::NamedDatums { .. }
        | Operation::Holes { .. }
        | Operation::HoleGroups { .. }
        | Operation::Interfaces { .. }
        | Operation::Measure { .. } => unreachable!(),
    };
    selection_budget(&selection, 1, &mut nodes)?;
    let selected = select(c, &selection)?;
    let (offset, key) = page(request, q, selected.len())?;
    let mut items = Vec::new();
    for t in selected.iter().skip(offset).take(q.limit) {
        items.push(if let Operation::Datums { derive, .. } = &q.operation {
            datum(c, *t, derive)?
        } else {
            c.describe(*t)?
        });
    }
    let end = offset + items.len();
    let next = (end < selected.len()).then(|| format!("{key}:{end}"));
    Ok(
        json!({"id":q.id,"op":q.operation.name(),"status":"complete","returned_count":items.len(),"total_count":selected.len(),"items":items,"next_cursor":next,"coverage":{"selection":"complete","recognition":"not_requested"},"diagnostics":[]}),
    )
}

fn page(request: &Request, q: &Query, total: usize) -> Result<(usize, String)> {
    // Cursor identity binds every semantic input except the cursor itself and page size.
    let key = format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&(
                request.schema.clone(),
                request.query_revision_id.clone(),
                &request.target,
                request.space,
                request.tolerance,
                &q.operation
            ))
            .unwrap()
        )
    );
    let offset = if let Some(cursor) = &q.cursor {
        let (hash, offset) = cursor
            .split_once(':')
            .ok_or_else(|| CadError::new("QUERY_ARGUMENT", "invalid cursor"))?;
        if hash != key {
            return Err(CadError::new(
                "QUERY_REVISION_MISMATCH",
                "cursor belongs to a different query/revision",
            ));
        }
        offset
            .parse::<usize>()
            .map_err(|_| CadError::new("QUERY_ARGUMENT", "invalid cursor offset"))?
    } else {
        0
    };
    if offset > total {
        return Err(CadError::new(
            "QUERY_ARGUMENT",
            "cursor offset exceeds complete selection",
        ));
    }
    Ok((offset, key))
}

pub(crate) fn validate_direction(d: &DirectionInput) -> Result<()> {
    match d {
        DirectionInput::Vector(v) => {
            unit(*v)?;
        }
        DirectionInput::Datum { datum } => {
            if !valid_id(datum) {
                return Err(CadError::new(
                    "DATUM_REFERENCE",
                    "invalid direction datum name",
                ));
            }
        }
    }
    Ok(())
}
pub(crate) fn direction_vector(c: &Context, d: &DirectionInput) -> Result<Vec3> {
    match d {
        DirectionInput::Vector(v) => unit(*v),
        DirectionInput::Datum { datum } => {
            let d = c
                .manifest
                .body
                .datums
                .get(&c.definition)
                .and_then(|ds| ds.get(datum))
                .ok_or_else(|| CadError::new("DATUM_REFERENCE", datum))?;
            let v = match d.kind.as_str() {
                "axis" | "direction" => &d.geometry["direction"],
                "plane" => &d.geometry["normal"],
                "frame" => &d.geometry["axes"][2],
                _ => {
                    return Err(CadError::new(
                        "DATUM_KIND_MISMATCH",
                        "direction predicate requires an axis, direction, plane or frame",
                    ))
                }
            };
            let v: [f64; 3] = serde_json::from_value(v.clone())
                .map_err(|_| CadError::new("DATUM_GEOMETRY", "invalid stored direction"))?;
            Ok(c.pose.vector(unit(v)?))
        }
    }
}
pub(crate) fn direction_dependencies(s: &Selection) -> Vec<&str> {
    fn dir<'a>(d: &'a DirectionInput, out: &mut Vec<&'a str>) {
        if let DirectionInput::Datum { datum } = d {
            out.push(datum);
        }
    }
    fn filter<'a>(f: &'a Filter, out: &mut Vec<&'a str>) {
        if let Some(d) = &f.planar_coplanar_with {
            out.push(&d.datum);
        }
        for d in [&f.planar_normal_parallel_to, &f.planar_normal_aligned_with]
            .into_iter()
            .flatten()
        {
            dir(d, out);
        }
        for f in f.all.iter().chain(&f.any).chain(f.not.iter().map(|x| &**x)) {
            filter(f, out);
        }
    }
    fn visit<'a>(s: &'a Selection, out: &mut Vec<&'a str>) {
        match s {
            Selection::Entities { .. } | Selection::Refs { .. } => {}
            Selection::Filter { input, r#where } => {
                visit(input, out);
                filter(r#where, out);
            }
            Selection::Rank { input, by, .. } => {
                visit(input, out);
                if let RankBy::CenterProjection { direction } = by {
                    dir(direction, out);
                }
            }
            Selection::Children { input, .. }
            | Selection::Ancestors { input, .. }
            | Selection::Siblings { input, .. }
            | Selection::Expect { input, .. } => visit(input, out),
            Selection::Union { inputs } | Selection::Intersection { inputs } => {
                for s in inputs {
                    visit(s, out);
                }
            }
            Selection::Difference { left, right } => {
                visit(left, out);
                visit(right, out);
            }
        }
    }
    let mut out = vec![];
    visit(s, &mut out);
    out.sort();
    out.dedup();
    out
}
