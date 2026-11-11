# Six shape controls on macOS arm64

The four-hole mounting plate, 64-hole plate, fillet, 270° revolve, electronics enclosure and open Bézier surface repeat the original macOS comparison. They use build123d 0.11.1 and the Colocad executable under review. The three-backend Linux benchmark uses different software and resource accounting.

From the repository root, copy `benchmarks/shapes` to a new output directory. Set `AGENT_CAD_BINARY` to the absolute path of the candidate executable. Use a Python environment with build123d 0.11.1 and NumPy.

```sh
export COLOCAD_BENCH_REVISION=$(git rev-parse HEAD)
cp -R benchmarks/shapes /tmp/colocad-shapes
mkdir /tmp/colocad-shapes/data
python3 /tmp/colocad-shapes/scripts/benchmark.py --python /absolute/path/to/venv/bin/python
/absolute/path/to/venv/bin/python /tmp/colocad-shapes/scripts/analyze.py
```

Each backend gets a warm-up and five fresh processes per case, alternating order. Timings are medians. The memory figure is the maximum process RSS observed over five runs. The retained first export from each backend undergoes common OCCT readback, including two-way Boolean differences for solids and sampled point differences for the open surface. The gallery renders those retained STEP imports with the same camera, tessellation and display scale.
