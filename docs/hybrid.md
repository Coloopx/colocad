# Sculpted parts and precision mechanisms

An agent can keep a generated sculpture as a triangle solid, design a mounting interface as a B-rep, and assemble both in millimetres. A second option is to model the housing itself with editable spline sections. Select the local hybrid executor by setting `format` to `colocad/hybrid/1`. It loads OCCT WebAssembly for exact operations and Manifold WebAssembly for triangle operations in the calling process. The mechanical feature-tree and kernel-model entry points continue to use Rust/SolidCore.

## Run the example

```sh
npm install @coloopx/colocad@0.1.0-alpha.10
npx colocad build node_modules/@coloopx/colocad/examples/hybrid-enclosure/model.json ./housing-build
```

Open `housing-build/review.html` locally. Inspect the poses, part colours and cutaway with the offline viewer. This example records one static pose per build. Change the lever angle in the source and build into a fresh directory to check a second pose.

```js
import {readFile} from 'node:fs/promises';
import {dirname} from 'node:path';
import {buildHybrid} from '@coloopx/colocad/hybrid';

const file = './design/model.json';
const spec = JSON.parse(await readFile(file, 'utf8'));
const report = await buildHybrid(spec, './build-15deg', {assetRoot: dirname(file)});
if (!report.ok) console.error(report.checks, report.inspections);
```

`build` at the package root also accepts this document. The machine-readable contract is exported as `@coloopx/colocad/hybrid/schema`. Inputs use project-relative paths. An existing output directory remains intact. A completed build with a failed engineering check retains its geometry and report, and the CLI exits with status 1. A kernel or import error also exits with status 1 and identifies the failed feature.

## Features and materials

| Input or operation | Contract |
| --- | --- |
| `profile_loft` | Each station is a planar ring of at least four distinct XYZ landmarks. Use the same start landmark, winding and landmark count at each station. Interpolating cubic Bézier spans share tangents around the ring. Smooth loft interpolation has maximum degree three along the stations. |
| `shell` and `hollow` | Positive inward normal offset in mm. For an open shell, `openFace: {plane:"XY",at:0}` must select one planar cap. XY, XZ and YZ use the world Z, Y and X coordinate respectively. A closed hollow subtracts the inward-offset core. Each resulting B-rep must pass OCCT topology checks. Tight concavities can prevent an offset. |
| `cut`, `union`, `intersect` | Exact solid operations including smooth loft against smooth loft. `import_step` brings in existing mechanical parts, including exports from the Rust kernel. `box`, `sphere`, `cylinder` and `cone` provide stock, joints and optical envelopes. The cone starts at its tip and reaches `radius` at distance `height` along `axis`. |
| `import_mesh` | Binary/ASCII STL and polygonal OBJ require explicit mm, cm, m or in. Static uncompressed triangle GLB uses metres, applies its scene graph and converts Y-up to CAD Z-up. Base-colour factors and opacity survive GLB import. Textures, OBJ MTLs, vertex colours, animation and skinning are outside this material workflow. Bake those into an appropriate static asset or set the instance colour. |
| `mesh_cut`, `mesh_union`, `mesh_intersect` | Triangle-solid output. B-rep operands tessellate at the reported chord tolerance. Import requires a closed, consistently oriented manifold with positive signed volume after joining coincident seams. Invalid input fails. |
| `faceted_brep` | Explicitly turn a mesh into a B-rep with planar triangle faces for subsequent exact operations. Each independent outer shell is supported. Enclosed negative-volume cavity shells fail faceted STEP conversion. Cost increases with face count. For a scan needing only exchange use `facetedStep:true` and the streaming writer. |
| Placement and appearance | Instance rotation uses an axis, centre and angle in degrees, followed by translation in mm. Material overrides use `material:{color:"#339e76",opacity:0.8}`. The HTML and GLB both use these values. |

## Engineering checks

Pairwise checks reference instance IDs. Two B-reps use the volume of their exact Boolean common. A mixed pair uses Boolean common on the actual placed triangles. Both methods test containment. Zero overlap permits contact. A positive clearance requirement or a collision-free continuous sweep needs additional measurements. Tessellation can affect a close mixed result. Repeat at finer tolerance when the fit approaches that tolerance.

A `wall_probe` measures each material interval crossed by a segment whose endpoints are in air. Use a probe approximately normal to the wall and set an acceptable interval. A missed or ambiguous crossing fails the check. A `slice` reports the area and contour count at a world Z height. An `overhang` reports downward-facing triangle area steeper than the chosen threshold from vertical, above the stated print bed. These inspections sample the selected build orientation. A slicer trial remains necessary to evaluate bridges, actual supports, seams and printer settings.

## Delivered files

| File | Contents |
| --- | --- |
| `assembly.glb`, `review.html` | All posed instances, exposed materials and full display geometry. HTML works offline. |
| `INSTANCE.stl` | One posed part in mm for slicer import. Separate parts can intersect if checks failed. |
| `exact.step` | Every B-rep instance as a named, world-placed product root. |
| `INSTANCE.faceted.step`, `assembly.step` | With `facetedStep:true`, triangle bodies also get planar B-rep exchange. The combined file includes all exact and faceted product roots. It preserves static positions. Parametric history, mates and motion remain in the source document. |
| `report.json` | Representation and volume for each feature, triangle counts, overlap method and volume, inspection results and the path of the complete STEP when available. |

## Release regressions

The bundled fixture exercises a 1.6 mm offset shell, a subtraction between two smooth lofts, an articulated lever and an external mesh insert. Tests build 0°, 15° and 30° poses, then deliberately move the insert into the wall. An independent Python OCCT reader reopens both constituent STEP files and the mixed three-solid delivery.

The external stress fixture uses the original Stanford Armadillo range-scan reconstruction. The converter preserves its 172,974 vertices and 345,944 triangles when writing OBJ. It uses a 2 mm cube at a known surface vertex as the positive intersection control. The scan and test cubes use the declared mm scale for this experiment. The repository distributes the reproduction script and measurements. [Scan source](https://graphics.stanford.edu/data/3Dscanrep/) · [Release evidence](evidence/release-alpha.10.json) · [Full benchmark rerun](benchmark-alpha10.md)

```sh
python3 test/validation/hybrid-scan.py /tmp/colocad-scan
node bin/colocad.mjs build /tmp/colocad-scan/model.json /tmp/colocad-scan-build
```

The 512 MiB result applies to the measured one-CPU Linux cgroup and scan. The larger combined STEP writes a planar face for every triangle. A receiving CAD system can require substantially more resources to reopen that file.
