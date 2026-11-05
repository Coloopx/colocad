# Why the hand slowed down as more parts were added

A fresh build of a single proximal link took 228 ms with the published colocad kernel. Repeating the **same definition** at 43 locations took 7,420 ms. Repeating that test with the optimized kernel took 249 ms. A hand with 43 different occurrences, including links, sockets and angled thumb parts, took 2,239 ms. All times are medians of five new processes.

| Workload | Published alpha.2 | Optimized source | build123d 0.11.1 |
|---|---:|---:|---:|
| One link | 228 ms | 236 ms | 2,117 ms |
| Four copies of this link | 694 ms | 241 ms | 2,226 ms |
| Sixteen copies of this link | 2,719 ms | 229 ms | 2,131 ms |
| Forty-three copies of this link | 7,420 ms | 249 ms | 2,612 ms |
| Sixteen copies of a simple pin | 518 ms | 68 ms | 2,200 ms |
| Complete tendon hand, 43 occurrences | 3,929 ms | 2,239 ms | 3,335 ms |
| Highest measured cgroup peak, complete hand | 52.7 MiB | 37.9 MiB | 271.3 MiB |

The repeated-link cases evaluate one identical feature graph and apply a grid of translations. Comparing the pin and link rows holds count at sixteen while changing geometry. The real hand has 20 source definitions and 21 distinct definition/orientation combinations. [Five-run statistics](evidence/reuse-summary.json) · [All trials including warm-ups](evidence/reuse-trials.json)

## What consumed the time

The Python process pays the import cost once per call, whether it builds a link or a hand. In a separate import-only probe on the same runner, the median was 364 ms for importing OCP alone and 1,369 ms for importing build123d. Loading build123d also loaded NumPy, SciPy, IPython, ocpsvg and ocp_gordon. These are separate fresh-process timings and they do not attribute the difference to any one dependency. [Import receipts](evidence/import-sample.json)

The old Rust assembly path copied and placed each body, then ran a level-1 B-rep check, manifold check, precise volume integration and tight bounding-box calculation at **every placement**. The feature graph and the STEP writer already supported definition reuse. In a diagnostic build with separate timers, the three-run full-hand medians were 3,135 ms for the geometry and manifold checks, 826 ms for volume integration and 201 ms for world bounds. Coordinate transformation took 4 ms. Diagnostic timings were measured separately from the delivery table. The sweep uses neither collision constraints nor preview rendering. [Diagnostic timings](evidence/profile.json) · [Timer patch against commit e7b6d92](bench/profile-alpha2.patch)

## What CadQuery contributed

CadQuery 2.8.0 caches a part definition when building its XCAF assembly and places component references with `TopLoc_Location`. The sandbox benchmark uses build123d 0.11.1 with the cadquery-ocp-novtk 7.9.3.1.1 OCCT bindings. The `cadquery` high-level Python package was absent from the image and `OCP` was present in the loaded modules. The connection to this optimization is how the assembly distinguishes a reusable part from its placements. The benchmark compares build123d with colocad. [CadQuery 2.8.0 assembly implementation](https://github.com/CadQuery/cadquery/blob/v2.8.0/cadquery/occ_impl/assembly.py#L444) · [CadQuery release and OCP description](https://pypi.org/project/cadquery/2.8.0/)

## What the source kernel now does

The first occurrence of a definition at a given solved orientation is fully measured. A later occurrence with the same definition and identical linear transform inherits the validated topology, solid count and volume. Its tight world bounding box is translated by the difference in origins. References stay local to one build. A new orientation gets its own measurement. Explicit spatial checks materialize and validate every placed body, and out-of-range bounds fall back to direct measurement. The kernel report records `measured_instances` and `translated_instance_reuses`. [Implementation](../../crates/agent-cad/src/assembly.rs)

OpenCascade independently reopened the three outputs for each workload. Every placed solid's bounds and integrated volume were compared. For the complete hand both directions of boolean difference were zero for all 20 unique part definitions before and after the change. The Rust regression suite also checks rotated and translated bounds, assemblies, constraints and collisions. The full-hand PNG, GLB and STL also match byte for byte. [Readback measurements](evidence/readback-reuse.json) · [Preview hashes](evidence/preview-reuse.json)

## Replay and reproduce

The hand animation contains two fresh instrumented comparisons at actual playback speed. These use the same optimized Linux arm64 kernel as the five-run experiment. The section and assembly milestones use independently reopened geometry and are outside the measurement clock. The reference is Shadow Hand E3M5, and the internal cord routes are assumptions. [Single-file player with both STEP pairs](https://github.com/Coloopx/colocad/raw/refs/heads/main/showcase/tendon-hand/replay/hand-replay.html) · [Finger trace](replay/finger-trace.json) · [Hand trace](replay/hand-trace.json) · [Reference provenance](reference/NOTICE.md)

Install `@coloopx/colocad@0.1.0-alpha.3` to use the optimized kernel directly. The historical npm baseline for the following reproduction is `@coloopx/colocad@0.1.0-alpha.2`. To use the optimized source kernel, build on the target platform from this checkout, then point the installed SDK at that executable. The following commands run from the checkout on Linux arm64 with the project's Rust toolchain.

```sh
npm run build:kernel
export AGENT_CAD_BINARY="$PWD/kernel/native/linux-arm64/agent-cad"
```

The evaluation runs in an offline Linux arm64 container with 1 vCPU and 512 MiB. All three arms receive the same JSON, tarball, SDK wrapper where applicable and output policy. Each workload and backend gets one excluded warm-up and five measured fresh processes and cgroups. Backend order varies by fixture and repetition. Time includes startup, modeling, each arm's validation, and STEP export. Memory is the largest observed cgroup peak across the five measured runs. No package installation, LLM inference or preview is timed. The separate 100-second experiment in the [original study](README.md) applies to the original npm binary.

```sh
npm pack @coloopx/colocad@0.1.0-alpha.2
python3 showcase/tendon-hand/bench/scope.py abelien/runner:dev ./coloopx-colocad-0.1.0-alpha.2.tgz ./paired --instances --candidate "$AGENT_CAD_BINARY"
node showcase/tendon-hand/bench/scope-models.mjs ./inputs
.venv-eval/bin/python showcase/tendon-hand/bench/check-reuse.py ./paired ./inputs/hand.json ./checks.json
```

Use the public runner alternative and interpreter override documented in the [benchmark instructions](../../benchmarks/readme/README.md) if needed. The evaluated binary's SHA-256 is recorded in the raw trials. For a replay, add `--case finger` or `--case hand` and `--kernel "$AGENT_CAD_BINARY"` to the [trace recorder](../../benchmarks/readme/replay/trace.py). Place the trace and readback receipts in `replay/`. Generate the images with `tools/build-scenes.mjs`, `tools/assets.py` and `tools/prepare-replay.py`, then use `tools/hand-film.py` and `tools/hand-offline.py`.
