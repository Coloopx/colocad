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

## Reproduce the public alpha.10 figures

Use the source tree at tag `v0.1.0-alpha.10` for the workload inputs and the reporting scripts from this documentation revision. Install the public npm tarball. Verify that `src/`, `bin/`, `kernel/dist/`, `kernel/native/` and the skill files in the measured checkout match that install byte for byte. The Linux image digest and package integrity are in the [campaign record](../../docs/benchmarks/alpha10/observations.json).

Run steps 1–4 above in fresh directories, including the extended stress plans. In this rerun the main scaling jobs used CPU 0, fixtures and queues CPU 2, shards CPU 3, extremes and history CPU 4, and query, boundary, compositional and native controls CPU 6. Each measured job used one CPU. These lanes shared an eight-vCPU development VM. In the separate alpha.9/alpha.10 pairs, four observations from two pairs were set aside when the validation lane overlapped the timing lane. Those pairs were repeated on CPU 0 and both long sessions on CPU 2; the diagnostic record preserves the 120 s SDK timeout. Timed binaries, thresholds, input queues, random seeds and stop-after-failure rules came from the frozen plans. Wait for the independent verifier processes to exit successfully before reading their incrementally written JSON.

Additional runs associated with the figures:

- Run the [six shape controls](../shapes/README.md) into `$OUT/shapes`. The shape gallery uses the macOS build123d 0.11.1 environment. Linux fixtures use the pinned image.
- Run steps 1–4 of the [matched release comparison](../bounded/README.md), selecting the public alpha.9 package as the baseline. Write the geometry trials to `$OUT/pairs`, the session measurements to `session-before` and `session-after`, and the projects to `project-before` and `project-after`. Run `node benchmarks/bounded/audit-session.mjs "$OUT/project-VARIANT/edits"` into `audit-VARIANT.log` for each variant.
- Run the two frozen [geometry splits](../../specs/experiments/0005/README.md), the [KK60](../../examples/hiwin-kk60/README.md) and [turbofan](../../examples/turbofan/README.md) sessions, and the [replacement and external-STEP checks](../../examples/engineering-interfaces/README.md). Store the splits in `g2-regression` and `g2-held`, the engineering examples in `flange` and `kk60-interface`, and the per-stage OCCT readbacks in `kk60-readback.json` and `turbofan-readback.json`. Render the KK60 session to `$OUT/cover.gif` at 960 × 600, 8 fps and 0.75 s per step.
- Run the packaged housing example and the [Stanford scan reproduction](../../docs/hybrid.md#release-regressions) in separate Linux arm64 containers, at 1 vCPU and 512 MiB with swap and network disabled. Write into `$OUT/hybrid-housing` and `$OUT/hybrid-scan`. For each case, record the space-separated fields `start_ns end_ns memory.peak` and then the lines of `/sys/fs/cgroup/memory.events` in `hybrid-CASE.log`. Run `test/validation/hybrid-exchange.py` on the housing directory into `hybrid-occt.log`. These are Colocad capability and resource checks.

With the new `campaign.json` in `$OUT` and the completed readbacks, generate the data and figures below. Use a Python environment with build123d, NumPy, Pillow and Matplotlib for the graphics scripts.

```sh
DEST=docs/benchmarks/alpha10
mkdir -p "$DEST"
python3 benchmarks/release/report.py "$OUT" "$DEST"
python3 benchmarks/release/scaling.py "$DEST"
python3 benchmarks/release/unique-phases.py "$OUT" "$DEST"
python3 benchmarks/release/race.py "$OUT" "$DEST"
python3 benchmarks/release/embed.py "$DEST"
python3 benchmarks/release/shape-gallery.py "$OUT/shapes" "$DEST"
python3 benchmarks/release/controls.py "$OUT" "$DEST"
python3 benchmarks/release/acceptance.py "$OUT" "$DEST"
python3 benchmarks/release/extend-scorecard.py "$DEST"
```

The independent readbacks need the original STEP artifacts. After completing validation and rendering, the per-trial workspaces can be losslessly archived. Keep the public JSON, charts and cover as the portable comparison.
