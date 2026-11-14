# Five product models on macOS arm64

The November 14, 2026 study compared Colocad 0.1.0-alpha.11-rc.1 with CadQuery 2.8.0. Release 0.1.0 contains the same SDK, CLI and native binaries. A separate clean install of the public release built the packaged manifold. [中文](benchmark-appearance.zh-CN.md) · [Package comparison](https://github.com/Coloopx/colocad/releases/download/v0.1.0/release-validation.json) · [Public install receipt](https://github.com/Coloopx/colocad/releases/download/v0.1.0/registry-proof.json).

## What ran

Five concept images, product definitions and parts lists matched the original 15 file hashes. The modeling plan for each product was reconstructed from those inputs and earlier output geometry. Colocad and CadQuery then executed the same plan, including the same circular substitution for the SenseCAP base. Inference, retries and visual review belonged to the earlier end-to-end experiment and were outside these timers.

| Execution mode | Start and finish of the timer | Memory measurement |
| --- | --- | --- |
| Fresh process | Start Node or Python, load dependencies and model, validate geometry, export STEP, exit. OS file caches were warm. | Sum of resident memory across the process tree sampled about every 10 ms. The published value is the median of three run peaks. |
| Loaded host | Start from a parsed plan with the SDK or Python module loaded. Rebuild the whole product and export to a new directory. Colocad starts its native process on each SDK build. Both backends rebuild geometry. | Time only. |

Each product and backend ran once for warm-up and three times for measurement in each mode. The fresh runs alternated backend order. OCCT independently reopened all 80 STEP files outside the timed interval. The machine imposed no Linux cgroup limit. [Per-run times, fresh RSS peaks, input identities and STEP hashes](benchmarks/appearance-0.1.0/measurements.json).

## Results

On these products a fresh Colocad process peaked at 69–83 MiB and a fresh CadQuery process at 481–495 MiB. The smallest timing gap was Mori at 2.24 s and 2.29 s respectively. After loading the host, CadQuery completed all five rebuilds 4.5–8.6 times faster, including Mori at 0.38 s versus Colocad at 2.12 s. [Five-row comparison and STEP views](../README.md#five-product-appearance-models).

Together the five models contain 67 named solids. The readback checked the expected solid counts and validity, plus 23 material/air probe points. Of 139 specified part dimensions, 123 could be mapped to individual solids. Of those, 109 were within the study threshold of 0.25 mm and 14 were outside it. The other 16 lacked an independently measurable part dimension. This threshold was used for comparing digital shapes. StackChan’s screen expression and top print are absent from the STEP, and Mori retains dimensional differences.

## SenseCAP base

The specified base has an elliptical section of 49.9858 × 49.963 mm and an extrusion depth of 12.0991 mm. Its minimal Colocad model returns `GEOMETRY_VALIDATION_FAILED` under native solid validation. CadQuery produced a valid exact solid. Using a circle of diameter 49.9858 mm changes the minor dimension by 0.0228 mm. That geometry is used in both timed runs. The isolated ellipse also failed on Colocad alpha.9 and alpha.10. [Minimal exact input](benchmarks/appearance-0.1.0/mount-base-ellipse.json) · [Regression outcomes](benchmarks/appearance-0.1.0/measurements.json).

To reproduce the exact case, download the input above as `mount-base-ellipse.json` in a Node.js 22+ project with `@coloopx/colocad@0.1.0` installed. Use a fresh output directory.

```sh
node node_modules/@coloopx/colocad/skills/colocad/scripts/build.mjs \
  mount-base-ellipse.json ./ellipse-check
```

For budgets in a Linux agent sandbox, use the separate study that measured cgroup memory with one CPU per task. [Mechanical cases and retained revisions](benchmark-alpha11-rc1.md).
