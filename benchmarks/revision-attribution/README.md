# Attribute the late memory growth in Colocad 0.1.1

[Measurements and proposal (中文)](../../docs/revision-memory-attribution.zh-CN.md) · [Samples, checks and allocation stacks](../../docs/benchmarks/revision-memory-attribution/evidence.json)

All timings, memory counters and page-residency probes are Linux-specific. Use the same npm tarball, three inputs and Docker image as the [release campaign](../readme/stable/README.md). Keep the package, the experiment scripts and the verifier separate. These probes run only on disposable output volumes. Production CAD code is unchanged.

## Input

Set `WORK` to an absolute directory and `PY` to a Python 3.12 interpreter with Matplotlib. Extract the verified release to `$WORK/package`. The original release archive contains `trial/history/round-{1,2,3}/{colocad,cadquery,build123d}/result.json`, the three neutral STEP inputs under `round-1/inputs`, and the accepted session under `round-1/colocad/session`. Extract these to `$WORK/raw`. To produce equivalent new data without the archive, use the [release reproduction](../readme/stable/README.md) and supply its results to the live and layout probes. New wall times and original-trial deltas are machine-dependent. The summarizer's original-trial check requires the exact release samples.

```sh
INPUTS="$WORK/raw/trial/history/round-1/inputs"
SRC="$WORK/raw/trial/history/round-1/colocad/session"
BENCH=benchmarks/revision-attribution
"$PY" "$BENCH/capture.py" "$SRC" "$WORK/capture"
"$PY" "$BENCH/live.py" "$WORK/package" "$INPUTS" "$WORK/live"
"$PY" "$BENCH/run-layout.py" "$WORK/capture" "$WORK/layout"
"$PY" "$BENCH/run-layout.py" "$WORK/capture" "$WORK/sync" --sync-pairs
sh "$BENCH/probes.sh" "$WORK" "$INPUTS" lookup
sh "$BENCH/probes.sh" "$WORK" "$INPUTS" allocations
"$PY" "$BENCH/live.py" "$WORK/package" "$INPUTS" "$WORK/serial" --cold-audit
"$PY" "$BENCH/summarize.py" "$WORK" "$WORK/report"
"$PY" "$BENCH/plot.py" "$WORK/report"
```

The tested host exposes four guest CPUs. Paired live runs use lanes 0 and 1, each with one vCPU. Each lane runs the two GC conditions in opposite order. The storage pairs run serially on CPU 3. In the recorded campaign some storage runs and the allocation probe shared the host with live tests, and the supplemental cold-page census ran separately. Each geometry run is subject to the same STEP, mesh, receipt and historical query checks as the release. A failed live run preserves its Docker volume for investigation.

## Controls

- A geometry trial saves 4 setup builds and 256 real edits. Sampling follows setup, edit 1 and every fourth edit. Both GC conditions start Node with `--expose-gc`. Only the forced condition requests collection at each sample. The fixed-size geometry and the benchmark's growing list of edit identifiers, parameters and hashes are common to both arms.
- The V8 probe samples allocations across 64 edits at 32 KiB intervals including objects collected by minor or major GC. Attribute each sampled node's own allocation to the nearest package/benchmark frame on its call stack. This measures allocation traffic in one JS process, excluding Rust and external buffer storage.
- The persistence replay consumes the exact source, receipts, geometry, checkpoints, snapshots and previews from the 256 accepted edit artifacts. A common sequential payload input avoids charging thousands of source directory entries to the writer. Both layouts maintain an append-only member ledger, sync once per member and sync the ledger at each commit. Segment addressing needs fewer index bytes than loose paths. The sync comparison varies only the segment data fsync cadence. Both arms keep the renderable geometry and every byte of every member. A fresh process checks the complete manifest and all member hashes outside the timed interval.
- Random-access testing builds a fixed-width offset sidecar, selects nonsequential historic revisions in a fresh process and materializes one revision at a time. The original OCCT verifier and published native snapshot query verify the selected revision. Full-session compatibility, concurrent readers and crash recovery are future implementation gates.
- The serial post-commit probe uses `mincore` to count resident pages in existing revision files, before and after advising only displaced immutable members. The head and SQLite file are observed without advice. Residency counts can include pages charged outside the measured cgroup. A full historical scan is diagnostic and is excluded from the production proposal.
