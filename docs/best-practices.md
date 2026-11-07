# Modeling and integration best practices

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
| Coarse clearance exploration | Use an explicit envelope/simplification, then check the final exact model |
| New exact geometry | Build a new revision and inspect the affected region |

Do not rebuild exact B-rep merely to orbit the view. Conversely, do not use a display mesh to prove hole depth, watertight solid validity or manufacturing tolerances. Export the exact STEP for independent checking.

## Give the agent a small, useful tool surface

For routine parts, capabilities, validate, build, preview and edit are enough to start. Add STEP inspection and sketch solving when the task requires them. Supply the skill for workflow guidance and assets; keep the full schema accessible instead of pasting it into every system prompt.

Keep model-facing results concise: success/failure, the failing feature if any, solid count, dimensions/volume, validation summary, artifact paths and a preview image. Store the detailed feature trace and timings for diagnostics. Never send a full STEP, mesh JSON or base64 PNG as ordinary text in the conversation when the harness supports image/file attachments.

Use a bounded correction loop: locate the failed feature, inspect its inputs and dependencies, change the cause, rebuild, then recheck earlier requirements. Preserve the previous accepted revision until the new one passes. Do not silently drop a failing cut to make a model appear complete.

## Acceptance beyond a screenshot

Check solid count, envelope, material volume and critical internal regions. For assemblies, check named occurrences, placements, repeated-part consistency and required clearances. A visually plausible rendering can hide fused fingers, solid-filled holes or inverted placements.

Keep a reproducible record: input model, relevant source views and assumptions, package/native version, full receipt, exact STEP and chosen preview settings. For high-value geometry, reopen the STEP using an independent kernel. [Existing OCCT checks](../test/validation/geometry.py) illustrate volume and material probes; they are not a universal verifier for arbitrary parts.

## Measure the workflow you actually deploy

Separate fresh-process time to STEP, resident-session edits, preview-only views and full agent turnaround. Record whether startup/import, STEP export, independent validation and inference are included. Measure memory for the sandbox/process tree, not just the parent Node process. Compare matched accepted geometry, not only successful exit codes.

The [100-second hand study](../showcase/tendon-hand/window/README.md) compares a fixed revision queue and explicitly excludes live-model inference. Its throughput and memory gains are workload-specific. A sequential primitive benchmark does not establish that all booleans or all complex assemblies are faster.
