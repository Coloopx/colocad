---
name: colocad
description: Build and verify parametric mechanical parts or assemblies with @coloopx/colocad from dimensions, engineering drawings or multiple views; export local STEP and previews.
---

Use the installed project dependency `@coloopx/colocad` with Node 22+. The agent, harness, CLI and Rust executable share one filesystem. Resolve `node_modules/.bin/colocad` from the project; avoid an implicit npm download in an offline sandbox.

For a drawing, identify the view directions, units, scale and shared datum first. Record what you can measure separately from occluded or assumed features. If only a scale bar is present, retain that uncertainty. Never take the input silhouettes from the output being evaluated.

**One XY feature tree:** write a `colocad/1` JSON document. `colocad plan tree.json` checks the feature sequence; `colocad build tree.json NEW_DIR --preview` writes `NEW_DIR/part/model.step`, the source tree, report and preview. Circles use radius, lengths are mm, cuts run toward +Z from startZ; a linear pattern count includes its seed. See the installed package’s `src/index.d.mts` for the full surface.

**An assembly, arbitrary sketch planes or placed bodies:** build a kernel model with `version:1, units:"mm", parameters, features, assembly`. Its schema is exported as `@coloopx/colocad/schema`. Reuse a part definition by ID; name each instance and its pose. Use either an assembly list or a single output. From the project directory run `node <installed-skill>/scripts/assembly.mjs model.json NEW_DIR --preview`. This thin CLI uses the project's installed `CadClient`; the same JSON can be passed directly to the SDK. A preview or a line representing a tendon is not evidence of kinematics, load capacity or manufacturability.

Build inspectable milestones (e.g. blank, drilled part, placed subassembly), each in a new directory. Keep the camera and scale fixed for progress frames. Check the final STEP by reopening it: named parts or solids, envelope, critical holes and clearances. For multi-view reconstruction compare projections at the input camera poses; report deviations and which internal details remain assumptions. Return the editable model and exact STEP alongside the preview.
