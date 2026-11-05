# Reproduce the README figures

The [recorded runs](../../docs/benchmarks/runs.json) used the public **@coloopx/colocad 0.1.0-alpha.2** tarball, build123d 0.11.1 and OCP 7.9.3.1.1 on Linux arm64. See that file for the archive and image digests and interpreter versions. Both backends read the *same* JSON fixture from the tarball. The [Python baseline](baseline.py) builds its own sketches, extrusion, fillet and cutters; it batches pattern copies in a single boolean. It never imports colocad geometry. The colocad arm runs the actual npm CLI, including Node and the bundled Rust executable.

## What the numbers mean

- **Time to STEP:** external wall clock inside each container, around one fresh CAD process. Includes import or CLI startup, modeling, each backend’s own validation, STEP export and process exit. The colocad path also writes its normal revision artifacts. Excludes installation, container launch, independent readback, rendering and agent inference.
- **Geometry only:** instrumented Rust kernel geometry phase or build123d modeling phase until the part is constructed, before validation/export. These are implementation phases, not isolated timings for equivalent algorithms.
- **Memory:** the largest cgroup v2 `memory.peak` over the five measured fresh containers. Includes the small measurement process, CAD process(es), charged file pages and tmpfs outputs. Warm or shared pages may be charged elsewhere. This is not per-process RSS and does not include a resident agent, harness, or LLM. The largest excluded warm-up reached 485.7 MiB on build123d.
- **Protocol:** Linux arm64 on a shared development machine; 1 vCPU, 512 MiB RAM, no swap, no network, read-only root, UID 1000. Identical inputs, image, thread settings and tmpfs for both arms. One excluded warm-up per fixture/backend, then five runs for each; serial execution with alternating order. Each trial has a fresh process **and** cgroup, with warm host caches. For example, measured build123d rail wall times ranged from 2.21 to 6.65 s. Five samples do not characterize tails or a resident Python interpreter.
- **Geometry and pictures:** [check_step.py](check_step.py) reopens the first measured STEP per arm after timing, checks validity, solid count, bounding box, integrated volume and both directions of OCCT boolean difference. Bounds must agree within 0.0001 mm; relative volume and symmetric difference must be below 1e-6. [figures.py](figures.py) tessellates the four exports with identical OCCT settings (0.06 / 0.08) and uses the same orthographic camera, scale, material and lighting within each row. The manifold is viewed from below to expose blind holes.
- **Changing the view:** a separate, colocad-only SDK experiment on the same tarball, CPU and memory budget. Start with an isometric preview. For a top view, compare a new full kernel build against `observeArtifact` on the saved mesh. Each call uses a new SDK client and local Rust process; Node is already running. One warm-up and five alternating pairs per part. PNG, GLB and STL must match by SHA-256 for each pair. This measures the whole camera change, not just tessellation; it does not measure a geometry edit or compare against build123d. [Full receipts](../../docs/benchmarks/preview-runs.json).

## Run it

From the repository root, obtain the exact archive. The archive SHA-256 for the recorded run is `f7d255b09b638ef6109a3fa45fe9076ebc92502e5abc31461c495c53e4fe0891`.

```sh
npm pack @coloopx/colocad@0.1.0-alpha.2
docker build -t colocad-bench -f benchmarks/readme/Dockerfile.public benchmarks/readme
python3 benchmarks/readme/run.py --image colocad-bench --python /usr/local/bin/python3 \
  --tarball coloopx-colocad-0.1.0-alpha.2.tgz --out out/readme-trials
```

Docker needs Linux cgroup v2. The public Dockerfile is an alternative base; results will vary. The original run used the existing Abelien runner image on arm64. The script adds the exact npm tarball and measurement scripts before starting the isolated trials. It refuses to overwrite an existing output directory. Rep 0 is the excluded warm-up. The four `.step` files in the output directory are from rep 1.

To repeat the optional readback and figures on a machine with Python 3.12:

```sh
python3 -m venv .venv-eval
.venv-eval/bin/pip install 'build123d==0.11.1' 'cadquery-ocp-novtk==7.9.3.1.1' numpy pillow matplotlib
.venv-eval/bin/python benchmarks/readme/check_step.py out/readme-trials > out/step-checks.json
.venv-eval/bin/python benchmarks/readme/figures.py out/readme-trials out/figures
```

To reproduce the preview comparison with the same staged tarball, use a clean directory as a Docker build context:

```sh
mkdir -p out/preview-image
cp coloopx-colocad-0.1.0-alpha.2.tgz out/preview-image/bundle.tgz
cp benchmarks/readme/preview.mjs out/preview-image/
printf 'FROM colocad-bench\nADD bundle.tgz /opt/\nCOPY preview.mjs /opt/bench/\n' > out/preview-image/Dockerfile
docker build --network=none -t colocad-bench-preview out/preview-image
docker run --rm --network none --cpus 1 --memory 512m --memory-swap 512m --pids-limit 64 \
  --read-only --user 1000:1000 --cap-drop ALL --security-opt no-new-privileges \
  --tmpfs /tmp:rw,nosuid,size=128m -e RAYON_NUM_THREADS=1 --entrypoint node \
  colocad-bench-preview /opt/bench/preview.mjs > out/preview-runs.json
```
