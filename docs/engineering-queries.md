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

A queryable build saves `query-manifest.json` plus exact `query-definition-*.smb` payloads. Reopen them in a fresh process or move the entire artifact directory: references retain identity. Queries do not replay features, read STEP, export STEP, heal geometry or generate preview meshes. The receipt reports definition loading, indexing and query time separately. Ordinary builds remain opt-in; a model declaring `datums` automatically requires these artifacts. Explicitly disabling them with such a declaration fails.

Use `target: {instance_path: ['subassembly', 'instance']}` with `space: 'assembly'` for world-space measurements. Positions include the solved rigid translation; directions do not. Queries use normal solved placements, independently of exploded previews. Lengths and areas are invariant. The SDK confines paths to host-configured `queryRoots` (default `outputRoot`); agents cannot change that configuration through query JSON.

The CLI accepts the same request file:

```sh
colocad record model.json session --queryable --label 'Queryable part'
# Use artifact_directory and query_revision_id from the build receipt.
colocad query ARTIFACT_DIRECTORY request.json --out receipt.json
colocad view session
```

`--out` creates a new file and refuses to overwrite one. For a `colocad/1` tree, `colocad build tree.json NEW_DIR --queryable` also enables queries. `ModelingSession.query(request)` records the observation and source revision without changing accepted geometry. A query failure is retained in history. In `createSessionTools`, `cad_query` is bound to the latest accepted artifact; raw `createCadTools` takes an allowed artifact directory explicitly. Query history is inspectable; topology highlight overlays are not implemented yet.

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

## Evidence and remaining boundaries

The example imports an independently generated OCCT flange, derives its bore axis and bottom plane, mates a shaft, exports STEP, and rejects an oversized shaft by exact collision checking. Independent OCCT readback validates both solids, analytic volumes and a 0.2 mm minimum gap. This is a generic analytic fixture, not a reconstructed commercial product or an industrial success-rate corpus.

Still pending: named-datum-relative filter predicates; all specialized periodic/singular descriptors; hole/counterbore recognition, hole grouping and engineering interfaces; general replacement acceptance; broad difficult-model reliability; and target Linux sandbox resource acceptance. Bounding boxes are rigidly transformed definition AABBs and can be conservative after rotation. Query results do not certify machining tolerances, threads, load capacity or global uniqueness of a local mate solution. [Ordered goals and the future optimization loop](../specs/0005-capability-reliability-and-optimization.md).
