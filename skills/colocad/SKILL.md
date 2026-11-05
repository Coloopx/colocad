---
name: colocad
description: Build and verify editable mechanical parts and assemblies from dimensions, engineering drawings or multiple views with @coloopx/colocad. Use for quick parametric construction, fit checks, STEP and previews.
---

Use the project's installed `@coloopx/colocad` on Node 22+. The agent and harness use the SDK and native executable inside the same sandbox.

For a dimensioned part, start with the cross-section or stock that controls its shape. Use the [quick-build guide](references/quick-build.md) for single-feature stock (block, pin, ball, taper) and common structures (hole grid, sleeve, L-section, open housing). Each model has editable dimensions. The guide gives their datums, measurements to check, and an example of reusing a definition in an assembly. Pick only the features the drawing calls for. Guide dimensions are examples.

For drawings, identify the views, units, scale and common datum. Separate measured dimensions from hidden or assumed details. Compare projections from the final STEP against the supplied views at the same scale. Flag dimensions that prevent a critical fit from being checked.

Choose the entry point that matches the model:

- A kernel model (`version: 1`, units `mm`, `parameters`, `features`, and either `output` or named `assembly`) supports arbitrary sketch planes, single solids and placements. Run `node <skill-dir>/scripts/build.mjs MODEL.json NEW_DIR [--set name=value] [--preview]` in the project. Use the returned `directory` to find the effective `model.json`, exact `model.step` and `report.json`. The installed package exports the full schema as `@coloopx/colocad/schema`.
- For a `colocad/1` XY feature tree use the project's `node_modules/.bin/colocad plan TREE.json` and `node_modules/.bin/colocad build TREE.json NEW_DIR --preview`. Cuts start at `startZ` and travel along +Z. Linear pattern counts include their seed. This entry point also produces the feature tree and native job in `NEW_DIR/part/`.

Return the editable model and STEP. Reopen the STEP and check the dimensions that distinguish success for this task, such as a bore's diameter and axis or a cut reaching the cavity. For a long construction, keep inspectable milestones and a fixed camera. An assembly preview establishes appearance. A fit or motion claim needs the corresponding measurement.
