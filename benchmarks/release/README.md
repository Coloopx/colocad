# Reproduce the alpha.9 release comparison

Node 22 and the three native executables come with the npm package. For a source checkout, stage those exact executables under `kernel/native` and run `node scripts/check-release.mjs`. Use the pinned Linux arm64 [memory study image](../memory/README.md) with separate CadQuery 2.8.0 and build123d 0.13.0 environments. Measurements use a shared development host; a fresh process and cgroup do not imply a cold page cache.

1. Run `python3 benchmarks/memory/run.py --plan benchmarks/memory/plan-v1.json --out "$OUT/main"` on a new absolute output path. This is the frozen 108-call scaling matrix.
2. Run `python3 benchmarks/release/run-fixtures.py "$OUT"` followed by `python3 benchmarks/release/run-windows.py "$OUT"`. The runner generates the 11 product/parameter fixtures, including the same complete tendon hand from three separately implemented builders. Three randomized repeats per fixture; the hand queue uses one window per backend after warm-up. Each family uses its own original resource budget. Do not combine them into one hardware performance score.
3. For the archived stress controls, `lanes.py "$OUT" --history FIXTURES --old-indexed EXTRACTED_BASELINE --query-seed SEED` runs both product suites above and the frozen stress plans. Use this *instead of* step 2, after step 1. The recorded run used separate guest CPU sets (0, 2, 4, 6) for products, extremes, diagnostics and shards on an 8-vCPU shared VM. Each lane is serial, each workload gets one vCPU; cache, disk and the host are shared. The historical indexed baseline is commit `e568748`; prepare the imported 256-hole session as in the [query experiment](../query-io/README.md), and metadata fixtures using `benchmarks/memory/history-prepare.py`.
4. Outside timing, run `verify-main.py "$OUT"` and pass its manifest to [verify.py](../memory/verify.py) with `$OUT` mounted as `/data`; write `/data/main-verified.json`. Run `verify-fixtures.py "$OUT" fixtures` and `verify-fixtures.py "$OUT" windows` with Python 3.12, build123d and OCP. The checker verifies each successful STEP, each counted handoff, bounds and material volume of matched solids, and the bore and cable edits. The first repeat of the product cases also checks both directions of geometric difference against the reference export.
5. `python3 benchmarks/release/report.py "$OUT" docs/benchmarks/alpha9` requires completed independent readbacks. `race.py "$OUT" docs/benchmarks/alpha9` uses Pillow, NumPy and build123d to draw the timed queues and the four actual STEP views. Finish with `python3 benchmarks/release/embed.py docs/benchmarks/alpha9` to embed the replay and downloadable observations. Charts and motion are calculated from timestamps; MP4/GIF production is not part of build time.

Time includes process startup, model construction, backend validation and delivery. The main workload uses normal SDK checkpoints; Python outputs one STEP. Product jobs record `/proc/PID/io` after child exit and before reaping it: `wchar` counts bytes passed to write-like syscalls (including waited-for children and stdout). The separate retained-byte metric counts workspace artifacts excluding sources and benchmark instrumentation. Neither number is physical SSD traffic or durability latency. Shared mount caches can bypass cgroup block attribution, so absent block counts do not imply free I/O. Memory is kernel cgroup peak (main and fixtures), or sampled peak (100 s queue); the agent, harness, verifier, renderer and inference are excluded.

The runnable frozen geometry corpus is in [0005](../../specs/experiments/0005/README.md). The catalog append/migration and query experiments are internal comparisons; a successful eight-part shard is not counted as a monolithic assembly in the three-backend score. See the release report for the separate outcomes.

## Source candidate and assembly follow-up

The [candidate scorecard](../../docs/benchmarks/bounded-candidate/scorecard.html) records the source commit and measured native executable hashes. The first campaign reran the frozen primary suites and stress controls at `9f53a4e`. A second campaign reran all 207 primary jobs and all three hand queues on the subsequent assembly optimization. Fresh containers used the same image, budgets and inputs. The follow-up ran scaling, product/queue and paired jobs on separate CPU sets (0, 2 and 4). The stress records identify the earlier binary. Full result sets from different revisions have separate run directories.

Build an isolated checkout of each revision. Stage its compiled native and catalog binaries or explicitly provide `AGENT_CAD_BINARY` in every measured container. Use steps 1, 2 and 4 above for the primary corpus. Run [the paired runner](../bounded/README.md) with `--cases unique:64 unique:16 unique:1` for the direct follow-up and audit the retained STEP hashes against the primary OCCT readbacks. The [six shape controls](../shapes/README.md) use separate macOS processes. Diagnostic profiling of one saved definition is available with `cargo run --release -p agent-cad --example profile_splines -- ARTIFACT_DIRECTORY DEFINITION_ID`.

Set `campaign.json` in each run directory with the measured label, commit, native hashes and image ID, as recorded under `campaign` in the observations. Generate the complete comparison from verified run directories, in this order:

```sh
python3 benchmarks/release/report.py FINAL_RUN REPORT_DIR
python3 benchmarks/release/scaling.py REPORT_DIR
python3 benchmarks/release/unique-phases.py INITIAL_RUN FINAL_RUN REPORT_DIR
python3 benchmarks/release/race.py FINAL_RUN REPORT_DIR
python3 benchmarks/release/embed.py REPORT_DIR
python3 benchmarks/release/controls.py INITIAL_RUN REPORT_DIR
python3 benchmarks/release/shape-gallery.py SHAPE_RUN REPORT_DIR
python3 benchmarks/release/iterations.py INITIAL_RUN FINAL_RUN PAIRED_HISTORY_JSON DIAGNOSTIC_DIR REPORT_DIR
python3 benchmarks/release/extend-scorecard.py REPORT_DIR
python3 benchmarks/release/candidate-docs.py REPORT_DIR
```

Use the plotting / build123d Python environment for `scaling.py`, `unique-phases.py`, `race.py` and `shape-gallery.py`. The final HTML embeds its figures, GIF, filterable measurements, shape readbacks and supplementary observations. Candidate images in the READMEs are repository-relative so the checked-out revision selects the matching files.
