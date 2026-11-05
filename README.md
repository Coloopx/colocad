# colocad

Local parametric CAD for agents. A TypeScript-compatible API compiles ordered sketch extrusions, cuts, constant-radius fillets and linear feature patterns. A Rust B-rep kernel executes the graph inside the calling agent's sandbox. Node.js 22+.

**Alpha status.** Linux supports instruction-to-STEP, mesh previews, parameter edits and geometry verification. Direct editable SLDPRT generation on Linux is under development. A separate local Windows author uses the native SOLIDWORKS FeatureManager. It compiles against the 2024 interop API and awaits a licensed SOLIDWORKS runtime test. The Linux CLI returns `NATIVE_AUTHOR_UNAVAILABLE` for the native command.

## Run the examples

```sh
npm install @coloopx/colocad@alpha
npx colocad build node_modules/@coloopx/colocad/examples/manifold.json ./manifold --preview
npx colocad build node_modules/@coloopx/colocad/examples/sensor-rail.json ./rail
```

The manifold contains a six-sided extrusion, a blind circular cut, an upper-rim fillet, a four-instance pattern referencing the earlier cut, and a rectangular pocket. The sensor rail uses a different sketch, feature order and pattern direction. Both run through the same compiler. The pattern count includes the seed.

Each call requires a new output directory. The `part/` subdirectory holds `model.step`, `feature-tree.json`, a kernel report, and `native-job.json`. With `--preview` it also contains PNG, STL and GLB files. The native job stores the ordered feature definitions, resolved inputs and reference geometry measurements.

```js
import { readFile } from 'node:fs/promises';
import { build } from '@coloopx/colocad';

const tree = JSON.parse(await readFile('./node_modules/@coloopx/colocad/examples/manifold.json', 'utf8'));
await build(tree, './original');
tree.parameters.instances = 3;
const edited = await build(tree, './edited');
console.log(edited.geometry.volume_mm3);
```

All coordinates and lengths are in mm. The first extrusion creates a solid. Further extrusions add material and cuts remove material along +Z from a sketch at `startZ` (default 0). Profiles can be rectangles, circles or polygons. Fillets select edges on a specified Z plane and require the declared match count. A linear pattern refers to an earlier extrusion or cut by ID, with X/Y direction, positive pitch and instance count. IDs are unique, case-insensitively. A scalar is a finite number or a reference such as `{"param":"thickness"}`. The public TypeScript declaration defines the full input.

The kernel API at `@coloopx/colocad/kernel` also exposes the lower-level geometry operations, STEP import, incremental revisions and preview cache. The npm tarball includes Linux arm64/x64 and macOS arm64 executables and dependency licenses. The checked-in sources build the kernel on supported toolchains.

## Native SOLIDWORKS author (experimental)

The Windows entry point consumes the same job. It starts a blank part using the application's configured default, builds sketches with native lines and circles, calls extrusion/cut/fillet/pattern APIs and names the resulting features. The fillet uses body edges and the native pattern selects the original seed feature. There are no part-specific model files. Running it requires local Windows, .NET 8 and a licensed SOLIDWORKS installation with a valid default blank part configuration.

```powershell
colocad solidworks .\manifold\part\native-job.json .\manifold.SLDPRT
```

Use `COLOCAD_SW_INTEROP` for an alternate directory containing the local SOLIDWORKS interop DLLs. The completion check is designed to save a temporary part, reopen it, verify native types, radius, pattern seed, count, pitch and reference volume, then edit a driving extrusion dimension, rebuild, restore and check it. Only a successful check publishes the output. The current release has **no native runtime pass** for either complex part. The package includes no four-hole template and does not route jobs to another machine.

## Build and verify from source

```sh
npm run build:kernel
npm test
node bin/colocad.mjs build examples/manifold.json ./out/manifold --preview
```

The first command fetches the pinned SolidCore archive, checks its SHA-256 and builds the local Rust executable. The plain Node tests use the local executable or `AGENT_CAD_BINARY`. The repository includes the TypeScript kernel SDK source and its compiled distribution. To rebuild the .NET author, build `solidworks/Colocad.SolidWorks.csproj` with a .NET 8 SDK and `-p:SWInteropDir=<directory containing the two SOLIDWORKS interop DLLs>`. Copy only Colocad.SolidWorks.dll, its deps.json and runtimeconfig.json to solidworks/managed.

Independent OCCT validation is in `test/validation/geometry.py`. It reopens the STEP and checks both volume and whether selected points lie inside the plate, the holes, the recess or the fillet. The measured results and the limits of the native experiment are in [the validation notes](docs/validation.md).
