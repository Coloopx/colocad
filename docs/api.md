# SDK, CLI and tool reference

Targets `@coloopx/colocad@0.1.0-alpha.7`. The [public declarations](../src/index.d.mts) and [kernel declarations](../kernel/src/types.ts) are the complete type contracts.

## Feature-tree SDK and CLI

```js
import { build, compileTree, kernelModel } from '@coloopx/colocad';
const resolvedFeatures = compileTree(tree);
const model = kernelModel(tree);
const result = await build(tree, './new-part', { preview: true });
// result: { directory, step, nativeJob, geometry }
```

`build` accepts `preview?: boolean` and `binary?: string`. It rejects an existing destination and throws on failure. The directory contains a `part/` subdirectory. `compileTree` validates/resolves the feature instructions without building a solid.

```text
colocad plan TREE.json
colocad build TREE.json NEW_DIRECTORY [--preview]
colocad solidworks NATIVE-JOB.json NEW_FILE.SLDPRT
```

CLI success prints JSON to stdout. Failure prints an error to stderr and exits nonzero. Feature-tree CLI input is limited to 2 MiB. The third command is the [experimental Windows author](native-solidworks.md).

## Kernel SDK

```js
import { CadClient, createCadTools } from '@coloopx/colocad';
const cad = new CadClient({ outputRoot: '/workspace/cad-output', wallMs: 30000 });
const receipt = await cad.build(model);
if (!receipt.ok) throw new Error(JSON.stringify(receipt.error));
console.log(receipt.data.artifact_directory, receipt.data.geometry);
```

The host chooses `outputRoot`, a local directory for generated revisions, and optionally `binary`, an executable override. `wallMs` defaults to 30,000 and accepts 1–120,000 ms per native invocation. The directory is created as needed. A call runs a local Rust child process in the same sandbox. The SDK does not provision CPU or memory limits; the surrounding sandbox does that.

| Method | Meaning |
| --- | --- |
| `capabilities()` | Native version, supported operations, budgets and declared limitations |
| `schema()` | Complete kernel model JSON Schema |
| `validate(model)` | Cheap input/dependency/budget preflight; no geometric validity guarantee |
| `build(model, optionsOrRevision?)` | Build/check exact geometry; options include `queryable` and `previousRevision`; a legacy revision string remains supported |
| `querySchema()` | Exact-query JSON Schema from the installed native |
| `query(artifactDirectory, request)` | Read an exact snapshot using its `query_revision_id`, within host-configured `queryRoots` |
| `edit(model, parameters)` | Change existing numeric parameters, then build; input remains unchanged |
| `preview(model, options?)` | Build or reuse a matching display mesh and render it |
| `observeArtifact(directory, options?)` | Render from a trusted saved mesh with integrity checks |
| `inspect(artifactDirectory)` | Read `report.json`; returns a record and can throw on I/O failure |
| `solveSketch(sketch, parameters?)` | Solve constraints and report degrees of freedom/residuals |
| `inspectStep(model, assetId)` | Inspect registered STEP and return a model template; reuse `model.assets` when building it |

Apart from `inspect`, these methods return a discriminated receipt:

```ts
{ ok: true, data: Record<string, unknown> }
// or
{ ok: false, error: { code: string, message: string, feature_id?: string } }
```

Build receipts include `artifact_directory`, `geometry`, `validation`, feature reports, `timing_ms` and `artifact_sha256`. Available fields vary by operation; a validate/capabilities/observation result is not a build result. Keep the full receipt for diagnostics; send the model a compact summary and any requested PNG.

Preview options include `view: "iso" | "top" | "bottom" | "front" | "back" | "left" | "right"`, image size, `chord_mm` and section settings. Use the schema for exact fields and limits. A different camera can reuse geometry; changing mesh precision or geometry can require new work.

The kernel SDK limits serialized input to 32 MiB. Query capabilities for native feature, assembly and mesh budgets. The SDK has no in-flight `AbortSignal` contract in alpha.7: the Pi example checks cancellation before dispatch, while an already-running native call is bounded by `wallMs`.

## Agent tools

`await createCadTools(cad)` returns definitions with `{ name, description, inputSchema, execute }`. Register them using your harness's tool interface:

| Tool | Arguments |
| --- | --- |
| `cad_capabilities` | `{}` |
| `cad_validate` | `{ model }` |
| `cad_build` | `{ model, options?: { queryable?: boolean } }` |
| `cad_preview` | `{ model, options? }` |
| `cad_edit` | `{ model, parameters }` |
| `cad_solve_sketch` | `{ sketch, parameters? }` |
| `cad_inspect_step` | `{ model, asset }` |
| `cad_query` | `{ artifact_directory, request }`; session-bound form takes `{ request }` against the accepted artifact |

Preserve the complete `inputSchema`, including root `$defs`. Do not wrap it without rewriting `$ref` locations. These definitions are transport-neutral; the package does not launch an MCP server. If a provider rejects the union-heavy schema, use the skill with shell tools or build a provider-specific schema adapter and test its validation behavior.

The Pi example exposes the first five tools plus capability-gated `cad_query`. It converts failed receipts into failed tool results and includes PNG bytes in image content when a preview is at most 4 MiB. Build/edit feedback retains the query revision. See the [query guide](engineering-queries.md) for selection, datums, holes/groups, interfaces and supported geometric relationships.

For STEP imports, use `attachStep(model, assetId, stepText)` from `@coloopx/colocad/kernel`. The host registers text under an opaque ID; the model need not choose arbitrary filesystem paths. The returned model is a copy. [Integration policy and lifecycle](integrations/overview.md).

## Persistent sessions and CLI visualization (alpha.4+)

Import `ModelingSession`, `createSessionTools`, `readSessionBundle` and `exportSessionHtml` from `@coloopx/colocad/session`. A recorded edit takes `{parameters}` and uses the accepted source; it does not take the plain tool's `{model, parameters}` input. The browser component is exported at `@coloopx/colocad/viewer`.

```sh
colocad record MODEL.json SESSION --label "Milestone"
colocad view SESSION --no-open --port 8787
colocad replay SESSION review.html
colocad render SESSION modeling.mp4 --width 1280 --height 800 --fps 15
```

`record` accepts feature trees or kernel JSON. The viewer serves a local snapshot. `replay` creates a self-contained HTML file. `render` uses optional Playwright/Chromium and FFmpeg to encode actual saved meshes; it writes a movie, poster and manifest. See [modeling history](modeling-history.md) for lifecycle, all options and artifact semantics.

### Recorded exact inspection in the alpha.9 candidate

`colocad head SESSION` returns the accepted step ID, geometry summary and query revision. An empty session returns JSON null. A build without a query snapshot has a null query revision. `colocad inspect SESSION REQUEST.json` executes a revision-bound request and records its receipt. `colocad edit SESSION PARAMETERS.json` changes numeric parameters on the accepted model. CLI failures exit nonzero. The SDK equivalents are `session.head()`, `session.query(request)` and `session.edit(parameters)`. Read the source through the relative `inputFile` in each ledger entry. Multiple inspections can point to the same immutable source. [Runnable plate workflow](../skills/colocad/references/modeling-loop.md#inspect-and-edit-from-a-shell).
