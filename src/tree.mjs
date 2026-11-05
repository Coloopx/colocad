// Parametric feature order is the order of edits on a single solid.
// Stable IDs refer to the operation, not to the most recent output body.
function fail(message) {
  throw new Error(message);
}
function object(v) {
  return v !== null && typeof v === "object" && !Array.isArray(v);
}
function fields(value, allowed, where) {
  if (!object(value) || Object.keys(value).some((k) => !allowed.includes(k)))
    fail("Unexpected fields in " + where);
}
function id(value) {
  if (typeof value !== "string" || !/^[A-Za-z][A-Za-z0-9_]{0,47}$/.test(value))
    fail("Invalid identifier");
  return value;
}
function number(value) {
  if (!Number.isFinite(value) || Math.abs(value) > 100000)
    fail("Expected a finite millimetre value within ±100000");
  return value;
}
function positive(n) {
  if (n <= 0) fail("Expected a positive value");
  return n;
}
function count(v, limit) {
  if (!Number.isInteger(v) || v < 1 || v > limit) fail("Invalid count");
  return v;
}

export function compileTree(tree) {
  fields(tree, ["format", "units", "parameters", "features"], "document");
  if (
    tree.format !== "colocad/1" ||
    tree.units !== "mm" ||
    !object(tree.parameters) ||
    !Array.isArray(tree.features) ||
    !tree.features.length ||
    tree.features.length > 128
  )
    fail("Expected colocad/1 feature tree in mm");
  for (const [key, v] of Object.entries(tree.parameters)) {
    id(key);
    number(v);
  }
  const dim = (v) => {
    if (typeof v === "number") return number(v);
    fields(v, ["param"], "dimension");
    if (!Object.hasOwn(tree.parameters, id(v.param)))
      fail("Unknown parameter " + v.param);
    return tree.parameters[v.param];
  };
  const point = (v, n) => {
    if (!Array.isArray(v) || v.length !== n) fail("Invalid coordinate");
    return v.map(dim);
  };
  const profile = (p) => {
    if (p?.kind === "circle") {
      fields(p, ["kind", "center", "radius"], "circle");
      return {
        kind: "circle",
        center: point(p.center, 2),
        radius: positive(dim(p.radius)),
      };
    }
    if (p?.kind === "rectangle") {
      fields(p, ["kind", "center", "size"], "rectangle");
      return {
        kind: "rectangle",
        center: point(p.center, 2),
        size: point(p.size, 2).map(positive),
      };
    }
    if (p?.kind === "polygon") {
      fields(p, ["kind", "points"], "polygon");
      if (
        !Array.isArray(p.points) ||
        p.points.length < 3 ||
        p.points.length > 128
      )
        fail("Polygon needs 3–128 vertices");
      return { kind: "polygon", points: p.points.map((q) => point(q, 2)) };
    }
    fail("Unsupported sketch profile");
  };
  const plan = [],
    seeds = new Map(),
    ids = new Set();
  for (const raw of tree.features) {
    if (!object(raw)) fail("Invalid feature");
    const name = id(raw.id),
      op = raw.op;
    if (ids.has(name.toLowerCase())) fail("Duplicate feature " + name);
    if (!plan.length && op !== "extrude")
      fail("First feature must create material");
    let f = { id: name, op };
    if (op === "extrude" || op === "cut") {
      fields(raw, ["id", "op", "profile", "startZ", "depth"], name);
      f = {
        ...f,
        profile: profile(raw.profile),
        startZ: dim(raw.startZ ?? 0),
        depth: positive(dim(raw.depth)),
      };
      seeds.set(name, f);
    } else if (op === "fillet") {
      fields(raw, ["id", "op", "radius", "edges"], name);
      fields(raw.edges, ["z", "count"], "edge selection");
      f = {
        ...f,
        radius: positive(dim(raw.radius)),
        edges: { z: dim(raw.edges.z), count: count(dim(raw.edges.count), 256) },
      };
    } else if (op === "linearPattern") {
      fields(raw, ["id", "op", "seed", "axis", "pitch", "count"], name);
      if (!seeds.has(raw.seed) || !["x", "y"].includes(raw.axis))
        fail("Pattern needs an earlier extrude/cut seed and X or Y direction");
      const n = count(dim(raw.count), 128);
      if (n < 2) fail("Pattern count includes seed and must be at least two");
      f = {
        ...f,
        seed: raw.seed,
        axis: raw.axis,
        pitch: positive(dim(raw.pitch)),
        count: n,
      };
    } else fail("Unsupported operation " + op);
    ids.add(name.toLowerCase());
    plan.push(f);
  }
  return plan;
}

// Resolve a feature into a world XY profile plus frame. Instance offsets move
// the seed sketch without moving its original material or an unrelated feature.
export function kernelModel(tree) {
  const plan = compileTree(tree),
    features = [],
    seeds = new Map();
  let result;
  function prism(f, offset = [0, 0]) {
    const p = f.profile,
      c = p.center ?? [0, 0];
    const profile =
      p.kind === "polygon"
        ? { kind: "polygon", points: p.points }
        : p.kind === "circle"
          ? { kind: "circle", radius: p.radius }
          : { kind: "rectangle", width: p.size[0], depth: p.size[1] };
    const frame = {
      origin: [c[0] + offset[0], c[1] + offset[1], f.startZ],
      x_axis: [1, 0, 0],
      y_axis: [0, 1, 0],
    };
    const tool = "F" + features.length;
    features.push({ id: tool, op: "extrude", profile, frame, height: f.depth });
    if (result) {
      const next = "F" + features.length;
      features.push({
        id: next,
        op: f.op === "cut" ? "cut" : "union",
        left: result,
        right: tool,
      });
      result = next;
    } else result = tool;
  }
  for (const f of plan) {
    if (f.op === "extrude" || f.op === "cut") {
      prism(f);
      seeds.set(f.id, f);
    } else if (f.op === "fillet") {
      const next = "F" + features.length;
      features.push({
        id: next,
        op: "fillet",
        input: result,
        radius: f.radius,
        select: {
          kind: "on_plane",
          axis: "z",
          value: f.edges.z,
          tolerance: 0.001,
          expected_count: f.edges.count,
        },
      });
      result = next;
    } else {
      for (let i = 1; i < f.count; i++)
        prism(
          seeds.get(f.seed),
          f.axis === "x" ? [i * f.pitch, 0] : [0, i * f.pitch],
        );
    }
    if (features.length > 1024) fail("Expanded feature limit exceeded");
  }
  return { version: 1, units: "mm", parameters: {}, features, output: result };
}
