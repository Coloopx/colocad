# Tool contracts for a local CAD harness

The existing harness owns inference, permissions, output locations and artifact retention. Bind SDK functions to that harness; do not introduce a CAD service or remote scheduling layer for this package. The agent supplies geometry inputs, not arbitrary executable paths or process flags.

## Expose tools at the useful level

Start with the five tools used by the executable Pi adapter; enable sketch solving or STEP inspection when that workflow needs them. Keep the SDK's schema and `$defs` intact instead of rewriting an approximate subset.

| Existing tool | Meaning | Important boundary |
| --- | --- | --- |
| `cad_capabilities` | Read operations, limits and unsupported cases | Installed version is authoritative |
| `cad_validate` | Check input/parameters/dependencies/budgets | Does not build or validate the resulting shape |
| `cad_build` | Execute an explicit model and return exact artifacts | Creates a new revision; inspect errors/degradations |
| `cad_preview` | Produce a view, reusing eligible saved mesh | Geometry, pose or precision changes can require work |
| `cad_edit` | Rebuild changed named parameters | Plain tools take `{model,parameters}`; recorded tools take `{parameters}` |
| `cad_solve_sketch` | Local sketch constraints/residuals/DOF | Local solution/rank is not proof of global uniqueness |
| `cad_inspect_step` | Inspect a registered STEP asset and its instances | Import budgets and supported representations apply |

The current API does not provide `image_to_cad`, an automatic drawing-dimension extractor, or a universal exact measurement tool in this tool set. Use the harness's file/image tools for evidence and an appropriate independent geometry verifier for checks. The SDK's `inspect` reads a report; it is not an arbitrary geometric query engine.

## Keep feedback focused and inspectable

A useful response includes `ok`, the failing feature/code/message, revision and artifact paths/hashes, solid count, bounds/volume, relevant measurements, timings, cache reuse and every degradation. Keep full feature traces and source files on disk. Return relevant details or a PNG when they change the next decision; do not send STEP text, complete meshes or every intermediate image back to the model.

The repository's `examples/integrations/compact-tools.mjs` wraps `createSessionTools` without changing tool schemas or error meaning. It retains full receipts, returns a compact feature summary and points to the original receipt. `guided-workflow.mjs` exercises real builds, dimension edits, an invalid edit, recovery, cached observation and CLI HTML export. The sequence is scripted; it is not an LLM success-rate evaluation.

For a provider that expects an error flag or thrown error, map `ok:false` into that provider's tool-error convention. Preserve the structured detail so the agent can repair the exact cause. A successful transport call is not automatically a successful CAD operation. An `on_fail:record` result still contains omissions and cannot satisfy a required omitted feature.

## Reuse and persistence

Use one client/session for sequential revisions of one modeling task. Separate independent tasks' output roots. A session stores accepted inputs and failed attempts; archive its directory before the sandbox expires. `preview` can reuse a checked mesh; observation-only artifacts may point to `source_artifact_directory` for the exact STEP. Avoid overlapping writes to one session.

Prefer a small semantic set of SDK tools over an RPC endpoint per primitive. Repeated geometry can be generated in a program/feature graph and sent in a meaningful build. Keep enough milestones to inspect geometry creation, not just the final assembly.
