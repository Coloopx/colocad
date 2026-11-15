# SDK, CLI and tool reference

The examples target `@coloopx/colocad@0.1.0`. The [public declarations](../src/index.d.mts) and [kernel declarations](../kernel/src/types.ts) are the complete type contracts.

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

The kernel SDK limits serialized input to 32 MiB. Query capabilities for native feature, assembly and mesh budgets. The Pi integration checks `AbortSignal` before dispatch. Once the native process starts, `wallMs` limits its runtime.

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

### Recorded exact inspection in alpha.9

`colocad head SESSION` returns the accepted step ID, geometry summary and query revision. An empty session returns JSON null. A build without a query snapshot has a null query revision. `colocad inspect SESSION REQUEST.json` executes a revision-bound request and records its receipt. `colocad edit SESSION PARAMETERS.json` changes numeric parameters on the accepted model. CLI failures exit nonzero. The SDK equivalents are `session.head()`, `session.query(request)` and `session.edit(parameters)`. Read the source through the relative `inputFile` in each ledger entry. Multiple inspections can point to the same immutable source. [Runnable plate workflow](../skills/colocad/references/modeling-loop.md#inspect-and-edit-from-a-shell).

For new compact sessions in the source candidate, use `session.receipt(step)` to read a checked, full receipt from either storage layout. See [session compatibility and persistence](indexed-sessions.md).


## Invocation-local STEP parsing diagnostics

Build receipts include `step_source_cache`: `parse_calls`, `hits`, `capacity`, `retained_models` and `scope`. These counts cover adjacent ordinary part/definition imports only; whole-assembly import, inspection and checkpoint decoding retain their existing paths. Adjacent imports of exactly the same verified source SHA and the same ordinary-part/definition mode reuse the most recently admitted neutral source model. A different source or mode evicts it before the original complete parse/admission callback runs. Asset aliases with identical verified bytes may hit the same slot; an asset ID alone is never a cache key.

The source slot is local to one native build, holds at most one neutral model, is dropped before parsing another source, and is cleared for other geometry operations and before final snapshots/STEP/display. It never stores a B-rep or selects a body for the caller. Only a successfully completed original parser/admission callback populates the slot. A hit reuses that source under the same mode; the stricter ordinary-part mode cannot inherit a definition-mode admission. Whole-model asset digest verification still runs before execution, and each feature still checks its own body index and identity and constructs its own B-rep. No byte/count/geometry/output/time budget is increased.

The instrumentation-only Windows red at `41e9e9e` observed 32 parses and zero hits for 32 adjacent definitions after all solid, volume, bounds and literal-frame assertions passed. The source candidate adds reuse after that red; its nine targeted controls, independent STEP readback and full-suite acceptance must be reported from the matching Windows generation. Parse-count reduction does not by itself establish elapsed-time, RSS or whole-CAD acceptance.

### Windows source-reuse validation (2026-11-15)

The instrumentation-only revision `41e9e9e` completes the 32-solid geometry, identity and literal-placement assertions, then reports 32 source reads and zero hits and fails the intended count assertion. Production revision `04d62d7198d8c2aff8209035ec64967f41ccf1a6` reports one read and 31 hits. Final test revision `5e87cf4ac04c80783c5a2ee065018561747a4729` adds the public revision-checkpoint control without changing production code: cold imports, complete restoration, a changed body index with two cache hits/one miss, and stricter part-mode refusal all pass.

Windows/WSL2 final acceptance: all 10 targeted controls and 144 native workspace tests across 27 executables pass, with zero failures or ignored tests. SDK/consumer type checks and doctests pass. The serial Node suite is 52/53; the remaining streaming-migration `SESSION_BUSY` failure is retained and also reproduced with baseline session/catalog sources. No all-green Node claim is made.

Compilation and runtime both used two CPUs, 3 GiB RAM, zero swap and no OOM. Measured peaks were 1,508,564,992 bytes for full compilation, 372,731,904 for Rust runtime and 596,074,496 for Node runtime. These are the public test workload's observed peaks, not a bound for complex hardware or a speedup claim.

Independent OCCT WASM reads the actual retained 32-solid output as valid, with volume 98378.7614034052 mm³ and bounds `[[0,0,0],[1012,16,10]]`. The public reference is byte-exactly related to the native input by one added comment. OCCT's attempt to read the single one-MiB comment hit its scanner buffer limit; that raw padded transfer did not pass. The unchanged reference and actual delivered STEP were independently read without rebuilding or repairing the native output. Global material equivalence, real vendor hardware and original seven-case acceptance remain unestablished.

### Host STEP count policy

`new CadClient({outputRoot, stepImportLimits: {maxEntities, maxDefinitions, maxOccurrences, maxGroups}})` freezes a host-owned count policy. Every key is optional. Values must be positive safe integers; unknown keys fail before execution. The native equivalent is `agent-cad inspect-step --step-import-limits '{"maxEntities":200000,"maxDefinitions":256,"maxOccurrences":1024,"maxGroups":2048}'` (the same option works with build/validate/query/observe). Resource configuration is not a Model field and cannot be supplied by agent IR.

| Count | Default | Hard maximum |
|---|---:|---:|
| External Part21 entities | 100000 | 2000000 |
| Source B-rep definitions | 128 | 1024 |
| Mapped-per-product and total expanded leaf occurrences | 128 | 4096 |
| Imported/model groups | 512 | 8192 |

Parse, inspect, ordinary/definition/assembly STEP imports use the same policy and check the whole source, including occurrence expansion and fallback bodies. Internal generated STEP serialization remains separately limited to 1000000 entities per definition. No bytes, UV rules, units rules or geometric tolerances change. Source counts are not counts of hardware slots.

`RESOURCE_LIMIT` errors include `details.resource_limit = {resource, actual, allowed, stage}`. An entity parser stops at the first excess record: `actual` is the observed `allowed+1`, marked `actual_is_lower_bound:true`, not a claim to have scanned the remaining file. Successful inspect returns exact `source_entities`, source definitions/instances and `step_import_limits`; builds record their effective count policy.

The Rust entrypoints are `StepImportLimits`, `validate_with_import_limits(model, limits)`, `build_revision_with_import_limits(model, checkpoints, queryable, limits)`, and `step_assembly::inspect_with_import_limits(asset, limits)`. `step_import_limits::with_import_limits` establishes a synchronous thread-local scope for composition with other host policies; nested scopes and unwind restore the caller's policy. These are distinct from byte-budget APIs. Spawned threads do not inherit a scope.

Explicit extensions support plain native assemblies and import-only definition tables within the configured finite limits. Ordinary feature programs retain the 128-feature budget. Query snapshots and display caches remain limited to 128 definitions/occurrences: unsupported expanded queryable/preview builds fail before geometry, and cache readers enforce the same fixed capacities. The unchanged 8128 checked-pair, solver-variable, face, mesh, byte and time budgets can still reject an expanded model. No instances are merged, silently truncated, dropped or converted to a compound to fit a smaller table.

Checkpoint keys include the count policy as well as the existing geometry/execution identity. Reusing a revision under a different or lower policy therefore replays source imports instead of trusting a high-budget cached body. Query/preview artifacts retain their independent fixed format capacities.


STEP policy values are read once into an own-data snapshot with no custom prototype, validated there and serialized from that same snapshot. Inherited or non-enumerable `toJSON` hooks are not invoked; an accessor cannot substitute a different value during serialization. A constructor's stored flag therefore represents exactly the values that passed validation.

Query instance paths retain a fixed limit of 513 entries **including the leaf**. Queryable builds (including those also requesting preview) check each used leaf's actual ancestor chain before feature execution and return `RESOURCE_LIMIT` with resource `query_instance_path`, actual path length, allowed 513 and stage `downstream_capacity`. This is not a total-group 512 restriction: 513 shallow groups with a short used leaf path remain supported under an explicit group policy. Preview-only artifacts flatten world placements and do not consume query paths, so the 513-entry query-path ceiling is not invented as a new display restriction; their existing 128 definition/occurrence ceilings remain unchanged.
