# Colocad 0.1.3 release verification

## Native construction and failure reporting

The two packaged macOS arm64 kernels built the same source in fresh directories on an Apple M3 Max. The table gives median elapsed milliseconds including process startup and artifact writes, and median peak process RSS. Each case used one warm-up pair and seven measured pairs in alternating order. The failure case measured the time to an exact collision receipt. All successful cases used the normal Level-1 validation and STEP exporter.

| Case | 0.1.2 ms | 0.1.3 ms | 0.1.2 MiB | 0.1.3 MiB |
|---|---:|---:|---:|---:|
| Manifold part | 251.1 | 222.4 | 24.08 | 23.08 |
| Collision followed by an independent manifold | 252.6 | 10.5 | 24.67 | 14.50 |
| 24 manifolds, 276 collision pairs | 5,885.5 | 405.7 | 46.55 | 26.31 |
| 24 manifolds, no spatial checks | 254.9 | 229.3 | 27.16 | 25.80 |
| KK60 bracket, ordinary STEP import | 150.3 | 72.3 | 17.20 | 16.94 |
| KK60 bracket, definition import | 149.9 | 72.7 | 17.17 | 16.95 |

The separated assembly returned 276 findings, each with zero overlap. The failing assembly reported 500 mm³ against the same 0.001 mm³ limit. Fourteen incoming STEP parts retained their solids, bounds and volumes in independent OCCT readback. A separate set of 21 exported files covered repeated placements, the drilled block and the washer assembly. A host allowance of 129 occurrences admitted all 129 washers, with a readback volume of 9,367.71093 mm³. The default allowance and the exact-query snapshot capacity remain 128 occurrences.

## Durable editing in Linux

Each version ran one complete 256-edit KK60 retrofit in a separate arm64 Linux container with one pinned vCPU, a 512 MiB cgroup limit, no swap and a native filesystem volume. Each edit produced a distinct STEP delivery. Historical checkout, artifact hashing and independent OCCT geometry checks ran after each session. Shared-library and filesystem page-cache charging affect cgroup peaks. The previous three-backend campaign remains separately labeled in the README.

| Metric | 0.1.2 | 0.1.3 |
|---|---:|---:|
| Median durable edit, ms | 639.96 | 471.46 |
| 95th-percentile durable edit, ms | 684.13 | 530.50 |
| Whole-sandbox peak, MiB | 107.65 | 79.68 |
| Whole-sandbox memory after edit 256, MiB | 91.08 | 60.45 |
| Within-session change after edit 128, MiB | -1.27 | -1.92 |
| Artifact hashes verified | 4,676.00 | 4,676.00 |

## Execution and recovery

A controlled subprocess emitted 13 ordered progress events over 124 seconds. The public CLI received the fixture's final diagnostic at 124.16 seconds. The SDK also received the final diagnostic. This exercises supervision independently of geometry speed. A single native primitive can remain silent throughout its execution. Hosts set the inactivity window for the primitives they admit. Fixed poses support early collision checks. Mated instances are checked after the solver fixes their placement. The original 170-second production recipe was unavailable.

The regression suites passed 184 Rust tests, 129 SDK/CLI tests, and 6 harness integrations. Raw samples include the warm-ups, paired measurements, Linux checkpoints, native digests and independent geometry checks. The two-version scope supports conclusions for the measured workloads.

[Raw release evidence](benchmarks/stable-0.1.3/gate.json) · [Native A/B runner](https://github.com/Coloopx/colocad/blob/v0.1.3/benchmarks/release/native-comparison.py) · [KK60 reproducibility instructions](https://github.com/Coloopx/colocad/tree/v0.1.3/benchmarks/field-task) · [Three-platform delivery](https://github.com/Coloopx/colocad/releases/download/v0.1.3/release-validation.json).
