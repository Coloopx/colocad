# Three-backend measurements for Colocad 0.1.1

On November 17, 2026 the benchmark downloaded and integrity-checked the public npm release. Linux arm64 workloads used Colocad 0.1.1, CadQuery 2.8.0 and build123d 0.13.0. The package includes the merged STEP budgets, planar-conic bounds and adjacent-source reuse fixes. [中文](benchmark-stable.zh-CN.md) · [Package, native hashes and exact image dependencies](benchmarks/stable-0.1.1/observations.json).

## Memory over 256 saved revisions

The camera adapter is fitted to an imported KK60 saddle. Each trial accepts four setup deliveries, then changes plate thickness and camera-hole pitch through 256 distinct assemblies. All three backends retain editable inputs, assembly STEP, mesh preview and an indexed revision. Each session uses one vCPU, 512 MiB, zero swap and a fresh native Linux volume with networking disabled. After preloading all three runtimes, three rounds rotate backend order. Independent OCCT readback passed on all 2,304 edited STEPs in the scored trials and all 768 in the initial run.

| Workflow | Peak MiB (median, range) | Current at edit 128 (MiB) | Current at edit 256 (MiB) | Last 128 edits (Δ MiB) |
|---|---:|---:|---:|---:|
| Colocad 0.1.1 | **91.3** (91.0–91.4) | 68.3 | 74.3 | +6.5 |
| CadQuery 2.8.0 | **199.7** (199.6–200.3) | 195.5 | 195.9 | +0.8 |
| build123d 0.13.0 | **268.1** (261.2–272.1) | 258.4 | 264.4 | +6.0 |

The kernel tracks the cumulative high-water mark in `memory.peak`. Current memory comes from `memory.current` after setup, edit 1 and every fourth commit. Each column shows the median across the three runs. For late growth, subtract edit 128 from edit 256 within each run first. The min–max range shows run-to-run variation on this host. These 256-edit observations establish the change over this interval. Longer sessions and other geometry would be needed to infer an asymptotic growth rate.

The first run is recorded separately. Peak memory was **82.9 / 259.0 / 300.2 MiB**, in Colocad / CadQuery / build123d order. Shared file pages can be charged to the cgroup that first loads them. The download retains file, anonymous and kernel charges so the cache effect can be inspected. [Checkpoint series, all individual trials and independent readbacks](benchmarks/stable-0.1.1/engineering.json).

Python uses SQLite FULL synchronization, fsync and Linux clean-page advice. Colocad additionally stores native checkpoints, query snapshots and extra previews. Colocad uses 1 mm preview chord tolerance. Python STL export uses 0.15 mm linear and 0.15 rad angular deflection. The common oracle checks exact STEP solids, mounting and camera bores, window, surrounding material, volume, contact and overlap. Each session checks every preview hash and reads six saved pitches.

## Construction, assembly and throughput

Three fresh processes per backend and case delivered 207 verified STEPs. Timers include startup, native checks and export. Eleven fixtures have a 512 MiB limit and twelve scaling points have 2 GiB. Values below follow Colocad / CadQuery / build123d order.

| Workload | Time to STEP (s) | Peak sandbox memory (MiB) |
|---|---|---|
| 128 placements of one plate | 0.60 / 1.69 / 2.30 | 51.7 / 260.8 / 301.7 |
| 64 independently sized plates | 9.85 / 1.95 / 2.44 | 101.2 / 255.7 / 290.9 |
| One 1,024-hole solid | 45.44 / 3.94 / 3.67 | 311.4 / 240.8 / 275.4 |
| 43-solid tendon hand | 2.12 / 1.73 / 2.10 | 51.0 / 207.6 / 244.8 |

One 100-second queue per backend delivered **52 / 63 / 52** verified hand revisions. Maximum sampled sandbox memory was **200.4 / 512.0 / 291.3 MiB** at about 25 ms intervals. The mechanical matrix uses kernel high-water marks and macOS appearance tests use process-tree RSS. [Hand readbacks](benchmarks/stable-0.1.1/windows-verified.json) · [Phase timings for 64 distinct plates](benchmarks/stable-0.1.1/unique-phases.svg).

On the two installation workflows each backend delivered seven valid assemblies and rejected three incompatible inputs. High-level blind-hole recognition on the reimported saddle remained partial. Colocad located the mount using the circular edges. Independent checks covered all 16 rebuilt KK60 stages ending with 81 occurrences, the six macOS shape controls and the frozen 24-case geometry corpus. [Engineering change-order replay](benchmarks/stable-0.1.1/engineering-changes.gif) · [Shape readbacks](benchmarks/stable-0.1.1/shapes.json) · [Geometry corpus](benchmarks/stable-0.1.1/geometry-corpus.json).

[Offline single-file scorecard](benchmarks/stable-0.1.1/scorecard.html) · [Reproduction](../benchmarks/readme/stable/README.md) · [Five appearance models in fresh and loaded hosts](benchmark-appearance.md).

## External STEP round trips and budgets

Independent OCCT readback compared solid count, volume, bounds and validity before and after import. **17 of 19** STEP files passed. The outstanding cases were mori, mushroom. The 32 adjacent definitions sharing one source needed one parse and 31 cache hits, then exported 32 valid solids. All three input STEP, output STEP and report byte-budget controls rejected their over-budget operation without leaving a partial output directory. [Per-input results](benchmarks/stable-0.1.1/step-roundtrips.json).

## Resident Pi agent and harness

| Task | Acceptance | Sandbox peak MiB | OOM kills |
|---|---|---:|---:|
| 24-case corpus with Node supervisor | Passed | 214.7 | 0 |
| Live Pi four-hole plate edit and query | Passed | 94.8 | 0 |
| Live Pi 82-solid actuator edit and query | Failed | 333.9 | 0 |

Each task used a fresh 1-vCPU, 512-MiB, zero-swap Linux cgroup. For both Pi tasks the agent, harness and Colocad shared the container. Inference ran remotely. The table retains each task’s first outcome. [Acceptance results](benchmarks/stable-0.1.1/agent-sandbox.json).

The actuator’s first edit wrote a STEP, but the model ended its response with `length` before checking the updated interface and clearance. The task failed acceptance. One fresh repeat with the same package, prompt, tools and acceptance criteria **failed**, peaking at 323.5 MiB. The largest query receipts were 158 and 150 KiB of compact JSON in the two trials. Both model responses ended with `length`.
