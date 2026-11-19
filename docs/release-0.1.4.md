# Colocad 0.1.4

This patch fixes STEP imports of cylindrical faces with finite axial extent. In 0.1.3 a change of loop start or loop direction could extend a reconstructed seam beyond the trim and cause a valid part to fail validation. The importer now applies the face's finite parameter domain to the supporting cylinder before constructing the seam. The SDK and CLI use this path for STEP imports. [Implementation and regression tests](https://github.com/Coloopx/colocad/pull/31)

## Geometry

The test part has outer radius 2 mm, bore radius 1 mm and thickness 0.025 mm. Two angular placements, four loop starts and two traversal directions give 16 STEP inputs representing the same annulus. Version 0.1.3 imported eight of these inputs. Version 0.1.4 imports all 16 and reimports their STEP exports. Each output has one solid, four faces and six edges. A 0.02 mm vertex displacement still triggers rejection.

The release package is tested on macOS arm64, Linux arm64 and Linux x64. Independent OCCT readback checks solid validity, the expected volume of 0.235619449 mm³ and the 4 × 4 × 0.025 mm envelope. Point-in-solid tests check that the bore is empty and that the annular wall contains material. The release also exercises CLI session recording, editing and historical checkout on all three platforms. [Release results](https://github.com/Coloopx/colocad/releases/download/v0.1.4/release-validation.json) · [STEP fixtures and replay scripts](https://github.com/Coloopx/colocad/releases/download/v0.1.4/step-annulus-reproduction.zip)

## Performance scope

On an Apple M3 Max, 25 measured rounds per entry point ran the npm 0.1.3 binary, a fresh build of the same main commit and the 0.1.4 macOS kernel in shuffled order. Each observation starts a new native process and builds the same KK60 bracket in a fresh directory. The table gives medians.

| KK60 import | npm 0.1.3 | Rebuilt 0.1.3 | 0.1.4 |
|---|---:|---:|---:|
| Ordinary, elapsed ms | 73.35 | 72.78 | 72.31 |
| Ordinary, peak RSS MiB | 16.969 | 16.953 | 16.891 |
| Definition, elapsed ms | 72.05 | 71.58 | 71.98 |
| Definition, peak RSS MiB | 16.906 | 16.875 | 16.844 |

The measured improvement is successful geometry delivery across seam representations. The KK60 control measures per-process costs on this host. The earlier 256-revision sandbox measurements remain attributed to the version on which they ran. [Paired measurements](https://github.com/Coloopx/colocad/releases/download/v0.1.4/release-validation.json) · [0.1.3 long-session benchmark](release-0.1.3.md)
