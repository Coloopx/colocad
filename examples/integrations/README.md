# Runnable integration examples

The standalone `sdk.mjs` keeps published `@coloopx/colocad@0.1.0-alpha.3` as a compatibility example. The Pi/session adapters import this checkout's SDK; their current query tests require the matching development native. Node.js 22+ is required, plus Rust to build the development executable.

From the repository root:

```sh
node scripts/vendor.mjs
cargo build --locked --release -p agent-cad
export AGENT_CAD_BINARY="$PWD/target/release/agent-cad"
npm ci --prefix kernel --ignore-scripts
npm run build --prefix kernel
npm ci --prefix examples/integrations --ignore-scripts
npm test --prefix examples/integrations
node examples/integrations/sdk.mjs ./out/sdk-example
```

If you set `CARGO_TARGET_DIR`, point `AGENT_CAD_BINARY` at that directory's release executable instead. Unset `AGENT_CAD_BINARY` to run only the legacy `sdk.mjs` against its bundled alpha.3 native; that does not test the new query API.

`sdk.mjs` validates a four-hole plate, renders it, changes the camera using its cached mesh, changes the bore diameter and saves the current source. It prints the artifact paths. Its output root can be reused because the SDK creates new revision subdirectories.

| File | Purpose |
| --- | --- |
| [sdk.mjs](sdk.mjs) | Actual SDK modeling/edit/observation cycle |
| [pi-tools.mjs](pi-tools.mjs) | Five core Pi tools plus capability-gated `cad_query`, schema preservation, revision-aware feedback, error mapping and PNG attachment |
| [pi-core.mjs](pi-core.mjs) | Construct a Pi core agent with a caller-supplied model and stream function |
| [pi-extension.mjs](pi-extension.mjs) | Register the same tools in Pi coding agent |
| [integration.test.mjs](integration.test.mjs) | Real native calls and Pi loop/extension/skill discovery regression tests |

The test model stream is scripted; no LLM provider or credentials are needed. The test asserts a failed edit reaches the agent as an error and the subsequent corrected call exports STEP. It measures integration correctness, not model intelligence or drawing reconstruction success.

Separate [G3 live acceptance](../../specs/experiments/0005/README.md) runs Pi inference with local tools, harness and native in the same 1 vCPU / 512 MiB sandbox. It includes a text-built plate and a seeded 82-solid product edit, with actual tool traces and independent STEP readback. These are bounded tasks, not a drawing-reconstruction success-rate study.

Use the [Pi guide](../../docs/integrations/pi.md) to connect real inference. [Codex](../../docs/integrations/codex.md) and [Claude Code](../../docs/integrations/claude-code.md) use the bundled skill and local CLI. These example files live in GitHub and are not part of the existing alpha.3 tarball.

## Record history in the source checkout

The original SDK tests still pin the published alpha.3 package as a compatibility baseline. The new recorded paths import this checkout's `/session` module and use its native executable.

```sh
node examples/integrations/recorded-pi.mjs ./out/recorded-pi
node examples/integrations/shell-harness.mjs codex ./out/codex-project
node examples/integrations/shell-harness.mjs claude-code ./out/claude-project
```

`recorded-pi.mjs` runs an actual Pi tool loop with a scripted stream: build, failed edit, correction, then offline replay. `shell-harness.mjs` installs the complete skill into the corresponding project skill directory and executes its helper and the record/replay CLI. These validate local tool integration, not live Codex/Claude inference. The Pi extension opts into recording when `COLOCAD_SESSION_DIR` is set. SDK callers pass `cadSession` to `createCadAgent`. All code runs in the same sandbox.

## Compact feedback and a complete guided workflow (alpha.5)

`compact-tools.mjs` preserves the existing schemas and error/degradation semantics while returning measurements, timing and references to full receipts. It is transport-neutral. `guided-workflow.mjs` executes real native modeling, analytic edit checks, failure recovery, cached observation and CLI HTML export. Both files are included in the alpha.5 package.

```sh
node examples/integrations/guided-workflow.mjs ./out/guided-workflow
```

Read the [handbook](../../docs/agent-modeling-handbook.md) for materials, tool abstractions, skill structure, prompt examples and tuning. The walkthrough's decisions are scripted; no live-model reconstruction score is claimed.
