# Reproduce the November 13 candidate rerun

The measured package is `@coloopx/colocad@0.1.0-alpha.11-rc.1` from source commit `01079a9`. Its tarball and executable digests are recorded in the [observations](../../../docs/benchmarks/alpha11-rc1/observations.json). For checkout-based programs use matching platform binaries, SDK, CLI and skill entry points. Check `node scripts/check-release.mjs` before starting. The Linux arm64 image is `colocad-memory:20261111`; the independent verifier and graphics scripts use a separate Python environment with OCCT, build123d, NumPy, Pillow and Matplotlib.

Set `OUT` to a new absolute experiment directory. Set `BASE` and `RC` to the unpacked public alpha.10 and local candidate npm packages. Run the following from the repository root.

1. Use steps 1, 2 and 4 in the [release benchmark](../../release/README.md) for the frozen 108 scaling jobs, 99 fixtures and three 100-second hand windows. Each workload has one CPU; this campaign used CPUs 0/1 for the interleaved halves of the original scaling plan, 2 for fixtures and 3 for queues. Join scaling rows in original plan order before running the verifier. A serial run of the unmodified plan exercises the same cases. Keep all output and verified JSON.
2. Run the four persistent workflows and ten work orders below. Each persistent workflow uses a fresh Docker volume. The Python adapters persist the input recipe, editable parameters, STEP, triangle preview and indexed hashes, fsync the files, and advise clean exchange pages out of cache. Each backend reads six earlier revisions after the write phase. The same OCCT verifier checks each of the 256 edited STEPs per workflow. The flange verifier checks real contact, overlap, volume and all four mounting axes.
3. Rebuild the 16 KK60 stages via the packaged `examples/hiwin-kk60/run.mjs "$OUT/kk60"`. Run the [independent verifier](../../../examples/hiwin-kk60/verify.py) on its session into `$OUT/kk60-readback.json`. Render `$OUT/cover.gif` and `$OUT/cover.mp4` using `node bin/colocad.mjs render "$OUT/kk60/session" OUTPUT --width 960 --height 600 --fps 8 --seconds-per-step .75`.
4. Repeat the [six macOS shapes](../../shapes/README.md) in `$OUT/shapes` and run `specs/experiments/0005/run.py` for `regression` and `held_out` in `$OUT/g2-regression` and `$OUT/g2-held_out`, respectively. Keep their independent checks.
5. Record a `campaign.json` in the trial root with measured version, runtime source commit, actual package digest, image and native executable digests, and the resource setup. The [packed runtime identity](../../../docs/benchmarks/alpha11-rc1/observations.json) is under `campaign`. Run the final publisher below only after all verifiers exit.

```sh
python3 benchmarks/field-task/run-comparison.py "$OUT/field" --candidate "$RC" --baseline "$BASE" --cpu 4
for runtime in alpha10 cadquery colocad build123d; do
  python3 benchmarks/field-task/verify-comparison.py "$OUT/field" "$runtime"
done
python3 -c 'import sys,json; print(json.dumps(json.load(open(sys.argv[1]))["edits"]))' \
  "$OUT/field/colocad-checked.json" > "$OUT/field/native-oracle.json"
for runtime in alpha10 colocad; do
  node benchmarks/field-task/check-revisions.mjs "$OUT/field/$runtime" \
    "$OUT/field/native-oracle.json" "$OUT/field/$runtime-archive.json"
done
python3 benchmarks/readme/candidate/run-flanges.py "$OUT/flange" --cpu 5
python3 benchmarks/readme/candidate/publish.py "$OUT" docs/benchmarks/alpha11-rc1
```

The publisher rebuilds the scorecard, cover, engineering GIF and memory GIF from saved outputs. The scorecard embeds the hand replay, work-order and history replays, six paired shape images and downloadable measurements. Animation time follows work orders or revision counts except for the explicitly marked 10× timed hand replay. The shell examples use Python only to run experiments and independent checks; normal Colocad usage needs Node and its bundled native binaries.
