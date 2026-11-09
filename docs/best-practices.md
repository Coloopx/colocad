# Modeling and integration best practices

Applies to the supported Colocad alpha.7 APIs. [中文](best-practices.zh-CN.md) · [Complete agent handbook](agent-modeling-handbook.md) · [Query contract and limits](engineering-queries.md).


## Start from the structure that controls the part

Use a parameterized asset when it matches the requested geometry. A plate with a hole grid should begin with stock dimensions and grid parameters; a sleeve should begin with outer diameter, bore and length. The [quick-build guide](../skills/colocad/references/quick-build.md) gives datums and checks for eight structures. Adapt the feature program when the part differs; an asset is a starting point, not a replacement for reading the drawing.

For an unfamiliar part, write a short dimension table: overall envelope, datum, wall thickness, hole diameters/pitches/depths, symmetry and interfaces. Separate measured dimensions from assumptions. If four views disagree, identify the conflict before treating one as ground truth. Occluded bores and tendon routing need explicit assumptions or sectional evidence.

Use stable named parameters for interface dimensions. Preserve feature IDs across revisions. Build stock and main cuts first, inspect them, then add small rounds and secondary details. A failed fillet is easier to diagnose on a correct base shape.

## Choose the cheapest operation that answers the question

| Change | Action |
| --- | --- |
| Camera or image size only | Reuse `preview()` on the same client, or `observeArtifact()` on a saved mesh |
| Parameter dimension | `edit()` against the latest accepted source; save the new source |
| Repeated identical components | Define the part once; place named assembly occurrences |
| Repeated face bores | Use a hole-grid/pattern operation where it represents the design |
| Measure an accepted part or installation interface | `query()` its exact snapshot with the current revision, explicit target, space and tolerance |
| Replace a compatible imported component | Re-evaluate semantic datum/interface declarations, solve mates and rerun required fit/clearance checks |
| Coarse clearance exploration | Use an explicit envelope/simplification, then check the final exact model |
| New exact geometry | Build a new revision and inspect the affected region |

Do not rebuild exact B-rep merely to orbit the view. Conversely, do not use a display mesh to prove hole depth, watertight solid validity or manufacturing tolerances. Export the exact STEP for independent checking.

## Give the agent a small, useful tool surface

For routine parts, capabilities, validate, build, preview and edit are enough to start. Add capability-gated `cad_query` for measured geometry and assembly work; add STEP inspection or sketch solving when needed. Supply the skill for workflow guidance and assets; keep the full schema accessible instead of pasting it into every system prompt.

Keep model-facing results concise: success/failure, failing feature/query ID, `query_revision_id`, relevant geometry, coverage/ambiguity, validation, paths and requested images. Store full receipts and timings on disk. A query result may be partial even when the call succeeds; retain its coverage and diagnostics. Never send a full STEP, mesh JSON or base64 PNG as ordinary text when the harness supports attachments.

Use a bounded correction loop: locate the failed feature, inspect its inputs and dependencies, change the cause, rebuild, then recheck earlier requirements. Preserve the previous accepted revision until the new one passes. Do not silently drop a failing cut to make a model appear complete.

## Query, declare intent, assemble, then challenge the edit

1. **Build once with exact query artifacts.** Use `{queryable:true}` when observations are needed. Declaring `Model.datums` or `Model.interfaces` enables them automatically. Record `artifact_directory` and `query_revision_id`; use `definition_id` for a part's local coordinates or `instance_path` for an assembly occurrence.
2. **Narrow by engineering intent.** Select the mounting plane, then its openings and hole group. A normal test alone can also match a blind-hole bottom; use a named plane with `planar_coplanar_with` when position matters. Assert cardinality and inspect partial coverage, extra holes and competing candidates.
3. **Persist a recipe.** Put supported geometric selection recipes in `Model.datums`/`Model.interfaces`. Refs from a query are observations of that revision; do not put them into a persistent recipe or use incidental face indices. Symmetric groups need an explicit or measured orientation datum for an oriented mate.
4. **Check the solved assembly.** Use named attachments, all-hole `interface_compatibility`, and required `geometric_relationship` checks for the relevant pairs. A positive boundary distance alone does not prove clearance: a contained part can have one. Require the appropriate relationship, overlap and clearance results.
5. **Edit or replace, then re-query.** Rebuild from accepted source, obtain the new revision and rerun the recipe/checks. Keep missing/ambiguous/incompatible outcomes as failed attempts; do not patch in new face numbers to force success. Use a compatible and an incompatible replacement to challenge the same assembly recipe.
6. **Deliver the evidence.** Retain source, session, exact STEP, numerical checks and unknowns. Export human review through `colocad view/replay/render`. Discrete checked poses do not certify continuous travel, and a display explosion is not a motion check.

### Run the edit/query loop against the installed package

Install `@coloopx/colocad@0.1.0-alpha.7` in a new project. Save this as `query-workflow.mjs`; keep `query-session` and `query-review.html` unused before the first run:

**Alpha.7 known issue:** the bundled `plate-grid.json` uses `face_hole_grid`, which fails when building exact query artifacts on the tested macOS arm64 native (`BSplineCurve::make_nurb`, status 1001). Ordinary build/preview pass. This example expresses the same four-hole plate with `holes`; it does not claim the grid snapshot defect is fixed. [Reproducer and checks](evidence/alpha.7-query-guidance.json).

```js
import assert from 'node:assert/strict';
import {ModelingSession} from '@coloopx/colocad/session';

const model = {
  version:1, units:'mm', parameters:{holeDiameter:6},
  features:[
    {id:'plate', op:'box', origin:[0,0,0], size:[80,50,6]},
    {id:'mounts', op:'holes', input:'plate',
      frame:{origin:[0,0,-1], x_axis:[1,0,0], y_axis:[0,1,0]},
      positions:[[15,10],[65,10],[15,40],[65,40]],
      radius:{op:'div', a:{param:'holeDiameter'}, b:2}, depth:8}
  ],
  output:'mounts', preview:{width:320, height:240, chord_mm:1}
};
const session = await ModelingSession.create('./query-session');
const checked = r => { if (!r.ok) throw Error(JSON.stringify(r.error)); return r.data; };
const request = revision => ({
  schema:'colocad.query/1', query_revision_id:revision,
  target:{definition_id:model.output}, space:'definition',
  tolerance:{linear_mm:1e-5, angular_rad:1e-7},
  queries:[{id:'bores', op:'holes', where:{termination:'through'}}]
});
const observe = async (revision, diameter) => {
  const r = checked(await session.query(request(revision)));
  const result = r.results[0];
  assert.equal(result.status, 'complete');
  assert.equal(result.items.length, 4);
  for (const hole of result.items) {
    assert.ok(hole.segments.length > 0);
    for (const segment of hole.segments)
      assert.ok(Math.abs(segment.diameter_mm - diameter) < 1e-5);
  }
  assert.equal(r.execution.geometry_build_calls, 0);
  return result.items.map(h => h.axis.point_mm.slice(0,2))
    .sort((a,b) => a[0]-b[0] || a[1]-b[1]);
};
const initial = checked(await session.build(model, {queryable:true}));
const centers = await observe(initial.query_revision_id, 6);
const edited = checked(await session.edit({holeDiameter:8}));
const stale = await session.query(request(initial.query_revision_id));
assert.equal(stale.ok, false);
assert.equal(stale.error.code, 'QUERY_REVISION_MISMATCH');
const newCenters = await observe(edited.query_revision_id, 8);
centers.forEach((p,i) => p.forEach((x,j) =>
  assert.ok(Math.abs(x-newCenters[i][j]) < 1e-5)));
assert.ok(Math.abs(edited.geometry.volume_mm3 - (80*50-4*Math.PI*4**2)*6) < 1e-5);
console.log({session:session.root, revision:edited.query_revision_id,
  holes:4, diameter_mm:8, staleRefRejected:true});
```

```sh
node query-workflow.mjs
./node_modules/.bin/colocad replay ./query-session ./query-review.html
./node_modules/.bin/colocad view ./query-session
```

The old-revision error is intentional and remains in history. This verifies one dimensioned plate and edit; it is not a drawing-reconstruction study. For the complete foreign STEP → datums/interfaces → mates → exact checks → replacement workflow, use the [engineering-interface example](../examples/engineering-interfaces/README.md), including its three compatible and two incompatible sources and independent OCCT verifier.

## Acceptance beyond a screenshot

Check solid count, envelope, material volume and critical internal regions. For assemblies, check named occurrences, placements, repeated-part consistency and required clearances. A visually plausible rendering can hide fused fingers, solid-filled holes or inverted placements.

Keep a reproducible record: input model, relevant source views and assumptions, package/native version, full receipt, exact STEP and chosen preview settings. For high-value geometry, reopen the STEP using an independent kernel. [Existing OCCT checks](../test/validation/geometry.py) illustrate volume and material probes; they are not a universal verifier for arbitrary parts.

## Measure the workflow you actually deploy

Separate fresh-process time to STEP, resident-session edits, preview-only views and full agent turnaround. Record whether startup/import, STEP export, independent validation and inference are included. Measure memory for the sandbox/process tree, not just the parent Node process. Compare matched accepted geometry, not only successful exit codes.

The [100-second hand study](../showcase/tendon-hand/window/README.md) compares a fixed revision queue and explicitly excludes live-model inference. Its throughput and memory gains are workload-specific. A sequential primitive benchmark does not establish that all booleans or all complex assemblies are faster.

For query workloads, record snapshot creation/load and query time separately from build/STEP/preview time. Bound result pages and query scope before raising budgets. Cgroup peak includes descendants and charged file/cache pages; the [alpha.7 installation checks](evidence/release-alpha.7.json) include npm installation, while the [G3 runs](../specs/experiments/0005/README.md) measure different CAD/agent workloads. Keep their accounting separate. G4's benchmark/retrospective optimization campaign has not started.
