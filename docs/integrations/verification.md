# Integration verification

Verified on **2026-11-07**, Node.js **22.23.3**, **macOS arm64**, using the public npm package **@coloopx/colocad@0.1.0-alpha.3** and its bundled Rust binary. Pi core, Pi AI and Pi coding agent are pinned to **1.5.0** in the example lockfile.

| Check | Result | What it establishes |
| --- | --- | --- |
| SDK validate → preview → camera change → parameter edit | Pass | Real geometry, cached observation, expected removed volume and persisted source |
| Pi adapter schemas, preview image, failure mapping and cancellation before dispatch | Pass | Complete `$defs`, valid PNG content, error results and no dispatch after pre-cancellation |
| Explicit degraded feature | Pass | The omitted fillet and native error remain in model-visible tool text |
| Pi core tool loop with a failed edit then corrected build | Pass | Actual agent loop propagates failure and resumes to a successful STEP export |
| Pi coding agent extension loader and skill discovery | Pass | Five executable CAD tools register; the complete skill is discoverable |
| Existing root Node regression suite | 5 passed | Feature compiler, edits, output collision handling and native-output refusal |
| Documented CLI and copied skill commands | Pass | Feature-tree planning/build/edit and kernel-model helpers execute from a consuming project |
| Independent OCCT STEP readback | 4 passed | Two manifold revisions and two copied-skill plates are valid single solids with checked material regions |

[Machine-readable results, volumes, probes and STEP hashes](../evidence/harness-integrations.json).

Reproduce the integration tests from the repository root:

```sh
npm ci --prefix examples/integrations --ignore-scripts
npm test --prefix examples/integrations
node examples/integrations/sdk.mjs ./out/sdk-example
```

The five integration tests run native CAD operations and actual Pi library code. The model response stream is deliberately scripted: no provider API key, inference request or autonomous model decision is involved. Thus the result verifies execution contracts and recovery, not unseen-drawing success rate or a model's ability to choose the correct CAD features.

Codex skill paths/invocation and Claude Code skill paths/invocation were checked against their official documentation. The bundled helper was run from both `.agents/skills/colocad` and `.claude/skills/colocad`. No Claude Code live session was run, and this test run did not evaluate a live Codex model reconstructing a drawing. The Pi live-provider schema negotiation remains dependent on the provider you configure.

This verification adds no new performance claims. Cross-platform npm release checks are recorded separately in [release-alpha.3.json](../evidence/release-alpha.3.json); the new adapter regression itself was run on macOS arm64. Native SOLIDWORKS acceptance remains subject to the [existing limitations](../validation.md).
