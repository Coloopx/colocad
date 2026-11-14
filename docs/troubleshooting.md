# Troubleshooting

| Symptom | Check and action |
| --- | --- |
| Near-circular elliptical extrusion returns `GEOMETRY_VALIDATION_FAILED` | Reproduced in 0.1.0 for a 49.9858 × 49.963 mm base. Preserve the specified ellipse when both dimensions matter. The circular variant in the appearance study changes one dimension by 0.0228 mm. [Reproduction and measurements](benchmark-appearance.md). |
| Skill cannot be found | Check `.agents/skills/colocad/SKILL.md` for Codex/Pi or `.claude/skills/colocad/SKILL.md` for Claude Code; start in that project and refresh/restart discovery. |
| Skill helper cannot import colocad | Install the pinned package in the current project and run the helper from there. Copy the complete skill folder. |
| `colocad build` rejects `version: 1` | That CLI expects `format: "colocad/1"`. Use `CadClient` or the skill helper for a kernel model. |
| Output directory already exists | Pick a new revision destination for the CLI/helper. Keep earlier accepted output. `CadClient.outputRoot` can contain multiple revisions. |
| Unsupported platform / missing executable | Alpha.8 bundles Linux x64/arm64 and macOS arm64. Check `process.platform`, `process.arch` and the installed `kernel/native` directory. Source builds require a Rust toolchain and the pinned dependency. |
| Native launch fails | Check `LOCAL_EXECUTION_FAILED`, executable permissions, mount policy and `binary`/`AGENT_CAD_BINARY` overrides. Run the local CLI to isolate harness wiring. |
| `TIMEOUT` | Inspect the failing feature and workload. Simplify or split unnecessary work; adjust `wallMs` within 1–120,000 only when appropriate. Sandbox memory failures need sandbox diagnostics too. |
| `RESOURCE_LIMIT` / `OUTPUT_LIMIT` | Inspect capabilities and reduce model/mesh payload. SDK stdin is limited to 32 MiB; the feature-tree CLI accepts 2 MiB. |
| Unknown/missing parameter | Use an existing finite numeric parameter. `edit()` does not create parameters; update the owned source to add one. |
| An edit undoes a prior change | You reused stale source. Load the last accepted `model.json` or persist the merged parameters after each successful edit. |
| `cad_query` is absent | Inspect installed native capabilities. Alpha.7 includes exact queries; an older package or `AGENT_CAD_BINARY` override can expose older capabilities. Keep SDK and native from the same release/build. |
| No exact query snapshot | Rebuild the accepted source with `{queryable:true}` or CLI `--queryable`. A preview mesh or STEP file alone is not the native snapshot. Datums/interfaces require snapshots and reject explicit disabling with `QUERY_SNAPSHOT_REQUIRED`. |
| Alpha.7 `face_hole_grid` + `queryable` reports `BSplineCurve::make_nurb` status 1001 | Upgrade to alpha.8 with its matching native. The nested seam UV-line snapshot defect is fixed; preview precision was not the cause. Check `AGENT_CAD_BINARY` if an old native is still used. [Repair evidence](evidence/release-alpha.8.json). |
| `QUERY_REVISION_MISMATCH` | Obtain the latest accepted `query_revision_id` and rerun the semantic selection. Do not rewrite an old topology ref's revision field to bypass the check. |
| `QUERY_ARTIFACT_OUTSIDE_ROOT` | The harness must configure the appropriate trusted `queryRoots` or relocate the complete artifact directory under its allowed output root. The agent's query JSON cannot grant itself new roots. |
| `SELECTION_CARDINALITY`, missing datum/interface or ambiguous candidates | Inspect actual candidates, support plane, coordinate space and orientation. Refine a geometric rule; do not choose the first face or suppress ambiguity. |
| `INTERFACE_INCOMPATIBLE` / `INTERFACE_NOT_FOUND` during replacement | Compare required pitch, bore size, count, plane relation and register geometry. Preserve accepted source; retain a genuinely incompatible replacement as a failed attempt. |
| Query succeeds with `partial` coverage | Read diagnostics and source refs. Unsupported or unobserved geometry is not evidence of absence; required fit/clearance claims need complete supported evidence. |
| `QUERY_LIMIT` | Reduce target scope, nested selection/search size or page size. Check advertised budgets. Do not truncate candidates and still claim unique/complete recognition. |
| Positive distance but clearance fails | Inspect `relationship` and `overlap_mm3`. Boundary distance is unsigned; containment is not separation. Curved/region-specific contact certification also has explicit support limits. |
| `NO_MATERIAL_REMOVED` | The required cut was disjoint or tangent. Check tool placement, axis, depth and intended stock; do not treat the unchanged body as a machined result. |
| Old checkpoint is not reused after an upgrade | A changed/missing `execution_policy` intentionally forces replay. Keep accepted source and let the new executor rebuild; do not forge cache metadata. |
| Preview succeeds but no STEP in its directory | A cached observation points to `source_artifact_directory`. Retrieve STEP from the source geometry revision. |
| Cached preview cannot be read | Retain its source directory and files; do not modify cache bytes. Rebuild from the saved model if the cache was pruned or is invalid. |
| Agent cannot see the rendered image | Attach PNG as image content or invoke the harness's image-reading tool. Printing the local filename alone is insufficient. |
| Provider rejects tool schema | Preserve root `$defs`; check the provider's schema restrictions. Use skill + shell tools if the provider cannot accept the complete schema. |
| No editable native SOLIDWORKS tree | STEP is not a feature-history file. The Windows author is experimental; generic Linux native authoring is not complete. |

Some low-level failures retain the legacy error code `WORKER_FAILED`. It is an error label in alpha.3, not a requirement to deploy a separate service. Native execution is local.

For a useful bug report, include package/platform/Node versions, minimal model JSON, the failed receipt, expected geometry and reproduction command. Remove credentials and private design data. Attach a small STEP and screenshot when shareable. Keep the original file if the failure depends on STEP import; otherwise reduce the model to the failing dependency chain.

See [Contributing](../CONTRIBUTING.md) for the test commands and [validation](validation.md) for known limitations.
