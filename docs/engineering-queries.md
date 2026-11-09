# Exact queries and geometry-backed assembly

**Development source capability; not included in the published alpha.6 native bundles.** Build the native executable and SDK from the same checkout. [中文](engineering-queries.zh-CN.md) · [Runnable example](../examples/engineering-queries/README.md) · [Detailed contract](../specs/0004-engineering-query-and-assembly-interfaces.md).

Use this layer when a part's installation geometry must be measured or selected before assembly. The local Rust executable evaluates exact B-rep geometry; the SDK, CLI and agent tools share that implementation. No Python geometry workflow or separate service is required. Agent, harness and executable run in the same sandbox.

## Build once, query the accepted revision

```js
import { CadClient } from '@coloopx/colocad/kernel';
const cad = new CadClient({ outputRoot: './jobs' });
const built = await cad.build(model, { queryable: true });
if (!built.ok) throw Error(built.error.message);
const request = {
  schema: 'colocad.query/1',
  query_revision_id: built.data.query_revision_id,
  target: { definition_id: 'part' }, // a final feature ID in this model
  space: 'definition',
  tolerance: { linear_mm: 1e-5, angular_rad: 1e-7 },
  queries: [{
    id: 'top', op: 'faces',
    where: { planar_normal_aligned_with: [0, 0, 1] },
    limit: 64
  }]
};
const result = await cad.query(built.data.artifact_directory, request);
```

A queryable build saves `query-manifest.json` plus exact `query-definition-*.smb` payloads. Reopen them in a fresh process or move the entire artifact directory: references retain identity. Queries do not replay features, read STEP, export STEP, heal geometry or generate preview meshes. The receipt reports definition loading, indexing and query time separately. Ordinary builds remain opt-in; a model declaring `datums` or `interfaces` automatically requires these artifacts. Explicitly disabling them with such a declaration fails.

Use `target: {instance_path: ['subassembly', 'instance']}` with `space: 'assembly'` for world-space measurements. Positions include the solved rigid translation; directions do not. Queries use normal solved placements, independently of exploded previews. Lengths and areas are invariant. The SDK confines paths to host-configured `queryRoots` (default `outputRoot`); agents cannot change that configuration through query JSON.

The CLI accepts the same request file:

```sh
colocad record model.json session --queryable --label 'Queryable part'
# Use artifact_directory and query_revision_id from the build receipt.
colocad query ARTIFACT_DIRECTORY request.json --out receipt.json
colocad view session
```

`--out` creates a new file and refuses to overwrite one. For a `colocad/1` tree, `colocad build tree.json NEW_DIR --queryable` also enables queries. `ModelingSession.query(request)` records the observation and source revision without changing accepted geometry. A query failure is retained in history. In `createSessionTools`, `cad_query` is bound to the latest accepted artifact; raw `createCadTools` takes an allowed artifact directory explicitly. With a queryable preview, the Query tab highlights native face/edge/loop outlines and datum/group markers. The build saves hash-bound `query-display.json`; these sampled outlines are display aids, never measurement authority. Exploding the viewer moves overlays with their parts without changing query coordinates.

## Select intent, preserve ambiguity

Selections are bounded expressions, not executable code. Available operations are `entities`, `refs`, `filter`, `children`, `ancestors`, `siblings`, `union`, `intersection`, `difference`, `rank` and `expect`. Filters provide typed analytic classes, names, area/length/radius ranges, planar normal tests, loop roles and bounded `all`/`any`/`not`.

For a hole center on one installation face, select that face, then its inner loops, their circular edges, and derive point datums. Selecting all circular edges on the body would also collect the opposite openings. `siblings` requires an explicit shared child type: faces connected through an edge use `{kind:'siblings', input: selectedFace, entity:'face', via:'edge'}`. It does not return every face in the body.

`rank` returns the entire tied rank. An `expect` asserting one result fails on a tie, even when the page limit is one. Exact identity deduplicates entity sets; ordered loop edge uses preserve repeated seam uses. Cursors bind the revision, target, space, tolerance and operation. Snapshot `TopologyRef` values are observations, not permanent edit recipes; an edit invalidates old references.

Limits: 64 queries per batch, 256 results per page, expression depth 8 / 128 nodes, 200,000 adjacency visits, 2 MiB input, 8 MiB output and 64 MiB total exact snapshot payload. These are admission limits, not proof of a 512 MiB runtime envelope. Source accuracy remains unknown when the source does not specify it.

## Name geometry for mates

A declaration belongs to a final part definition, independently of its assembly occurrences:

```json
{
  "id": "seat",
  "definition": "part",
  "tolerance": { "linear_mm": 0.00001, "angular_rad": 0.0000001 },
  "derive": {
    "kind": "selected",
    "selection": {
      "kind": "filter",
      "input": { "kind": "entities", "entity": "face" },
      "where": { "planar_normal_aligned_with": [0, 0, 1] }
    },
    "derive": { "kind": "plane" }
  }
}
```

Put declarations in `Model.datums`. IDs are unique within a definition. A selected primitive requires exactly one entity. `selected` supports vertex/circle points, analytic planes, cylinder/circle axes and straight-edge directions. Other recipes are `design_point`, ordered `point_difference`, and `frame`. A frame names an origin point and two nonparallel orthogonal directions (`x` and `z`, each with explicit positive/negative sense); it is right-handed. Its dependencies must occur earlier in the same definition. Persistent selections cannot contain prior-snapshot refs. Derived versus declared design intent remains visible in receipts.

Use `{instance:'part_occurrence', datum:'seat'}` instead of a numeric attachment frame. Plane mates consume a plane, concentric mates an axis, coincidence/distance a point, and fixed/prismatic/revolute mates require a full frame. A full frame can also provide these lower-dimensional attachments. `frame` and `datum` are mutually exclusive; omitted legacy attachment data still uses the original identity frame.

An internal basis completes a plane/axis only for numerical evaluation. It adds no physical orientation constraint: an axis plus an axial plane leaves rotation about that axis free. A cylinder axis alone cannot establish the axial origin required by a revolute joint. Declare the required point/frame or combine axis and plane constraints explicitly.

Read accepted named datums using `{id:'mount', op:'named_datums', ids:['seat']}`. Their source refs bind to the final query revision and transform with the chosen occurrence. Rebuilding after parameter edits reruns recipes. Missing, ambiguous, degenerate and incompatible datum kinds fail with structured errors; they do not silently select the first candidate.

## Holes, groups and installation interfaces

`holes` recognizes connected analytic cylindrical bores with planar entrances/ends in consistently oriented closed solids. It distinguishes through, blind and counterbored holes, joins supported axial/angular split patches, and excludes external shafts. A counterbore is one physical hole with multiple segments. Conical tips, intersecting bores, side-open recesses and spline cylinders return partial coverage with source references; they are not silently called complete holes.

`hole_groups` accepts optional `support_faces` refs and `where: {pattern, member_count, equal_diameter}`. Patterns are linear, rectangular, circular or unstructured. It returns physical centers, axes, dimensions, residuals, competing subsets, excluded openings and symmetry. Plane-scoped coverage excludes uncertainty away from that support's topology, but retains its diagnostic. Recognition is bounded to 64 openings per support and 10,000 hypotheses; large searches fail `QUERY_LIMIT`.

`interfaces` observes named declarations (`ids`) or recognizes candidates (`profile: 'planar_hole_mount/v1'`, optional `support_faces` and group filter). Candidates do not acquire a unique engineering orientation from symmetric holes. Put persistent intent in `Model.interfaces`: uniquely select a plane and hole group, optionally a coaxial register, specify the group-center/axis-intersection/design-point origin, verification tolerance, pattern dimensions, bore sizes and termination. Add an explicit design direction or geometry-backed orientation datum when an oriented mate needs one. The declaration is reevaluated against the built geometry; unsupported, missing, multiple or incompatible results fail.

Use `{instance, interface, orientation?}` in a mate. Full-frame mates require an orientation; lower-dimensional mates retain unresolved freedoms. `interface_compatibility` checks all-hole correspondence, plane relation/gap, axis offsets, each side's diameter range and optional register radial clearance. It records ambiguity and actual paired refs. Independent diameter ranges do not prove bolt or thread fit.

The [complete external assembly example](../examples/engineering-interfaces/README.md) shows the actual typed declaration, replacement and error workflow. Relative filters accept `planar_normal_aligned_with: {datum}`, `planar_normal_parallel_to: {datum}`, and `planar_coplanar_with: {datum}`. The last requires a plane and uses both normal and offset, avoiding a blind-hole bottom with the same normal. Projection ranks also accept a named datum direction. Dependencies must be earlier in the definition; ordered circle-center differences follow a rigidly rotated source instead of relying on its global axes.

## Exact relationships and acceptance checks

A read-only query can request:

```js
{ id: 'gap', op: 'measure',
  a: { instance_path: ['shaft'] }, b: { instance_path: ['support'] } }
```

The snapshot target and both measurement targets remain explicit. Definition space measures local shapes; assembly space uses solved occurrence poses. The result distinguishes separated, touching, overlapping, contained and indeterminate, with unsigned boundary distance, overlap volume, witness points and numerical tolerances. Containment can have a positive boundary gap; distance is not penetration depth. `items[0]` is also exposed as `measurement`. Indeterminate is a partial result; required assembly checks fail. Witness topology refs are marked unavailable where the copy-based solver cannot preserve their mapping.

`checks: [{kind:'geometric_relationship', a:{instance}, b:{instance}, tolerance, max_overlap_mm3, min_clearance_mm?, allowed_contacts?, pose_samples?}]` makes a required claim. Minimum clearance requires separation. Explicit pose deltas (at most 32) are evaluated individually and retained even on failure; this does not establish continuous-motion clearance. Required exact checks reject degraded feature ancestry and reduced representations.

Allowed contacts currently certify selected opposing planar faces on polyhedral solids by checking both complete solids against their support half-spaces. A single closest witness is insufficient. Other overlap is never exempted. Curved contact certification and region-restricted measurements fail explicitly as unsupported. Ordinary whole-part measurements support curved solids.

## Evidence and boundaries

The foreign flange workflow passes base, different-housing and arbitrarily rotated replacement inputs while rejecting wrong pitch and missing holes. The complete KK60 reconstruction with an independently authored bracket passes 82-solid OCCT readback and nine explicit contact/gap samples. These cases, tolerances and provenance are retained in the [experiment ledger](../specs/experiments/0004/README.md); they are not a general industrial success rate.

Descriptors include native UV/periodicity, ordered pcurves, seam uses, adjacency and tolerances. Trimmed definition bounds are transformed AABBs and may be conservative after rotation; `bounds_semantics` identifies this. Query results do not certify machining tolerances, threads, load capacity or global uniqueness of a local mate solution. Broader reliability and target sandbox acceptance are governed by [the ordered goals](../specs/0005-capability-reliability-and-optimization.md).

Development checkpoints carry `execution_policy`, a deterministic fingerprint of the executor Rust sources, manifest/lockfile, compiler and target. A missing or different policy forces replay, even when the version string matches. This prevents a development upgrade from restoring old feature semantics. The [G3 evidence](../specs/experiments/0005/README.md) covers actual process interruption, reopen/relocation, semantic split/merge/delete, replacements and Linux resource limits.
