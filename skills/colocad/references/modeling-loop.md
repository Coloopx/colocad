# Whole-product modeling loop

Read for assemblies, drawing reconstruction or edits whose dependencies matter.

## From product evidence to an executable model

1. Establish the assembly envelope, primary axes and interface datums. Identify unique part definitions and repeated occurrences. Attach an acceptance check to the interfaces that matter to the request.
2. Select the controlling cross-section or stock for each unique part. Use extrude/revolve for prismatic/turned structure; cut real bores and cavities; use loft/sweep when geometry requires it. A rotated or copied solid is an assembly occurrence, not a new manufactured part definition.
3. Preserve relationships in parameters/expressions. Examples: a centered hole pattern follows width edits; the nut's translation and screw rotation share the lead relation. Assign meaningful, stable feature/part IDs. Do not rely on an incidental edge number surviving a Boolean or parameter edit.
4. Validate the JSON and declared budgets, then build a meaningful milestone. Read `ok`, geometry, timings and degradations; inspect a projection or section aimed at the changed feature. A small subassembly can resolve an interface before an expensive complete build.
5. Compare a quantitative result with the target. Correct the interpretation, feature or parameter that caused the error. Preserve the last accepted model and the failed attempt; changing unrelated dimensions obscures the cause.
6. Assemble and check the whole selected configuration. Reopen STEP independently for significant delivery. Test at least the edits or poses required by the request, keeping paired interfaces consistent. Persist the session and launch `view/replay/render` from the CLI for human review.

This is an adaptive loop, not a mandatory approval gate between every feature. Use fewer milestones for a simple part. For a long product, keep checkpoints that explain both shape construction and assembly.

## Geometry-driven assembly with exact queries

Use this path when installation geometry must be measured or a part replaced. Check installed capabilities; alpha.7 provides `cad_query`.

Exact grid snapshots require alpha.8 or later: alpha.7 had a nested seam UV-line serialization defect. Upgrade the package and matching native if `BSplineCurve::make_nurb` status 1001 appears on this path; changing preview precision does not repair it. Use the normal grid/query workflow after upgrading.

- Build exact query artifacts with `queryable`, or declare `Model.datums`/`Model.interfaces`, which enable them. Keep the accepted `query_revision_id`, target definition/occurrence, coordinate space and tolerance with the observation.
- Narrow the mounting plane before its openings and group. Inspect complete/partial coverage, cardinality and symmetric alternatives. A normal-only selection can include a blind-hole bottom; a position-sensitive recipe can use an earlier named plane with `planar_coplanar_with`.
- Persist supported semantic recipes in datums/interfaces, not snapshot refs or face indices. A symmetric group needs design intent or a measured orientation datum for an oriented mate. Let named attachments drive the existing mate solver.
- Require all-hole compatibility and relevant exact pair checks. Boundary distance is unsigned; inspect relationship and overlap before calling it clearance. Unsupported/partial evidence cannot certify fit or absence.
- After an edit/replacement, re-query the new revision and rerun required checks. Missing, ambiguous or incompatible geometry is a retained failure. Never rewrite an old ref's revision to make it appear current. Resume from accepted source after failure/interruption.

For a replacement workflow, challenge the same recipe with compatible and incompatible sources. Do not add a new part template or hardcoded face mapping for each input. Use only the pose/pair checks required by the task; report their discrete scope.

## Inspect and edit from a shell

When `colocad --help` lists `inspect`, use this sequence for a mounting plate. Replace `MODEL` with the bundled `skills/colocad/assets/plate-grid.json` or your own source.

```sh
colocad record MODEL ./plate --queryable
colocad head ./plate
colocad inspect ./plate holes.json
colocad edit ./plate parameters.json
colocad head ./plate
```

For the bundled plate, use the following request in `holes.json`. Set `query_revision_id` to the value returned by the first head. Questions about the same definition, coordinate space and tolerance can share one request and one snapshot load.

```json
{
  "schema": "colocad.query/1",
  "query_revision_id": "REVISION_FROM_HEAD",
  "target": {"definition_id": "mounts"},
  "space": "definition",
  "tolerance": {"linear_mm": 0.00001, "angular_rad": 0.0000001},
  "queries": [
    {"id":"bores","op":"holes","where":{"termination":"through"}},
    {"id":"top","op":"faces","where":{"planar_normal_aligned_with":[0,0,1]},"limit":16}
  ]
}
```

With `{"length":100,"xCount":3,"xPitch":32,"holeDiameter":8}` in `parameters.json`, the expected result changes from four 6 mm bores to six 8 mm bores. Issue the semantic request against the new head and check status, item count and segment diameters. Resolve any topology refs again. `inspect` records each request and receipt. The direct `colocad query REVISION_DIR REQUEST.json` command also works on a detached revision. For a long session, use `head` to resume and `history SESSION --after N --limit K` to inspect a range.

## Match evidence to the claim

| Claim | A useful check |
| --- | --- |
| Outer size | Exact STEP/B-rep bounding dimensions against the drawing |
| Hole/cavity | Axis, radius/depth and material probes inside/outside; a rendered circle is insufficient |
| Assembly fit | Common axes and exact interference/clearance for the relevant pairs |
| Motion | Rebuilt required poses and the coupling relation; an exploded-view animation is not motion validation |
| Editable design intent | Perturb a meaningful parameter and recheck dependent geometry/interfaces |
| Manufacturing-ready | Required dimensions, tolerances, materials, processes and functional validation; do not infer these from a catalog render |

Negative material checks matter: verify that a supposed opening is empty and adjacent stock remains. A global volume can match despite a misplaced hole. Avoid probing exactly on an edge or boundary when testing inside/outside classification.

## State that survives a long session

Keep source IDs and coordinate conventions, accepted model path/hash, exposed parameters, semantic datum/interface recipes, latest receipt and `query_revision_id`, target/space/tolerance, coverage/ambiguity, passed checks, unresolved assumptions, and the next targeted test. Reload the current accepted source after a failure or context reset. In `ModelingSession`, recorded `cad_edit` takes only `{parameters}` and uses that accepted source.

The KK60 example illustrates this process. Enlarging the manufacturer's section corrected the screw axis to 23 mm; the real helical B-rep exposed an internal STEP budget defect; independent material probes and rail/carriage checks established specific geometry claims. Its undisclosed nut recirculation remains unmodeled. Do not reuse its assumed internal dimensions as a universal actuator design.
