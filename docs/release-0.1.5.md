# Colocad 0.1.5

- Reconstruct valid faces whose supporting surface rescales the UV coordinates. The 20 × 20 × 6 mm drilled plate retains its 1 mm bore and 2381.15041 mm³ volume. [PR #3](https://github.com/Coloopx/colocad/pull/3)
- Import explicitly bound, equivalent-length STEP representation contexts. The Mushroom product imports with ten solids. [PR #8](https://github.com/Coloopx/colocad/pull/8)
- Preserve the outward face sense when exporting reflected spherical trims. The mirrored filleted block and the hollow capsule both import back as single solids. The capsule retains its enclosed void. [PR #25](https://github.com/Coloopx/colocad/pull/25)
- Test closed Boolean union and intersection results against necessary spatial bounds. The pipe/sphere example now returns a structured geometry failure before writing an impossible intersection. This guard also applies after updating a saved session. [PR #26](https://github.com/Coloopx/colocad/pull/26)
- Remove zero-support endpoint poles when reconstructing clamped STEP B-splines. Four endpoint variants pass ordinary import, definition import and inspection-template replay, including export and reimport. [PR #34](https://github.com/Coloopx/colocad/pull/34)

## Verification

The comparison used the published 0.1.4 native binary, a same-toolchain rebuild of its main commit and the final 0.1.5 distribution binary. Each shuffled round ran identical JSON through fresh native processes. One warm-up round was excluded. Time includes process startup and output. Peak RSS includes resident executable pages.

| Workload | 0.1.4 median ms | 0.1.5 median ms | 0.1.4 peak MiB | 0.1.5 peak MiB | Paired rounds |
|---|---:|---:|---:|---:|---:|
| kk60-ordinary | 73.23 | 72.76 | 16.844 | 14.828 | 25 |
| kk60-definition | 72.74 | 72.70 | 16.828 | 14.766 | 25 |
| invalid-gap | 8.07 | 8.07 | 10.078 | 8.844 | 25 |
| manifold | 224.69 | 220.50 | 23.188 | 20.938 | 13 |
| flipper-build | 407.33 | 398.30 | 29.859 | 27.703 | 13 |
| six-spline-bores | 279.91 | 254.93 | 20.156 | 18.312 | 13 |
| watcher-build | 479.45 | 465.61 | 28.531 | 26.656 | 13 |
| drilled32 | 578.68 | 557.15 | 28.469 | 27.469 | 13 |
| polygon360 | 350.52 | 329.64 | 47.125 | 46.344 | 13 |
| union18 | 47.23 | 46.78 | 18.812 | 17.328 | 13 |
| watcher-inspect | 19.52 | 19.58 | 9.828 | 9.359 | 13 |
| polygon-inspect | 39.06 | 39.33 | 19.547 | 19.203 | 13 |
| rounded | 187.93 | 185.55 | 24.031 | 22.094 | 13 |

On Linux arm64, the KK60 import ran 30 paired rounds after one warm-up in a 1 vCPU, 512 MiB sandbox with swap disabled. The baseline control reran the same 0.1.4 executable. The table reports process peak RSS and elapsed time.

| KK60 mode | 0.1.4 median ms | 0.1.5 median ms | 0.1.4 peak MiB | 0.1.5 peak MiB |
|---|---:|---:|---:|---:|
| ordinary | 73.68 | 70.59 | 22.000 | 19.125 |
| definition | 72.97 | 69.29 | 22.125 | 19.250 |

The distribution profile uses whole-program LLVM link-time optimization. The candidate macOS and Linux arm64 binaries were benchmarked with this profile. The tested B-rep tolerances and rejection conditions are retained.

Of five product STEP assemblies, StackChan, SenseCAP Watcher, Mushroom and Flipper Zero passed independent geometry checks. Mori returned `IMPORT_INCOMPLETE` on both 0.1.4 and 0.1.5.

Two additional noncollinear near-end spline stress inputs remain unsupported. The four endpoint fixtures above describe the verified scope.

## Linux assembly and part workloads

Each row is the median of eleven measured runs per binary. Fresh processes used a single pinned vCPU with 512 MiB and no swap. The run order was shuffled in each round. Each successful build also had to return a valid solid and write STEP. The invalid-gap case had to refuse delivery.

| Workload | 0.1.4 median ms | 0.1.5 median ms | 0.1.4 peak MiB | 0.1.5 peak MiB |
|---|---:|---:|---:|---:|
| drilled32 | 496.86 | 509.60 | 28.984 | 26.164 |
| flipper-build | 368.67 | 359.03 | 28.410 | 25.574 |
| invalid-gap | 13.36 | 12.38 | 17.270 | 17.270 |
| manifold | 221.45 | 209.02 | 26.250 | 22.969 |
| polygon360 | 308.25 | 271.15 | 36.711 | 33.379 |
| six-spline-bores | 279.48 | 244.55 | 24.500 | 21.875 |
| union18 | 53.38 | 49.03 | 23.184 | 20.449 |
| watcher-build | 442.10 | 396.42 | 27.926 | 24.969 |

The source and exported STEP files were compared in Open CASCADE for B-rep validity, solid counts, bounds and volume. Source bounds had to agree within 0.0001 mm, and volume within the larger of 0.0001 mm³ or one part per million.

The distribution checks cover Node 22 on macOS arm64, Linux arm64 and Linux x64. The Linux arm64 session and the repeated KK60 test run with one vCPU and 512 MiB. The reproduction archive includes models, a Node replay and an independent OCCT verifier. [Validation data](https://github.com/Coloopx/colocad/releases/download/v0.1.5/release-validation.json) · [Reproduction archive](https://github.com/Coloopx/colocad/releases/download/v0.1.5/step-reproduction.zip)

Install with `npm install --save-exact @coloopx/colocad@0.1.5`.
