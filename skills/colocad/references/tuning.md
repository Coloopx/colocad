# Diagnose and tune the actual failure

Change one cause at a time and retain the original acceptance target. These are starting heuristics to evaluate on your workload, not a proven universal optimum.

| Symptom | Check first | Useful change / proof |
| --- | --- | --- |
| Looks plausible, dimensions wrong | Source annotation, scale, radius/diameter, frame | Reinspect crop; update dimension evidence; measure the interface |
| Wrong hidden geometry | Missing section/reference | Mark uncertainty; obtain a source or constrain the claim instead of inventing precision |
| A dimension edit breaks mating | Constants duplicated across parts, unstable references | Link parameter expressions/datums; rebuild an edit stress case |
| Boolean/fillet fails | Feature ID, operand overlap, tangent/degenerate regions, selector count | Reduce to a small reproducer; fix geometry/selection; keep required detail |
| Native success but omitted detail | `degradation` records | Restore required feature or report explicitly allowed omission |
| Large context and tool confusion | Redundant tools, full schema/traces/images every turn | Load the needed reference/operation; return compact receipts and targeted views |
| Slow repeated observation | Whether geometry actually changed | Reuse persisted meshes for camera-only changes; measure cache hits |
| Slow complete rebuild | Per-feature and validation/export timing | Reuse one client/checkpoints and instance definitions; optimize the measured bottleneck |
| Memory grows with repeated parts | Duplicated geometry/meshes, unnecessary copies | Shared definitions and bounded previews; sample the whole relevant cgroup/process set |
| Movie clips the product | Perspective framing, aspect ratio, section plane | Orthographic bounds-based fit; inspect all cameras; do not alter exact geometry to fix framing |

## Measure separate stages

For large assemblies, split geometry residency, validity checks, mass/count queries, export buffers, history metadata and display bundles. A shared definition does not guarantee every downstream verifier/exporter shares its temporary state. Prefer reusable definition-level checks when rigid-placement invariants justify them, and independently check the complete delivered geometry. Do not simply remove verification.

The alpha.8 feature/instance/cutter limits are policy boundaries, not evidence of logarithmic memory. A chunked project can bound active geometry by persisting completed parts and loading only the needed chunk; it is a different delivery contract from one assembly STEP. The memory-study prototypes are not installed SDK commands. Preserve units, placements, parameter source, hashes and all required geometry. Keep history and observations out of growing in-memory arrays; paged persistence is an alpha.9 project, not an existing promise.

Keep source interpretation/inference, CAD startup, geometry, validation, STEP export, tessellation, view rendering and video encoding separate. Time-to-STEP and time-to-preview answer different questions. A cached view cannot be compared with another tool's fresh geometry build as if they were the same task.

For a fair CadQuery or build123d comparison, use the same dimensions, features, instances, precision and required artifacts. Verify both geometries independently, pin environments and define cold/warm behavior. Report repeat counts, median/tails, failures and the accounting boundary. For memory, state sampled RSS/process sum/cgroup, sampling interval and whether the plot is instantaneous or the peak-so-far; retaining a peak is a presentation choice, not the original trace.

A 1 vCPU / 512 MiB CAD run establishes only that scoped run. It does not establish the memory budget of the resident agent/harness, an in-process LLM, Chromium or FFmpeg. Do not extrapolate a plate's speedup to a threaded multi-part product.

## Evaluate the skill/prompt, not just the executor

Hold product tasks, source packs, model/provider version, tools and budget fixed. Compare the existing skill with one changed instruction/tool response policy. Include a dimensioned part, a part needing section interpretation, an editable assembly and an ambiguous source case. Keep some tasks unseen during tuning. Measure required dimensions/features, edit preservation, fit checks, unsupported claims, correction count, inference cost and end-to-end duration.

Use numerical checks and inspectable artifacts for acceptance. Scripted tool tests establish transport and geometry contracts; they do not establish that a model understood an unseen drawing. Report failed tasks and unknown evidence, and do not describe an unrun prompt ablation as an improvement result.

## Preserve an improvement experiment

For an explicit optimization task, freeze the baseline, task split, required geometry, tolerances, verifier and resource budget before changing code or guidance. Retain the original failure and a small reproducer. Change one hypothesis, pass affected-family and frozen regressions, then challenge held-out product families. Include failures in the denominator and preserve rejected or inconclusive candidates. Do not change the evaluator/reference to make a candidate pass. A performance gain needs repeat measurements beyond noise; a skill gain needs live-agent evidence, not only scripted tests. Stop at the authorized iteration/cost budget and keep the last accepted state. Publishing is a separate authorized action, not a tool-score side effect.
