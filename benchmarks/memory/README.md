# Bounded-memory study (2026-11-11)

This directory contains reproducible **experiments**, not a replacement for the released persistence API. Baseline: Colocad alpha.8 / native 0.5.1. Public PyPI pins: CadQuery 2.8.0 and build123d 0.13.0 in separate environments (their supported OCCT dependencies differ).

Every timed job uses a fresh native Linux arm64 container, 1 vCPU, no network, no swap. Installation is excluded. The primary peak is cgroup v2 `memory.peak`; sampled anon/file/kernel and summed process RSS accompany it. The supervisor and all backend descendants are included, external inference/harness and the separate verifier are excluded. Sampling is 20 ms; kernel total peak does not depend on sampling. OOM uses `memory.events` and Docker state. This is a shared development host, with warmed host image pages after unscored pilots; fresh process is not a cold filesystem-cache guarantee.

`plan-v1.json` fixes three repetitions and randomized order before measured runs. The analytic families are square perforated plates, unique 16-hole plates in an assembly, shared-definition instances, and sequential one-hole Boolean feature chains. All require valid exact solids, analytic volume and STEP. Assembly exporters preserve names/placements. No tessellation, inference or video is timed. The geometry route for a regular hole profile is one extrusion on the Python backends; neither is forced to perform serial cuts in the bulk-hole family. The chain family deliberately uses serial cuts.

`plan-extreme.json` increases each family to the first failure or stated ceiling, with a 120 s deadline and 2 GiB cap. Larger planned cases after a failure are explicitly unrun. Limits, timeout, execution error and OOM are distinct outcomes; refusal is not a successful low-memory build. Three common-range repeats provide descriptive medians/ranges, not reliable tail statistics or asymptotic proof.

`plan-sharded.json` tests **a different delivery contract** fairly on all three backends: complete indexed chunked projects, eight unique plates per child invocation, 512 MiB cap, fsync/hash/SQLite commit, and advisory release of this project's completed file pages. It preserves all geometry and placements but does not produce one monolithic assembly STEP. The same physical parts are used. The prototype is Python orchestration for experiment isolation; the product implementation is planned in Rust/TS.

`plan-history.json` isolates the current SDK's full-ledger head lookup versus a bounded SQLite index. Fixtures duplicate an authentic accepted entry into 1k–500k metadata records. **These are not 500k executed CAD edits.** Fixture creation is outside timing; lookup is read-only at the SDK layer. Indexed lookup is O(log H) B-tree search with a 2 MiB cache, not a promise that all geometry fits in logarithmic memory.

`plan-history-node.json` repeats the indexed lookup using the same Node runtime and SDK import as the baseline, separating runtime floor from indexing. Node 22's SQLite API is experimental; this is a measured prototype, not a published SDK dependency. `plan-boundary.json` uses the same 2,048-part input at 4 GiB and 2 GiB and adds markers after validity, volume and solid-count calls. `plan-compositional.json` validates each definition before rigid placement, retains aggregate mass/count and exports the **complete** assembly STEP. It is an exploratory control, not a license to skip verification on arbitrary Booleans. `plan-native-control.json` compares the same direct native call with no checkpoints, checkpoints and query snapshots; it isolates artifact buffering from Node SDK overhead.

Timing is descriptive on this shared development machine. Synthetic history fixture preparation briefly overlapped the baseline on the host (not in another CAD container); do not use these wall times for a narrow speed-regression acceptance threshold. No timed CAD campaigns run concurrently. Original baseline/failed traces remain available; late diagnostic controls are not hidden holdout tests.

```sh
docker build -t colocad-memory:20261111 -f benchmarks/memory/Dockerfile benchmarks/memory
python3 benchmarks/memory/test_catalog.py
python3 benchmarks/memory/run.py --plan benchmarks/memory/plan-v1.json --out out/memory-main
python3 benchmarks/memory/run.py --plan benchmarks/memory/plan-extreme.json --out out/memory-extreme
python3 benchmarks/memory/run.py --plan benchmarks/memory/plan-sharded.json --out out/memory-sharded
```

Run campaigns **sequentially**, not concurrently. Use `history-prepare.py` with an actual session ledger, then pass `--history` to the runner. The runner stores every result, process exit, cgroup event, raw sampled trace and output. Keep original failures when correcting an experimental bug. Digest paths/source/image versions in the resulting evidence rather than assuming the mutable image tag is sufficient provenance.

## Reproducing the published study

Use the source/native baseline at `7b056ed3c6ed39acad7bfe082cd311d957916fb9`, with these benchmark files overlaid. The runner mounts that checkout at `/pkg`; it needs the alpha.8 Linux arm64 executable at `kernel/native/linux-arm64/agent-cad`, not just Rust source. The published npm tarball `@coloopx/colocad@0.1.0-alpha.8` contains it. Verify its SHA-256 against `kernel/release.json` before execution. Do not silently test a newer binary. Use a native arm64 Docker daemon; this campaign did not use x64 emulation.

The Dockerfile pins top-level Python packages. Its base tags and transitive resolution can change: the archived `environment.json` records the actual Python/Node/SQLite/kernel versions and complete distribution versions, and each `runs.json` records the built image ID. A later rebuild is a new environment unless these match. To reproduce dependency versions, use the archived distribution list as constraints when rebuilding. Provision at least 4 GiB for the separate verifier; never include its memory in the measured CAD result.

Run the additional `plan-boundary.json`, `plan-compositional.json`, and `plan-native-control.json` with the same runner. For history, create fixtures outside all timed campaigns:

```sh
python3 benchmarks/memory/history-prepare.py PATH_TO_REAL_SESSION_JSON out/history-fixtures
python3 benchmarks/memory/run.py --plan benchmarks/memory/plan-history.json --history out/history-fixtures --out out/memory-history
python3 benchmarks/memory/run.py --plan benchmarks/memory/plan-history-node.json --history out/history-fixtures --out out/memory-history-node
```

The report scripts expect sibling directories named `main`, `extreme`, `history`, `history-node`, `sharded`, `boundary`, `compositional`, and `native-control` under one result root. Use those names with `--out` when preparing a complete study. Then run `verification-manifest.py ROOT` to check every chunk hash/catalog and select the representative STEP set. Run `verify.py /data/verification-manifest.json /data/verification.json` with `/opt/cq/bin/python` in the image, mounting `ROOT:/data` and this directory at `/bench`, with 1 CPU / 4 GiB / no network / no swap. It verifies every solid and hole in each selected file. A failed assertion must stop report acceptance.

```sh
python3 benchmarks/memory/analyze.py ROOT docs/evidence/memory-2026-11-11
python3 benchmarks/memory/write-report.py ROOT docs
python3 benchmarks/memory/archive-evidence.py ROOT docs/evidence/memory-2026-11-11
```

Analysis additionally needs Matplotlib; it is outside timed jobs. The archive contains raw measurements, traces, receipts, source and plans; geometry and the large synthetic ledgers remain local and are reproducible from the inputs. `raw-manifest.json` hashes the archived files. Original runner versions are retained by their recorded SHA-256: the runner gained optional control dispatch during the investigation, while the timed baseline workload and monitor bytes remained unchanged. Diagnostic postprocessing corrections are retained separately. No failures were erased or rerun into the original successful set.

The lower bound for resident exact geometry is proportional to the information held. Bounded historical/index overhead and a bounded active geometry working set are realistic targets; O(log N) total memory for arbitrary whole-model Boolean operations is not. See [SDD 0006](../../specs/0006-bounded-memory-persistence.zh-CN.md).
