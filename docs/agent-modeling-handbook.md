# Agent CAD handbook

A complete workflow for source preparation, modeling tools, agent guidance, validation and tuning. This describes the implemented Colocad alpha.5 interfaces and recommendations informed by actual use and primary sources. It is not a claim of universal optimal prompting or autonomous industrial reconstruction.

[完整中文指南](agent-modeling-handbook.zh-CN.md) · [Research and evidence](agent-modeling-research.md) · [Packaged skill](../skills/colocad/SKILL.md)

## Start with the product and its evidence

Identify one manufacturer/model/configuration, units, required interfaces and deliverables. The whole product is the target; BOMs, drawings, sections, photographs and reference CAD are inputs describing it. Retain page/view identifiers, revisions and checksums when reproducibility matters. Read the overall page before detailed crops so dimensions remain attached to their datums and extension lines.

Separate **specified**, **derived**, **assumed** and **unknown** dimensions. Record formulas for derived quantities and the claims affected by unknowns. An uncalibrated photograph does not establish exact dimensions; a numerical acceptance threshold is not a manufacturer tolerance. Continue independent work when a critical interface is unresolved, while retaining that unresolved claim.

The [materials reference](../skills/colocad/references/materials.md) and [filled product brief](../skills/colocad/assets/product-brief.example.json) make this handoff concrete. The brief is evidence metadata, not executable kernel JSON.

## The actual data flow

The agent reads text/images using its existing harness tools, decides dimensions and feature dependencies, and writes a generator or explicit model JSON. The local SDK/CLI invokes the Rust kernel; actual measurements, sections and independent STEP checks inform the next correction. Accepted and failed revisions persist in a session. Humans inspect it using `colocad view`, `replay` or `render`.

Colocad does not currently expose an automatic image-to-CAD endpoint. The KK60 case used PDF text extraction and rendered-page inspection, followed by an authored feature generator. Enlarging SECTION A-A corrected the screw-axis height to 23 mm. The final 16-stage history, 81-solid assembly and independent checks are [available as an example](../examples/hiwin-kk60/README.md), with proprietary internals explicitly approximated or omitted.

## Keep the tools understandable

Register the existing capabilities, validation, build, preview and edit tools first; enable sketch solving or STEP inspection when needed. Preserve schemas and `$defs`. `cad_validate` is input preflight, not a shape check. Plain `cad_edit` takes `{model, parameters}`; recorded editing takes `{parameters}` and uses the accepted source. The harness owns paths, budgets, permissions and artifact retention inside the same sandbox.

Return the failing feature/code, revision paths/hashes, relevant geometry and timing, cache information and **all** degradations. Keep complete traces and large meshes on disk. The [compact adapter](../examples/integrations/compact-tools.mjs) implements this without changing execution semantics. The [guided workflow](../examples/integrations/guided-workflow.mjs) performs actual builds, quantitative edits, failure recovery, observation and CLI HTML export:

```sh
node examples/integrations/guided-workflow.mjs ./out/guided-workflow
```

This is a scripted contract walkthrough, not live-model inference. [Tool contracts](../skills/colocad/references/tool-contracts.md) explain the complete boundary, including what is not an implemented tool.

## Model and verify meaningful milestones

Plan the assembly envelope and interfaces before individual details. Use controlling sections, extrusions/revolutions, real cuts, and loft/sweep only where needed. Link dependent dimensions and placements; reuse part definitions for repeated occurrences. Keep stable semantic IDs and local frames. [Quick structures](../skills/colocad/references/quick-build.md) cover stock, plate patterns, sleeves, brackets and housings.

For each meaningful change: validate, build, inspect its receipt and a relevant view, then compare a quantitative check with the target. Repair the specific interpretation/frame/feature that failed. Preserve failures and the accepted source. Do not remove required geometry to obtain a successful receipt. An explicit optional degradation weakens its associated claim.

| Claim | Evidence needed |
| --- | --- |
| Geometry is valid | Native checks and independent significant STEP readback |
| Dimensions match | Sourced interface measurements, not appearance alone |
| Bores/cavities exist | Axes/depths and material checks inside/outside |
| Design is editable | Actual parameter perturbation and dependent-interface checks |
| Assembly fits/moves | Required pose checks, relevant clearances/interferences and coupling relations |
| Manufacturing readiness | Required tolerances, materials, processes and corresponding functional validation |

A display explosion is not motion verification. One matching total volume cannot locate a misplaced hole. [Modeling-loop reference](../skills/colocad/references/modeling-loop.md).

## Guide the agent without overloading it

The skill entry point routes to materials, modeling loop, tool contracts, prompts, tuning and history. Load only what the current task needs. Give the model a concise task contract and the relevant source views, accepted state and checks. Ask for inspectable dimensions, decisions and artifacts rather than a hidden chain of thought.

[Prompt patterns](../skills/colocad/references/prompt-patterns.md) cover the whole-product brief, evidence interpretation, feature planning, targeted repair, fit/edit verification and long-task handoff. They are adaptable examples, not approval gates between every operation or a fixed sequence for a simple part.

## Tune the measured failure

Separate interpretation quality from geometry correctness and execution performance. For wrong dimensions, improve source/frame interpretation first. For broken edits, fix dependencies. For tool confusion/context cost, reduce overlap and return focused evidence. For execution time, inspect feature, validation, export and preview timings before optimizing. Reuse clients/checkpoints and shared definitions; reuse a checked mesh for camera-only changes.

Comparisons with build123d need the same verified geometry, output requirements, precision, cold/warm policy and hardware limits. Report repeat counts, distributions, failures and memory accounting. An edited film duration is not a benchmark; cached observation is not equivalent to a fresh build. A 512 MiB CAD run does not cover a resident LLM harness or video encoder.

To evaluate skill changes, hold source packs, tasks, model/provider versions, tools and budget fixed, change one factor, and retain unseen tasks. Measure dimensions/features, edit preservation, fit claims, unsupported claims, correction count, inference cost and end-to-end time. Scripted regressions prove implementation contracts; they do not prove a prompt's success rate on unseen drawings. [Tuning reference](../skills/colocad/references/tuning.md).

## Deliver reproducible results

Return editable source, final exact STEP, the recorded session and requested CLI-exported HTML/movie. Archive the whole session when intermediate STEP files matter; the single HTML includes the final STEP and recorded meshes/receipts. State actual checks and unresolved limitations.

[CLI viewer and rendering](modeling-history.md) · [Harness integrations](integrations/overview.md) · [Real industrial-product example](../examples/hiwin-kk60/README.md)
