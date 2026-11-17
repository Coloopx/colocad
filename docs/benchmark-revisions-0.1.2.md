# KK60 revision history on Colocad 0.1.2

The workload fits a camera adapter to an imported KK60 saddle, accepts four setup deliveries and rejects a replacement with only three mounting circles. It then changes thickness and camera-hole pitch across 256 distinct configurations. Each backend saves an editable recipe, STEP assembly, mesh and revision index. Colocad saves its native checkpoints and exact snapshots using segmented storage. The SQLite-based Python adapters sync their files and advise the kernel when clean pages can be released.

| Backend | Peak MiB (range) | Edit 128 MiB | Edit 256 MiB | Last 128 edits Δ MiB | Edit p95 |
|---|---:|---:|---:|---:|---:|
| Colocad 0.1.2 | 82.9 (81.9–83.3) | 66.8 | 65.1 | -1.76 | 668 ms |
| CadQuery 2.8.0 | 199.8 (199.7–200.0) | 195.5 | 195.8 | +0.45 | 55 ms |
| build123d 0.13.0 | 270.9 (260.9–271.0) | 258.4 | 266.9 | +7.44 | 55 ms |

## Individual sessions

| Backend | Round | Peak MiB | Late cgroup Δ MiB | Late slab Δ MiB | Edit p50 ms | Edit p95 ms | All 256 edits s |
|---|---:|---:|---:|---:|---:|---:|---:|
| Colocad 0.1.2 | 1 | 82.88 | -0.46 | +0.08 | 627.3 | 675.0 | 163.5 |
| Colocad 0.1.2 | 2 | 81.89 | -1.76 | +0.07 | 631.8 | 667.7 | 162.8 |
| Colocad 0.1.2 | 3 | 83.33 | -1.84 | +0.09 | 629.8 | 664.5 | 161.4 |
| CadQuery 2.8.0 | 1 | 199.72 | +0.26 | +0.34 | 46.3 | 54.5 | 12.1 |
| CadQuery 2.8.0 | 2 | 200.02 | +0.48 | +0.34 | 43.4 | 46.8 | 11.2 |
| CadQuery 2.8.0 | 3 | 199.78 | +0.45 | +0.34 | 45.8 | 60.0 | 16.8 |
| build123d 0.13.0 | 1 | 270.91 | +6.99 | +0.34 | 51.7 | 56.4 | 13.4 |
| build123d 0.13.0 | 2 | 270.97 | +9.02 | +0.34 | 50.0 | 54.5 | 12.9 |
| build123d 0.13.0 | 3 | 260.94 | +7.44 | +0.33 | 51.4 | 55.4 | 13.3 |

Each timed session ran serially on CPU 0 in its own Linux arm64 cgroup and Docker-managed native volume, with 512 MiB, zero swap and networking disabled. The three runtimes were preloaded outside the scored cgroups. The orders were CadQuery–Colocad–build123d, Colocad–build123d–CadQuery and build123d–CadQuery–Colocad. Independent readback ran on the host outside the timed cgroups. The candidate package's SDK, CLI, viewer, schemas and native executable digests are recorded in the dataset. The npm release checks compare those digests against the final archive.

`memory.current` was sampled after setup, edit 1 and every fourth commit. Peak is Linux `memory.peak` through edit 256. Late deltas subtract edit 128 from edit 256 within each trial. Each p95 is the nearest rank of 256 complete edits. The aggregate table takes medians across the three trials. Linux page-cache charging and heap collection can move the endpoints. These trials measure the 128–256 interval.

The verifier reopened all 2,304 edited STEP files and checked two valid solids, placement and dimensions, volume, mounting and camera holes, window, contact and interference. It also verified each preview hash, six preview bounds per session and six historical hole pitches. Colocad historical queries used saved snapshots with zero geometry rebuilds. The Python adapters reopened saved STEPs. Colocad used 1 mm preview chord tolerance, the Python STL writers used 0.15 mm linear and 0.15 rad angular tolerance. Timings include these different preview and history deliverables. No trial reported an OOM kill.

[Raw samples, geometry readbacks and package file hashes](benchmarks/stable-0.1.2/engineering.json) · [Offline report](https://github.com/Coloopx/colocad/releases/download/v0.1.2/scorecard.html) · [Reproduction instructions](../benchmarks/readme/stable/README.md#three-backend-history-for-012) · [Paired comparison with 0.1.1](benchmark-segmented-sessions.md).
