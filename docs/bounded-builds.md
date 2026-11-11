# Bounded writes, assembly working sets and Level-1 validation

[Single-file interactive HTML / 中文对照图](benchmarks/bounded/comparison.html) · [All measured rows and artifact hashes](benchmarks/bounded/observations.json) · [Reproduce](../benchmarks/bounded/README.md)

Source candidate `9f53a4e` versus the public `@coloopx/colocad@0.1.0-alpha.9`. This is not an npm release. The three changes target different costs.

| Primary measure | alpha.9 | Candidate | Reduction |
|---|---:|---:|---:|
| 10,001-step session, whole sandbox peak | 213.19 MiB | 69.67 MiB | 67.3% |
| 64 independent parts, live feature bodies | 64 | 1 | 63 fewer |
| Same assembly, whole sandbox peak | 319.53 MiB | 100.62 MiB | 68.5% |
| 1,024-hole plate, isolated Level-1 check (macOS) | 34.68 s | 26.30 s | 24.2% |
| Same plate, end-to-end to STEP (Linux) | 54.33 s | 43.55 s | 19.9% |

**What changed.** The indexed session still executes every query. New sessions store requests by content, and each complete reply as a separately hashed gzip member. The catalog commits an offset only after data and directory syncs. It doesn't rebuild the accepted model or keep a process-local history list. A Linux hint is limited to already synced, sealed project packs. During plain assembly export, eligible terminal parts go directly from their local STEP definition to exact world placement, Level-1 validation and measurement. The executor then drops the B-rep. The single STEP remains deterministic and fully assembled. A planar Face 37 prefilter excludes provably distant spline loops while all other checks, tolerances and placements stay in place.

**Long-session evidence.** Both runs reopened the project, verified all 10,001 entries in order and found `s10001` as the accepted head. A separate audit checked every full receipt, all 9,900 request/revision bindings and all 101 model/STEP hashes. File count dropped from 21,217 to 1,503. Sampled maxima changed from 138.16 to 31.40 MiB of file cache and from 42.94 to 5.21 MiB of kernel memory. These maxima are not necessarily simultaneous. One full 10k write per reported variant; wall times are recorded, not used as a speed gate. Filesystem cache attribution and host load vary. [Storage layout, relocation and compatibility](indexed-sessions.md).

**Geometry measurements.** Linux arm64, same SDK workload and pinned container image, a fresh 1 vCPU / 512 MiB cgroup with zero swap per invocation. Median of three wall times and three independent kernel `memory.peak` readings per cell. Identical read-only binary mount paths for both variants; before/after order alternates by repetition. All 72 builds succeeded.

| Frozen input | alpha.9 s | Candidate s | alpha.9 MiB | Candidate MiB | Live bodies |
|---|---:|---:|---:|---:|---:|
| part 16 | 0.60 | 0.57 | 49.32 | 49.56 | 1 → 1 |
| part 256 | 8.73 | 8.34 | 122.48 | 122.83 | 1 → 1 |
| part 1024 | 54.33 | 43.55 | 326.41 | 326.82 | 1 → 1 |
| unique 1 | 0.58 | 0.56 | 49.69 | 50.27 | 1 → 1 |
| unique 16 | 8.19 | 7.93 | 119.97 | 67.61 | 16 → 1 |
| unique 64 | 32.48 | 31.47 | 319.53 | 100.62 | 64 → 1 |
| repeated 1 | 0.68 | 0.58 | 49.71 | 50.33 | 1 → 1 |
| repeated 32 | 0.60 | 0.58 | 51.23 | 51.23 | 1 → 1 |
| repeated 128 | 0.62 | 0.59 | 51.47 | 51.79 | 1 → 1 |
| chain 8 | 0.48 | 0.48 | 48.26 | 48.06 | 1 → 1 |
| chain 32 | 3.36 | 3.30 | 60.31 | 60.03 | 1 → 1 |
| chain 64 | 10.93 | 10.57 | 82.67 | 82.30 | 1 → 1 |

**Acceptance.** For all 12 inputs, both variants and all three repetitions agree on input hash, STEP bytes, geometry and validation reports. The unchanged independent OCCT verifier checked every distinct artifact: solid validity and count, each solid's bounds and placement, analytic volume and every hole's radius, axis and centre. The 18-case regression and six-case holdout suites also passed. The new native negative test checks a nested trim and a wrongly designated outer loop. Across the normal suite plus that separate negative fixture: 131 Rust, 50 Node and six harness integration tests passed. Strict Clippy with warnings promoted to errors still finds five diagnostics in unchanged alpha.9 library files (mates, query contracts/engine and relationships).

**Limits.** The plate's end-to-end RSS is not the reported metric and its sandbox memory did not materially fall. The large-assembly benefit requires one use of a terminal part and no preview, snapshot, mates or inter-part checks; repeated occurrences already shared a definition. Neutral/STEP bytes, genuinely live geometry, session metadata, new requests and geometry artifacts still occupy space. A full viewer export materializes its requested history. These measurements do not establish O(log n) whole-sandbox memory. The isolated Level-1 timer uses matched macOS builds and a timer-only alpha.9 probe; compare it only to its macOS counterpart. Small-case differences need not generalize.
