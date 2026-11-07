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

Keep source IDs and coordinate conventions, accepted model path/hash, exposed parameters, interface checks, latest receipt, unresolved assumptions, and the next targeted test. Reload the current accepted source after a failure or context reset. In `ModelingSession`, recorded `cad_edit` takes only `{parameters}` and uses that accepted source.

The KK60 example illustrates this process. Enlarging the manufacturer's section corrected the screw axis to 23 mm; the real helical B-rep exposed an internal STEP budget defect; independent material probes and rail/carriage checks established specific geometry claims. Its undisclosed nut recirculation remains unmodeled. Do not reuse its assumed internal dimensions as a universal actuator design.
