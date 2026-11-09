# Colocad

![HIWIN KK60 linear actuator — actual native modeling and assembly in four views](showcase/hiwin-kk60/modeling.gif)

**Parametric CAD for agents. Local Rust geometry, a TypeScript SDK, a CLI, and an inspectable modeling history.**

[中文](README.zh-CN.md) · [Documentation](docs/README.md) · [npm](https://www.npmjs.com/package/@coloopx/colocad) · [Changelog](CHANGELOG.md) · [Releases](https://github.com/Coloopx/colocad/releases) · [Model behind the cover](examples/hiwin-kk60/README.md)

Colocad turns explicit modeling instructions into editable source, exact STEP geometry and visual review artifacts. It runs alongside the agent and its harness **inside the same sandbox**. The npm package includes the compiled Rust kernel; installed modeling needs no Python, Rust toolchain, network or separate CAD service.

**Alpha software:** useful for supported mechanical parts and assemblies, with measured performance and reproducible examples. It is not a complete replacement for build123d or a manufacturing-certified CAD system. See [support and limits](#support-and-limits).

## What you can build

- **Parts and assemblies:** parametric features, repeated part definitions, exact STEP and PNG/GLB/STL previews.
- **Engineering queries (alpha.7):** exact topology, composable selection, datums, holes/groups, mounting interfaces and measured assembly relationships. [Supported domains](docs/engineering-queries.md).
- **Agent workflows:** SDK tools and a bundled skill for interpreting dimensions, planning features, editing and checking results.
- **Human review:** persistent build attempts, source/receipt inspection, offline single-file HTML and four-view MP4/GIF replay.

The SDK is the foundation; the CLI and tool adapters expose it to your harness; the skill guides modeling decisions. Images and drawings are evidence the agent interprets. Colocad executes explicit geometry instructions.

```mermaid
flowchart LR
  A[Agent + skill] --> H[Existing harness]
  H --> C[CLI / SDK tools]
  C --> K[Local Rust kernel]
  K --> F[STEP + source + receipts + meshes]
  F --> V[CLI viewer / HTML / animation]
```

## Install

Requires **Node.js 22+** on **Linux x64, Linux arm64 or macOS arm64**.

```sh
npm install @coloopx/colocad@0.1.0-alpha.7
```

The package ships JS/TS APIs, the CLI, native executables, viewer and complete skill. Rust source is in [crates/agent-cad](crates/agent-cad); the npm tarball contains compiled binaries. [Package and model formats](docs/concepts.md).

## Quick start

Run these commands from your Node project:

```sh
# Build the packaged manifold and save its source, receipt and geometry.
npx colocad record node_modules/@coloopx/colocad/examples/manifold.json ./history \
  --label "Build the manifold"

# Launch the interactive viewer.
npx colocad view ./history

# Export one offline HTML file, including a final STEP download.
npx colocad replay ./history ./review.html
```

The manifold includes an extrusion, blind cut, fillet, linear pattern and pocket. Record an edited JSON into the same history directory to append a revision; failed attempts remain visible. `view` serves a snapshot, so reopen it after adding revisions. [First part and parameter edit](docs/getting-started.md).

Prefer an API? Use the same kernel through a persistent session:

```js
import { ModelingSession } from '@coloopx/colocad/session';

const session = await ModelingSession.create('./sdk-history');
const result = await session.build({
  version: 1, units: 'mm', parameters: { length: 60 },
  features: [{ id: 'stock', op: 'box', origin: [0, 0, 0],
    size: [{ param: 'length' }, 30, 8] }],
  output: 'stock',
});
if (!result.ok) throw new Error(JSON.stringify(result.error));
await session.edit({ length: 80 });
```

Use `CadClient` for direct builds without session recording. [SDK, CLI and tool API](docs/api.md).

## Visualize and export from the CLI

![CLI modeling-history viewer showing the KK60 assembly](https://raw.githubusercontent.com/Coloopx/colocad/main/showcase/hiwin-kk60/viewer.png)

Inspect the timeline, source, features, receipts and selected parts. Switch cameras, show display sections or explosion, and download artifacts. The offline HTML embeds its rendering code and geometry. Hosts can also embed `@coloopx/colocad/viewer`.

| Command | Input | Result |
| --- | --- | --- |
| `colocad plan TREE.json` | `colocad/1` feature tree | Compile and inspect a native job |
| `colocad build TREE.json NEW_DIR --preview` | Feature tree | STEP, editable source and previews |
| `colocad record MODEL.json SESSION` | Feature tree or kernel JSON | Build and persist a milestone |
| `colocad view SESSION` | Saved session | Interactive local viewer |
| `colocad replay SESSION review.html` | Saved session | Self-contained offline HTML |
| `colocad render SESSION modeling.mp4` | Saved session | Four-view animation; `.gif` also supported |

In a headless sandbox, use `colocad view ./history --no-open --port 8787` with the harness's port forwarding; the listener binds to `127.0.0.1`.

```sh
# Optional movie dependencies; install FFmpeg on the host as well.
npm install -D playwright
npx playwright install chromium
npx colocad render ./history ./modeling.mp4 --width 1280 --height 800 --fps 15
```

Movies replay **actual saved revisions**. Recorded call times and edited playback duration are separate. Building geometry and exporting HTML do not need Chromium or FFmpeg. [History, viewer and rendering guide](docs/modeling-history.md).

## Use with your agent

Copy the whole skill, including its references, scripts and editable shape assets:

```sh
# Codex project skill; use .claude/skills for Claude Code.
mkdir -p .agents/skills
cp -R node_modules/@coloopx/colocad/skills/colocad .agents/skills/colocad
```

If that skill already exists, preserve local customizations before updating it. Then ask:

> Use the colocad skill to build a 100 × 50 × 6 mm plate with a centered 3 × 2 grid of Ø6 through-holes on 32 × 30 mm pitches. Keep editable source and modeling history, verify the dimensions, and export STEP and an offline HTML review through the CLI.

| Harness | Integration |
| --- | --- |
| Codex | [Project skill and CLI](docs/integrations/codex.md) |
| Claude Code | [Project skill and local tools](docs/integrations/claude-code.md) |
| Pi core / coding agent | [Typed tool adapter and extension](docs/integrations/pi.md) |
| Your own harness | [SDK schemas, state and artifact lifecycle](docs/integrations/overview.md) |

The [end-to-end handbook](docs/agent-modeling-handbook.md) covers source preparation, dimension uncertainty, tool abstraction, prompts, modeling loops, validation and tuning. A [runnable guided workflow](examples/integrations/README.md) demonstrates real builds, quantitative edits and recovery; its decisions are scripted, not a live-LLM benchmark.

## Example: a complete linear actuator

The cover reconstructs a **HIWIN KK60, 300 mm rail, 5 mm lead, 210 mm travel** from manufacturer drawings: **52 final features, 81 named solid occurrences and 16 modeling milestones**. It includes a physically cut helical screw groove, rail tracks, mounting holes, carriage, bearings and coupling.

```sh
node node_modules/@coloopx/colocad/examples/hiwin-kk60/run.mjs ./actuator
npx colocad view ./actuator/session
npx colocad render ./actuator/session ./modeling.mp4
```

Independent OCCT checks cover all 16 STEP revisions, 64 material probes and selected rail/carriage clearances. Proprietary internals are explicitly approximated or omitted; this is a catalog reconstruction, not a production drawing package. The animation is not a speed benchmark.

[MP4](showcase/hiwin-kk60/modeling.mp4) · [Editable model](showcase/hiwin-kk60/model.json) · [Checks](showcase/hiwin-kk60/verification.json) · [Dimensions and assumptions](examples/hiwin-kk60/sources.json) · [Full offline delivery](https://github.com/Coloopx/colocad/releases/download/v0.1.0-alpha.4/hiwin-kk60-delivery.zip)

## Performance

Performance depends on geometry and execution mode. In a measured 100-second queue of the same **43-solid tendon hand**, Colocad delivered **44 complete STEP revisions versus build123d's 33**, with **57.1% lower highest sampled sandbox memory**.

![10× replay: STEP deliveries and peak-so-far sampled sandbox memory](showcase/tendon-hand/window/hand-window-peak-en.gif)

| Same revision queue, 1 vCPU / 512 MiB | Colocad | build123d |
| --- | ---: | ---: |
| Complete STEP deliveries in 100 s | **44** | 33 |
| First STEP | **2.223 s** | 3.010 s |
| Median job, same first 33 inputs | **2.234 s** | 2.855 s |
| Highest sampled cgroup memory | **130.9 MiB** | 304.9 MiB |

One window per backend; a fresh CAD process per revision; approximately 25 ms memory sampling includes subprocesses, temporary files and charged file cache. Every counted STEP reopened successfully. This run used the alpha.2 SDK and the optimized kernel first bundled in alpha.3; it is not a new alpha.6 measurement. The GIF plots the highest sample so far, not instantaneous usage. [Method and samples](showcase/tendon-hand/window/README.md).

<details>
<summary>Cold-start parts and cached previews</summary>

![Time to STEP and peak memory on four historical workloads](docs/benchmarks/time-to-step.svg)

The published alpha.2 kernel reached STEP **22.0× faster** for a sensor rail and **12.9× faster** for a manifold, using **7.7% and 8.0%** of build123d's peak memory. Five fresh processes/cgroups per part and backend after warm-up; Linux arm64, 1 vCPU / 512 MiB. Startup/imports, modeling, checks and export count; installation, previews, inference and a resident agent do not. These gains do not apply to every operation: manifold geometry alone took 53.7 ms versus build123d's 43.8 ms, and the old complete-hand result was slower.

For unchanged geometry, saved-mesh observation was 5.6×–16.8× faster than Colocad's own rebuild-and-render path on two parts. This is an internal cache comparison, not a build123d result.

</details>

[Full performance evidence and limitations](docs/performance.md) · [Offline comparison with STEP downloads](https://github.com/Coloopx/colocad/raw/refs/heads/main/docs/benchmarks/comparison.html).

## Support and limits

- Native packages: Linux x64/arm64 and macOS arm64, Node.js 22+. [Release verification](docs/evidence/release-alpha.7.json).
- Editable JSON/programs and exact STEP are supported; STEP does not preserve a commercial CAD application's native feature tree.
- General editable SOLIDWORKS output on Linux remains incomplete. The experimental Windows author requires licensed SOLIDWORKS, and a native rebuild pass is still unverified. [Native author status](docs/native-solidworks.md).
- CAD checks, drawing interpretation, edit preservation, fit and manufacturing readiness are separate claims. [Validation scope](docs/validation.md).
- A 512 MiB CAD test is not a memory guarantee for an entire resident agent, browser or video encoder.

## Documentation and help

Available in alpha.7: [exact queries and geometry-backed assembly](docs/engineering-queries.md), with a [runnable external STEP example](examples/engineering-queries/README.md). The [four ordered goals](specs/0005-capability-reliability-and-optimization.md) track functional coverage, geometry reliability, engineering usability, then benchmark-driven optimization. The SDK, CLI, tool schema and all three native targets ship together; this is a Colocad API, not CadQuery Python/API compatibility.

G1–G3 are accepted for the [declared workloads](specs/experiments/0005/README.md): 24/24 geometry cases independently verified, 127 Rust / 41 Node tests, and two live Pi tasks in Linux arm64 **1 vCPU / 512 MiB**. Peak cgroup memory was **256 MiB** for the corpus with a minimal supervisor and **371 MiB** for resident Pi/harness plus the 82-solid actuator edit and preview; zero OOM. This is resource/correctness acceptance, not a new build123d speed comparison or full industry-wide parity. G4 has not started.


[Getting started](docs/getting-started.md) · [API](docs/api.md) · [Agent handbook](docs/agent-modeling-handbook.md) · [Skill](skills/colocad/SKILL.md) · [Troubleshooting](docs/troubleshooting.md) · [All documentation](docs/README.md)

For a bug or feature request, [open an issue](https://github.com/Coloopx/colocad/issues) with the package version, platform, minimal model and relevant receipt. Remove credentials and private source material.

## Develop and contribute

```sh
git clone https://github.com/Coloopx/colocad.git
cd colocad
npm ci
npm run build:kernel
npm test
```

Building from source requires the Rust toolchain and fetches the pinned SolidCore dependency. Viewer/render and Pi integration checks have additional dependencies; see [CONTRIBUTING.md](CONTRIBUTING.md) for setup, tests and release instructions. Contributions are welcome as issues and pull requests.

## License and acknowledgments

Colocad application code is [MIT licensed](LICENSE). The kernel builds on [SolidCore](https://github.com/tangency-labs/solidcore), including work derived from NVIDIA USD BRep / SMLib; the viewer uses Three.js. Dependency licenses and attribution are in [NOTICE.md](NOTICE.md) and [kernel/licenses](kernel/licenses). The industrial example is not affiliated with or endorsed by HIWIN.
