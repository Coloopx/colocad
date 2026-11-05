# Replay one fresh call to each CAD backend

The film uses the four-port manifold from the published npm package. After one warm-up per backend we recorded the first fresh call from each. The two jobs ran serially under the same 1 vCPU / 512 MiB limit and their wall-clock origins are aligned in the film. These are the same CAD entry points used for the [five-run benchmark](../README.md). The main chart uses the new recorded pair. The end card gives the five-run results separately.

The timer starts just before launching the tool and ends on process exit after STEP export. A thread in the measurement process samples the whole container’s cgroup v2 `memory.current` about every 20 ms. Scheduling can vary. The plot uses the recorded timestamps, with each sample held until the next. Sampling ends with the tool call. The final peak label uses `memory.peak`, which can capture a peak between samples. The sampler, tool, descendants, charged file pages and temporary files all count towards container memory. Package installation, container setup, later readback and rendering occur outside the timer.

Both resulting STEPs were opened in OCCT. Each has one valid solid with 27 faces and 54 edges and zero measured material difference in both boolean directions at 0.000001 mm fuzzy tolerance. The input drawing appears while the tool runs and the actual STEP readback appears on the first 25 fps frame at or after delivery. Intro and result cards have separate reading time. The measured interval plays at 1×.

## Reproduce

From the repository root, use the existing benchmark image with Node 22 and build123d 0.11.1 installed, or build the [public evaluation image](../Dockerfile.public). Run the following with Python 3.12. Local preparation and rendering require the evaluation dependencies from the [benchmark instructions](../README.md), FFmpeg, and Hiragino Sans GB or Noto Sans CJK SC for the Chinese text.

```sh
npm pack @coloopx/colocad@0.1.0-alpha.2
python3 benchmarks/readme/replay/trace.py record colocad-bench coloopx-colocad-0.1.0-alpha.2.tgz out/trace --python /usr/local/bin/python3
.venv-eval/bin/python benchmarks/readme/replay/prepare.py out/trace
.venv-eval/bin/python benchmarks/readme/replay/animate.py out/trace out/animation
cp out/trace/trace.json out/trace/readback.json out/animation/
python3 benchmarks/readme/replay/offline.py out/trace out/animation
```

The recorded image was the existing Abelien arm64 runner with its built-in Python environment. Its digest and the tarball hash are in the [raw trace](../../../docs/benchmarks/replay/trace.json). The [single-file replay](https://github.com/Coloopx/colocad/raw/refs/heads/main/docs/benchmarks/replay/manifold-replay.html) includes the video, the raw time series, the OCCT checks and both recorded STEP files.
