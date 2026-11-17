# Five product models on macOS arm64

On November 17, 2026 the published npm package **Colocad 0.1.1** and **CadQuery 2.8.0** reran both execution modes. The 15 source-file hashes were checked against the supplied input archive. [中文](benchmark-appearance.zh-CN.md) · [Runtime fingerprints](benchmarks/stable-0.1.1/observations.json).

## What ran

Five concept images, product definitions and parts lists matched the original 15 file hashes. The modeling plan for each product was reconstructed from those inputs and earlier output geometry. Colocad and CadQuery then executed the same plan, including the same circular substitution for the SenseCAP base. Inference, retries and visual review belonged to the earlier end-to-end experiment and were outside these timers.

| Execution mode | Start and finish of the timer | Memory measurement |
| --- | --- | --- |
| Fresh process | Start Node or Python, load dependencies and model, validate geometry, export STEP, exit. OS file caches were warm. | Sum of resident memory across the process tree sampled about every 10 ms. The published value is the median of three run peaks. |
| Loaded host | Start from a parsed plan with the SDK or Python module loaded. Rebuild the whole product and export to a new directory. Colocad starts its native process on each SDK build. Both backends rebuild geometry. | Time only. |

Each product and backend ran once for warm-up and three times for measurement in each mode. The fresh runs alternated backend order. OCCT independently reopened all 80 STEP files outside the timed interval. The machine imposed no Linux cgroup limit. [Per-run times, fresh RSS peaks, input identities and STEP hashes](benchmarks/stable-0.1.1/appearance.json).

## Results

On macOS arm64, median fresh process-tree peak RSS ranged from **71–84 MiB** for Colocad 0.1.1 to **479–492 MiB** for CadQuery 2.8.0. Colocad completed **5/5** fresh builds sooner. With each host already loaded, CadQuery rebuilt **5/5** sooner. [Five-row comparison](../README.md#five-product-appearance-models).

Together the five models contain 67 named solids. The readback checked the expected solid counts and validity, plus 23 material/air probe points. Of 139 specified part dimensions, 123 could be mapped to individual solids. Of those, 109 were within the study threshold of 0.25 mm and 14 were outside it. The other 16 lacked an independently measurable part dimension. This threshold was used for comparing digital shapes. StackChan’s screen expression and top print are absent from the STEP, and Mori retains dimensional differences.

## SenseCAP base

The specified base has an elliptical section of 49.9858 × 49.963 mm and an extrusion depth of 12.0991 mm. Its minimal Colocad model returns `GEOMETRY_VALIDATION_FAILED` under native solid validation. CadQuery produced a valid exact solid. Using a circle of diameter 49.9858 mm changes the minor dimension by 0.0228 mm. That geometry is used in both timed runs. [Minimal exact input](benchmarks/stable-0.1.1/mount-base-ellipse.json) · [Regression outcomes](benchmarks/stable-0.1.1/appearance.json).

To reproduce the exact case, download the input above as `mount-base-ellipse.json` in a Node.js 22+ project with `@coloopx/colocad@0.1.1` installed. Use a fresh output directory.

```sh
node node_modules/@coloopx/colocad/skills/colocad/scripts/build.mjs \
  mount-base-ellipse.json ./ellipse-check
```

For budgets in a Linux agent sandbox, use the separate study that measured cgroup memory with one CPU per task. [Mechanical cases and retained revisions](benchmark-stable.md).
