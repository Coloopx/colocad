# Models, revisions and artifacts

## Two input formats

| | Feature tree | Kernel model |
| --- | --- | --- |
| Discriminator | `format: "colocad/1"` | `version: 1` |
| Units | `mm` | `mm` |
| Entry point | `build(tree, newDirectory)` / `colocad build` | `CadClient` / skill `scripts/build.mjs` |
| Operations | Extrude, cut, Z-plane fillet, X/Y linear pattern | Wider native feature set, sketches, imported STEP, assemblies and previews |
| Output selection | Ordered part history | Explicit `output` feature or named `assembly` occurrences |
| Parametric source | `feature-tree.json` | `model.json` |
| Contract | [Type declarations](../src/index.d.mts) | [JSON Schema](../kernel/schemas/model.schema.json), `cad.schema()`, `cad.capabilities()` |

Do not send a kernel model to `colocad build`, or wrap a feature tree as the `model` argument of `cad_build`. Use `kernelModel(tree)` to compile the feature tree to a kernel model when moving to the richer SDK.

The two profile syntaxes also differ: a feature-tree rectangle uses `center` and `size`; kernel sketch profiles follow the kernel schema. Feature-tree scalars are numbers or `{ "param": "thickness" }`. Kernel scalars additionally support arithmetic expressions such as `{ "op": "div", "a": { "param": "diameter" }, "b": 2 }`.

## Coordinates and identity

The feature tree extrudes or cuts along +Z from `startZ` (default 0). Kernel boxes use a lower-corner origin; cylinders start at their base center along +Z. Other sketch planes and frames must be explicit. Choose a datum and units before translating a drawing.

Feature IDs identify dependencies and checkpoints. Keep them stable when changing a dimension. In an assembly, define a repeated part once and create named occurrences with placements. Keep moving links separate; union only material that forms one physical solid. The [hand example](../showcase/tendon-hand/README.md) demonstrates this distinction.

## Revision ownership

A successful `CadClient.build` returns a new `artifact_directory`. One sequential `CadClient` remembers its previous successful revision for checkpoint reuse. A new client can use `build(model, previousRevisionDirectory)`. The harness owns the accepted source and artifact record; the client is not a persistent project database.

`edit(model, patch)` returns a new revision and does **not** mutate `model`. After acceptance, save the returned `model.json` or apply the patch to your owned source. Otherwise a later edit can accidentally revert an earlier change. Retain the source, its receipt, tool version and artifact hashes together.

For an unchanged model, `preview()` can reuse a saved mesh. `observeArtifact(directory, options)` also works across SDK processes with a trusted saved preview directory. An observation directory contains display output; its `source_artifact_directory` points to the geometry revision. Keep that source when handing off STEP or pruning old files.

## What editable means

JSON parameters and feature definitions can be edited and rebuilt. Exact STEP contains boundary geometry, not the original driving feature history. GLB/STL/PNG are previews. Imported STEP is usable geometry; import does not reconstruct its original CAD history.

Generic editable SOLIDWORKS output is a separate experimental Windows author. It requires the installed application and a license, and licensed native rebuild verification is still outstanding. Linux STEP success is not evidence of native `.SLDPRT` feature-tree fidelity. See [validation](validation.md).
