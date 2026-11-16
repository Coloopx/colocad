<!-- [INPUT]: Rust/SDK宿主字节合同与公开产物读取入口。
[OUTPUT]: input/STEP/reportBytes合同、精确计量与独立验收边界。
[POS]: 资源合同文档；不替代实际Windows或真实消费链验收。
[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md -->
# Host-controlled resource byte budgets

`CadClient` accepts an optional **host configuration** named `resourceLimits`. It is not a model field or an agent tool argument. Existing input and STEP defaults remain unchanged; opt-in limits must be positive integers within the finite ceilings below.

| Host setting | Default | Ceiling | What it measures |
| --- | ---: | ---: | --- |
| `inputBytes` | 32 MiB | 512 MiB | Actual UTF-8 bytes of the serialized native request, including JSON escaping |
| `stepBytes` | 24 MiB | 256 MiB | UTF-8 content of each registered STEP asset |
| `totalStepBytes` | 24 MiB | 512 MiB | Sum of STEP asset content in one model |
| `outputStepBytes` | 32 MiB | 512 MiB | Generated STEP artifact UTF-8 bytes, independently of input assets |
| `reportBytes` | 8 MiB | 128 MiB | Complete command response and report UTF-8 bytes; minimum 1024 bytes |

## Matching SDK and native executable

These resource options require a matching rebuilt native executable. Updating the tracked generated SDK files alone does not upgrade a bundled or stock binary. The pull request must include the generated SDK bytes from the same TypeScript source, but platform binaries and release metadata are separate artifacts.

Use an explicit `CadClient` `binary` path or `AGENT_CAD_BINARY` pointing to the rebuilt executable used for validation. A stock binary from an earlier release is not evidence that the new options are supported. Check the rebuilt executable's actual capabilities and source/native identities before relying on them. No platform-binary update, release-metadata update or npm publication is implied by these source and SDK changes.

`stepBytes <= totalStepBytes <= inputBytes` is required. `outputStepBytes` is independent: an output limit larger than input permission is valid, and input permission never raises the output limit. Raising registration alone does not raise the native budgets. Query requests retain their independent 2 MiB ceiling; count, nesting, topology, geometry, preview and wall-clock limits retain their existing meanings.

```js
import {CadClient, attachStep} from '@coloopx/colocad/kernel';

const MiB = 1024 * 1024;
const binary = process.env.AGENT_CAD_BINARY;
if (!binary) throw new Error('Set AGENT_CAD_BINARY to a matching rebuilt executable');
const resourceLimits = {
  inputBytes: 64 * MiB,
  stepBytes: 40 * MiB,
  totalStepBytes: 48 * MiB,
  outputStepBytes: 128 * MiB, // Explicit host opt-in; SDK default remains 32 MiB.
};
const cad = new CadClient({binary, outputRoot: './out', resourceLimits, wallMs: 120000});
const source = attachStep(
  {version: 1, units: 'mm', parameters: {}, features: []},
  'foreign', stepText, {maxBytes: resourceLimits.stepBytes},
);
const inspection = await cad.inspectStep(source, 'foreign');
if (!inspection.ok) throw Error(JSON.stringify(inspection.error));
const result = await cad.build(
  {...inspection.data.model_template, assets: source.assets},
  {queryable: false},
);
```

The host policy is validated and snapshotted when creating the client. `cad.resourceLimits` exposes a frozen view. The public kernel `DEFAULT_RESOURCE_LIMITS.outputStepBytes` advertises **32 MiB** so consumers can negotiate support before sending the host option; an explicit consumer default such as128MiB does not change the SDK default. Native capabilities echo the actual current host option. Direct Rust capabilities advertise default byte policy.

`readStepArtifact(cad, artifactDirectory, {sha256}?)` reads the original `model.step` bytes under that client's output budget. By default it verifies `model.step.sha256`, a fixed65-byte sidecar (64 lowercase hex SHA256 digits plus newline) emitted atomically with the STEP; capabilities advertise its filename/format/exact length. A trusted host may instead supply the SHA from an already verified receipt, including for legacy artifacts without the sidecar. The reader never loads the full `report.json`. It confines artifact paths and reads regular files using a single descriptor, a stat preflight and a bounded loop; growth past the limit reports a lower bound. Producer limits cannot grant reader permission. The65-byte bound describes the exact digest wire format, not an assembly-size metadata quota.

`writeModelFile` uses this checked reader with the native receipt SHA for STEP and checks the final STEP bytes, including an optional embedded record, against `outputStepBytes`. JSON and embedded-model containers also retain their input/container limit; mesh/image limits remain unchanged. Session bundle/HTML export uses the current frozen reader budget and the already verified accepted receipt SHA before base64 encoding. Rejected HTML does not create the destination.

SDK/native complete responses and report readers share host reportBytes (default8MiB, ceiling128MiB, minimum1024B). Legacy checkpoint report over2MiB selects safe cold rebuild, recorded as checkpoint_recovery={mode:"cold",reason:"oversized_legacy_manifest"}; this is an optimization threshold, not a geometry/admission failure. Small corrupt JSON remains CHECKPOINT_IO and corrupt payload SHA never authorizes cache reuse. STEPreader uses its fixed65-byte SHA format or trusted native receipt digest independently of reports.

`readModelFile` remains a foreign-input codec using the client's input and STEP registration limits. **Reading a generated artifact is not permission to reimport it.** For example, a128MiB output budget plus a24MiB source budget may allow artifact delivery while a larger STEP reimport is refused. A trusted consumer may explicitly choose a separate, bounded input profile for reopening SHA-verified current artifacts; it must not infer this authority from producer metadata or model IR, or change ordinary source defaults. Actual transport bytes, aggregate source, entity/context/unit and geometry checks still apply. A source satisfying byte budgets can fail those checks.

Successful native inspection, validation and build receipts include `resource_limits` and `resource_usage`. Builds record `resource_usage.output_step_bytes` separately from imported-source usage. A rejected request uses `RESOURCE_LIMIT` with `details.resource`, `limit_bytes`, `observed_bytes` and `observed_is_lower_bound`. Assembly serialization counts its syntax and terminator and may refuse early with a lower-bound size. Completed STEP size is exact. SDK serialization knows the full input size. Native oversized stdin reads stop at the configured limit plus one byte and explicitly report a lower bound. Invalid host options use `INVALID_RESOURCE_LIMITS`.

The native CLI accepts one host-owned option:

```sh
agent-cad build --out NEW_DIR --resource-limits '{"inputBytes":67108864,"stepBytes":41943040,"totalStepBytes":50331648,"outputStepBytes":134217728}' < model.json
```

Rust callers can use `ResourceLimits`, `validate_with_resource_limits`, `build_revision_with_resource_limits`, and `step_assembly::inspect_with_resource_limits`. Existing entry points retain the default policy. Asset sizes and SHA checks run before checkpoint restoration, so a previous successful larger-budget build cannot bypass a stricter current policy.

This implementation still sends imported STEP text inline in JSON. Output uses staged file artifacts and atomic revision publication, but the pinned writer returns a complete String; this repair does not provide end-to-end streaming or unlimited memory. Geometry, shared definitions and the pinned vendor remain unchanged.

Related primary-source exporter examples are [CadQuery's assembly exporter](https://cadquery.readthedocs.io/en/latest/_modules/cadquery/occ_impl/exporters/assembly.html) and [build123d's exporter](https://build123d.readthedocs.io/en/latest/_modules/exporters3d.html). They are design references, not runtime geometry fallbacks or evidence that resource use is unbounded.

Larger limits do not guarantee a complex CAD fits a memory/time quota. Small native controls cover single/assembly boundaries and restored checkpoints; public padded STEP bytes cover the reader independently. These controls do not establish acceptance of a complete downstream product.
Run the public SDK controls with a freshly built native executable:

```sh
AGENT_CAD_BINARY=/absolute/agent-cad node --test test/resource-limits.test.mjs
node test/validation/resource-budgets.mjs /absolute/agent-cad NEW_OUTPUT 34603008
node test/validation/resource-budgets-occt.mjs crates/agent-cad/tests/fixtures/occt-drilled-block.step DELIVERED_STEP NEW_REPORT.json
```

The last command uses the repository's locked independent OCCT WASM dependency and records its SHA, validity, solid count, volume and bounds. It does not execute Colocad, modify the input or prove global material equivalence. The optional `resource-budgets.py NEW_OUTPUT` audit additionally compares material probes in an explicitly selected OCP environment; neither audit is a runtime geometry fallback.


## Report and checkpoint behavior

Native CLI measures the real compact complete response plus newline and the pretty report.json file without newline separately under reportBytes, before atomically publishing build/observe artifacts. Library build checks the response object. Counting stops at the limit and explicitly reports a lower bound when serialization is refused early. SDK full-command stdout, inspect/preview report and file/packed session receipts obey the current host budget, never producer authority. New session receipts use compact JSON plus newline; historical bytes and SHA remain immutable.

## Independent Windows validation

The matching rebuilt native executable used for this standalone validation has SHA-256 `b69fa92a93a65832cb0ff50634af11a75656c9d37ea6da15db54c94279cfdbc3`. The build uses the original pinned dependency archive and its two official patches; the verified vendor tree matches the source manifest, with no extra kernel patches.

Compilation and execution use separate resource lanes:

| Stage | Resource boundary | Profile and result |
| --- | --- | --- |
| Compile only | 2 CPU, 5 GiB RAM, no swap | Standard release profile, dependency optimization level 3, no profile overrides; full test-suite compilation completed in 21m32s |
| Native test execution | 2 CPU, 3 GiB RAM | The 28 compiled Rust test executables completed 147/147 tests |
| Full SDK tests | 2 CPU, 3 GiB RAM | 87/88 passed; the sole failure is the independently reproduced `SESSION_BUSY` baseline in `session-catalog.test.mjs:117`; session catalog logic was not changed |
| Types and schema | 2 CPU, 3 GiB RAM | All public type contracts and native schema/TS contract check passed |

Two earlier cold compilations at 2 CPU/3 GiB each failed with OOM: one used the default release optimization level 3; the other used dependency optimization level 1 with 64 codegen units. Both failures are retained. The successful 5 GiB compile-only run is not evidence that the default profile cold-compiles within 3 GiB, and the reduced-optimization attempt did not succeed.

### Reproduction recipe

Use the verified source/vendor tree and an independent target directory. No Cargo.toml or geometry changes, and no profile overrides, are required for the accepted default-profile build. The compile-only lane is separate from the runtime quota; it does not raise the runtime/helper lane's existing memory gate.

```sh
# Compile-only lane: 2 CPU / 5 GiB / no swap; default release profile.
# CARGO_TARGET_DIR points to this independent build's target directory.
cargo test --release --no-run
```

Run the 28 executable paths emitted by that compilation directly in the separate 2 CPU/3 GiB runtime lane. Do not trigger another cold Cargo compilation inside the runtime lane. For SDK tests, use the same source's Windows-generated tracked SDK files and explicitly select the rebuilt native executable whose SHA is recorded above:

```sh
AGENT_CAD_BINARY=/absolute/path/to/rebuilt/agent-cad node --test test/*.test.mjs
```

The full SDK result retains the known baseline failure rather than treating it as an all-green suite. Types and native schema/TS contract checks passed. Generated SDK changes belong in the pull request; platform-native binaries and release.json are not updated, and no npm or platform release is claimed. A stock binary does not automatically acquire these capabilities.
