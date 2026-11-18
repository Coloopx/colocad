# Reproduce the stable release scorecard

The [published measurements](../../../docs/benchmarks/stable-0.1.1/observations.json) identify the public npm tarball, its integrity, native executable hashes, Docker image and installed Python distributions. Use Node 22+, Linux arm64 Docker with cgroup v2 and a separate OCCT/build123d verification environment. The six macOS shape controls use build123d 0.11.1. The two timed Linux Python environments use CadQuery 2.8.0 and build123d 0.13.0. The release SDK, native executables, CLI and skill must all come from the same tarball.

## Stage a clean runtime

From the benchmark repository root, set `OUT` to a new absolute directory and `PY` to the absolute path of a Python 3.12 interpreter with build123d, OCP, NumPy, Pillow and Matplotlib. Keep the original checkout for rendering and validation. The temporary rig combines frozen experiment scripts with the extracted npm package. Docker must be able to mount both paths.

```sh
REPO="$PWD"
mkdir -p "$OUT/package" "$OUT/rig" "$OUT/trial"
npm pack @coloopx/colocad@0.1.1 --pack-destination "$OUT"
tar xzf "$OUT/coloopx-colocad-0.1.1.tgz" --strip-components=1 -C "$OUT/package"
rsync -a benchmarks showcase specs examples "$OUT/rig/"
mkdir -p "$OUT/rig/docs/benchmarks/alpha10"
cp docs/benchmarks/alpha10/kk60-readback.json "$OUT/rig/docs/benchmarks/alpha10/"
rsync -a "$OUT/package/" "$OUT/rig/"
docker build -t colocad-memory:20261111 -f benchmarks/memory/Dockerfile benchmarks/memory
```

Check the package SHA-256 against the campaign record. The historical KK60 input digest under `alpha10` is used only to check the identity of the source STEP. Image dependencies installed by pip can change. Record the new image ID and both environments’ `pip freeze` when rebuilding the image.

## Run and independently verify

Run from `"$OUT/rig"`. Each of the original twelve scaling sizes and eleven mechanical cases runs three times per backend. The fixtures generate the common input for the hand window. Each field workflow gets a fresh Docker-managed Linux volume, a 512 MiB memory limit and zero swap. Set `CPU` to an available guest CPU index.

```sh
cd "$OUT/rig"
D="$OUT/trial"
CPU=0
python3 benchmarks/memory/run.py --plan benchmarks/memory/plan-v1.json --out "$D/main"
python3 benchmarks/release/verify-main.py "$D"
docker run --rm --network none --cpus 1 --memory 2g --memory-swap 2g \
  -v "$PWD:/rig:ro" -v "$D:/data" colocad-memory:20261111 /opt/cq/bin/python \
  /rig/benchmarks/memory/verify.py /data/main-manifest.json /data/main-verified.json
python3 benchmarks/release/run-fixtures.py "$D"
"$PY" benchmarks/release/verify-fixtures.py "$D" fixtures
python3 benchmarks/release/run-windows.py "$D"
"$PY" benchmarks/release/verify-fixtures.py "$D" windows
python3 benchmarks/field-task/run-comparison.py "$D/field" --candidate "$OUT/package" --cpu "$CPU"
"$PY" benchmarks/readme/candidate/run-flanges.py "$D/flange" --cpu "$CPU"
for b in colocad cadquery build123d; do
  "$PY" benchmarks/field-task/verify-comparison.py "$D/field" "$b"
done
"$PY" -c 'import json,sys; print(json.dumps(json.load(open(sys.argv[1]))["edits"]))' \
  "$D/field/colocad-checked.json" > "$D/field/native-oracle.json"
node benchmarks/field-task/check-revisions.mjs "$D/field/colocad" \
  "$D/field/native-oracle.json" "$D/field/colocad-archive.json"
"$PY" benchmarks/readme/stable/history.py "$D/history" --runtime "$OUT/package" --cpu "$CPU"
```

The first field run is recorded separately. The memory figure uses three additional rounds after preloading all three runtimes, rotating their order and creating a fresh cgroup and volume for every session. A checkpoint follows setup, edit 1 and every fourth edit. The line shows the median post-commit memory, the band spans all three readings, and the peak column uses the kernel high-water mark through the indicated edit. Late growth is computed within each session before taking its median. The verifiers reopen all 256 edited STEPs in every session and check indexed history, mesh hashes and six historical pitches. Physical pages already cached on the host can be charged outside a new cgroup. macOS RSS and Linux cgroup bytes have separate charts.

## Generate the reports

Run the packaged KK60 example and its [OCCT verifier](../../../examples/hiwin-kk60/verify.py). Render `cover.gif` and `cover.mp4` from its session at 960 × 600, 8 fps and 0.75 seconds per milestone. Place the files and `kk60-readback.json` under `$D`. Run the [six shape controls](../../shapes/README.md) into `$D/shapes` with the extracted macOS binary and `COLOCAD_BENCH_REVISION` set to the release commit. Run the frozen [geometry corpus](../../../specs/experiments/0005/README.md) into `$D/g2-regression` and `$D/g2-held_out`.

Write `$D/campaign.json` with the measured package identity and environment, using the `campaign` object from the public observations as the schema. Set `runtime_kind` to `public npm release` and supply the newly measured hashes. From the original checkout, run the publisher and offline browser test. These require the original STEP outputs, receipt JSON files, Pillow, Matplotlib, build123d, Playwright and Chromium.

```sh
cd "$REPO"
DEST="$OUT/report"
"$PY" benchmarks/readme/candidate/publish.py "$D" "$DEST"
node benchmarks/readme/candidate/check-offline.mjs "$DEST/scorecard.html"
```

The separate [five-product appearance experiment](../../../docs/benchmark-appearance.md) uses the original supplied cases on macOS. Its public measurement file records model and source-input hashes, fresh-process samples, loaded-host times, STEP hashes, geometric checks and the exact-ellipse control.

## Sandbox and STEP acceptance

The [resident Pi harness](../../../specs/experiments/0005/README.md#reproduce-the-sandbox-runs) exercises the four-hole plate and 82-solid KK60 edit with actual inference. Mount the staged rig at `/repo` and set `AGENT_CAD_BINARY=/repo/kernel/native/linux-arm64/agent-cad` so both the installed SDK and native binary match the release. Use a fresh 512 MiB cgroup per task. The [release record](../../../docs/benchmarks/stable-0.1.1/agent-sandbox.json) retains tool counts, CAD readbacks, peaks and OOM events. External STEP round trips separately compare OCCT validity, solid count, bounds and volume against each input. The [per-input results](../../../docs/benchmarks/stable-0.1.1/step-roundtrips.json) include source hashes, the shared-source cache counters and byte-budget refusal details.

Run the external import check from the source checkout. Fourteen neutral fixtures are included in Git. To add the five product files, pass `--appearance-runs /path/to/runs` containing each product’s `cadquery/r1/model.step`; match the published source hashes when reproducing the exact 19-file comparison.

```sh
"$PY" benchmarks/readme/stable/verify-step-imports.py "$OUT/package/kernel/native/linux-arm64/agent-cad" "$D/step-roundtrips.json"
```

Use the native executable for the host running this command.

## Three-backend history for 0.1.2

Run from the source checkout, using an extracted 0.1.2 package with the matching native catalog in `$PKG`. Set `$PY` to Python 3.12 with OCP, NumPy, Matplotlib and Pillow. The host-side history checker uses Node and the matching host executable in the checkout. Each backend must finish and pass the independent verifier before generating the report.

```sh
"$PY" benchmarks/readme/stable/history.py "$OUT/history" --runtime "$PKG" --storage segmented --cpu 0
"$PY" benchmarks/readme/stable/revision-release.py "$OUT/history" "$PKG" "$OUT/report"
```

The report writes a self-contained HTML page, the animated memory chart, the edit-128 growth plot, and the per-trial measurements. It verifies the session descriptor for the segmented run and records the runtime file digests. Keep the generated STEP and session files until the geometry, archive and chart checks have passed. The published 0.1.2 dataset covers this revision workload. The mechanical matrix and product studies have separate versioned datasets.
