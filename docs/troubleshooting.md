# Troubleshooting

| Symptom | Check and action |
| --- | --- |
| Skill cannot be found | Check `.agents/skills/colocad/SKILL.md` for Codex/Pi or `.claude/skills/colocad/SKILL.md` for Claude Code; start in that project and refresh/restart discovery. |
| Skill helper cannot import colocad | Install the pinned package in the current project and run the helper from there. Copy the complete skill folder. |
| `colocad build` rejects `version: 1` | That CLI expects `format: "colocad/1"`. Use `CadClient` or the skill helper for a kernel model. |
| Output directory already exists | Pick a new revision destination for the CLI/helper. Keep earlier accepted output. `CadClient.outputRoot` can contain multiple revisions. |
| Unsupported platform / missing executable | Alpha.3 bundles Linux x64/arm64 and macOS arm64. Check `process.platform`, `process.arch` and the installed `kernel/native` directory. Source builds require a Rust toolchain and the pinned dependency. |
| Native launch fails | Check `LOCAL_EXECUTION_FAILED`, executable permissions, mount policy and `binary`/`AGENT_CAD_BINARY` overrides. Run the local CLI to isolate harness wiring. |
| `TIMEOUT` | Inspect the failing feature and workload. Simplify or split unnecessary work; adjust `wallMs` within 1–120,000 only when appropriate. Sandbox memory failures need sandbox diagnostics too. |
| `RESOURCE_LIMIT` / `OUTPUT_LIMIT` | Inspect capabilities and reduce model/mesh payload. SDK stdin is limited to 32 MiB; the feature-tree CLI accepts 2 MiB. |
| Unknown/missing parameter | Use an existing finite numeric parameter. `edit()` does not create parameters; update the owned source to add one. |
| An edit undoes a prior change | You reused stale source. Load the last accepted `model.json` or persist the merged parameters after each successful edit. |
| Preview succeeds but no STEP in its directory | A cached observation points to `source_artifact_directory`. Retrieve STEP from the source geometry revision. |
| Cached preview cannot be read | Retain its source directory and files; do not modify cache bytes. Rebuild from the saved model if the cache was pruned or is invalid. |
| Agent cannot see the rendered image | Attach PNG as image content or invoke the harness's image-reading tool. Printing the local filename alone is insufficient. |
| Provider rejects tool schema | Preserve root `$defs`; check the provider's schema restrictions. Use skill + shell tools if the provider cannot accept the complete schema. |
| No editable native SOLIDWORKS tree | STEP is not a feature-history file. The Windows author is experimental; generic Linux native authoring is not complete. |

Some low-level failures retain the legacy error code `WORKER_FAILED`. It is an error label in alpha.3, not a requirement to deploy a separate service. Native execution is local.

For a useful bug report, include package/platform/Node versions, minimal model JSON, the failed receipt, expected geometry and reproduction command. Remove credentials and private design data. Attach a small STEP and screenshot when shareable. Keep the original file if the failure depends on STEP import; otherwise reduce the model to the failing dependency chain.

See [Contributing](../CONTRIBUTING.md) for the test commands and [validation](validation.md) for known limitations.
