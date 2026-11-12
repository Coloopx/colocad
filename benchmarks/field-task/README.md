# KK60 retrofit and history-growth experiment

The [report](../../docs/field-history.md) and [measured checkpoints](../../docs/benchmarks/alpha10/field-history.json) separate a geometry-backed retrofit from the cost of persisting it. [orders.json](orders.json) is the shared brief. [prepare.mjs](prepare.mjs) exports three common STEP inputs outside the clock; the nominal file must match the independently accepted KK60 stage-six hash. The missing-hole and relocated variations are controlled changes to that reconstruction. No producer receives the feature tree or per-replacement placement. The camera bracket is an experiment design, not a vendor accessory.

Use the pinned Linux arm64 [benchmark image](../memory/Dockerfile) named `colocad-memory:20261111`, a source checkout with the matching native SDK on the host, and the public alpha.10 package unpacked at `$RUNTIME` for the sandbox. `--cpu` takes an available Docker VM CPU number. Each output must be new. Install a separate Python environment with OCCT (`cadquery-ocp-novtk==7.9.3.1.1` for the readback API) and Matplotlib for plotting.

```sh
python3 benchmarks/field-task/run.py out/field --runtime "$RUNTIME"
python3 benchmarks/field-task/verify.py out/field out/field/readbacks.json
python3 benchmarks/field-task/run.py out/field-queries --runtime "$RUNTIME" --history-queries 4096
python3 benchmarks/field-task/run.py out/field-edits --runtime "$RUNTIME" --history-edits 64
python3 benchmarks/field-task/verify.py out/field-edits out/field-edits/readbacks.json edits
python3 benchmarks/field-task/run.py out/field-cache --runtime "$RUNTIME" --history-edits 64 --cache-probe
python3 benchmarks/field-task/plot.py docs/benchmarks/alpha10/field-history.json out/field-history.svg
```

[Colocad](colocad.mjs), [CadQuery and build123d](python-backends.py) obtain their own circle measurements; [the verifier](verify.py) uses fixed work-order dimensions and independent geometry, not the producers' measurements. The four delivered assemblies for each backend can be opened directly from its output directory. The rejection is a pre-delivery cardinality check. Colocad uses a numeric session edit for the resize and a graph build for a new through-window. The immutable-geometry experiment counts only real, durable queries on one accepted revision. The repeated-edit experiment rebuilds 64 distinct thickness/pitch combinations, preserving topology count and two solids. Native SDK-boundary samples read cgroup v2 directly and do not generate a continuous trace file; the high-water mark includes initial geometry construction. Neither loop runs inference. The three-backend installation includes different persistence/preview behavior, so compare geometry outcomes, not workflow timings.

For the exact published 10,001-entry series, extract the [20 ms traces and time anchors](../../docs/benchmarks/alpha10/write-traces.tar.gz) into a fresh directory and use it as RAW_SESSIONS below. Alternatively, use the two **accepted** `session-before/`, `session-after/` monitor directories and their SQLite catalogs under `project-before/edits/`, `project-after/edits/` from the alpha.10 refresh archive (exclude `overlapping-verifier/`). [analyze.py](analyze.py) combines their timestamped progress markers, session creation time and cgroup wall time to bound the clock offset; it samples a four-second window ending before the earliest possible marker. The public [checkpoint JSON](../../docs/benchmarks/alpha10/field-history.json) retains per-window counts, component medians, high-water intervals and source digests. Reproduce those runs from scratch with [the paired session runner](../bounded/session.py) on the public alpha.9 and alpha.10 packages. After restoring or reproducing the traces under the indicated paths:

```sh
python3 benchmarks/field-task/analyze.py RAW_SESSIONS out/field out/field/readbacks.json \
  out/field-queries/history out/field-edits/history out/field-edits/readbacks.json \
  out/field-cache/history/cache-probe.json out/field-history.json
```

The optional [cache probe](cache-probe.py) runs only in its own fresh, completed experiment after the producer exits. It excludes the current head and live database, records a no-op and advisory-release sample, and reopens all 68 STEP hashes. No host-wide cache reset is used.
