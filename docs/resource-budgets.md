<!-- [INPUT]: Rust/SDK宿主字节合同与公开产物读取入口。
[OUTPUT]: 输入/库存/独立输出STEP默认、硬顶、来源与回读边界。
[POS]: 资源合同文档；不替代实际Windows或真实消费链验收。
[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md -->
# Host-controlled STEP byte budgets

`CadClient` accepts an optional **host configuration** named `resourceLimits`. It is not a model field or an agent tool argument. Existing defaults remain unchanged; opt-in limits must be positive integers within the finite ceilings below.

| Host setting | Default | Ceiling | What it measures |
| --- | ---: | ---: | --- |
| `inputBytes` | 32 MiB | 512 MiB | Actual UTF-8 bytes of the serialized native request, including JSON escaping |
| `stepBytes` | 24 MiB | 256 MiB | UTF-8 content of each registered STEP asset |
| `totalStepBytes` | 24 MiB | 512 MiB | Sum of STEP asset content in one model |
| `outputStepBytes` | 32 MiB | 512 MiB | Generated STEP artifact UTF-8 bytes, independently of input assets |

`stepBytes <= totalStepBytes <= inputBytes` is required. `outputStepBytes` is independent: an output limit larger than input permission is valid, and input permission never raises the output limit. Raising registration alone does not raise the native budgets. Query requests retain their independent 2 MiB ceiling; count, nesting, topology, geometry, preview and wall-clock limits retain their existing meanings.

```js
import {CadClient, attachStep} from '@coloopx/colocad/kernel';

const MiB = 1024 * 1024;
const resourceLimits = {
  inputBytes: 64 * MiB,
  stepBytes: 40 * MiB,
  totalStepBytes: 48 * MiB,
  outputStepBytes: 128 * MiB, // Explicit host opt-in; SDK default remains 32 MiB.
};
const cad = new CadClient({outputRoot: './out', resourceLimits, wallMs: 120000});
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

Existing SDK stdout limits remain independent:2MiB for ordinary commands and8MiB for query. `outputBytes` was an error resource name, not a public host metadata policy. Native previous-checkpoint manifests also retain their existing2MiB bound. This change neither makes those limits configurable nor claims large report generation/cache acceptance; it removes the newly introduced report-size dependency from STEP artifact reading. Other JSON consumers retain their own existing semantics, and outputStepBytes never authorizes arbitrary metadata.

`readModelFile` remains a foreign-input codec using the client's input and STEP registration limits. **Reading a generated artifact is not permission to reimport it.** For example, a128MiB output budget plus a24MiB source budget may allow artifact delivery while a larger STEP reimport is refused. A trusted consumer may explicitly choose a separate, bounded input profile for reopening SHA-verified current artifacts; it must not infer this authority from producer metadata or model IR, or change ordinary source defaults. Actual transport bytes, aggregate source, entity/context/unit and geometry checks still apply. A source satisfying byte budgets can fail those checks.

Successful native inspection, validation and build receipts include `resource_limits` and `resource_usage`. Builds record `resource_usage.output_step_bytes` separately from imported-source usage. A rejected request uses `RESOURCE_LIMIT` with `details.resource`, `limit_bytes`, `observed_bytes` and `observed_is_lower_bound`. Assembly serialization counts its syntax and terminator and may refuse early with a lower-bound size. Completed STEP size is exact. SDK serialization knows the full input size. Native oversized stdin reads stop at the configured limit plus one byte and explicitly report a lower bound. Invalid host options use `INVALID_RESOURCE_LIMITS`.

The native CLI accepts one host-owned option:

```sh
agent-cad build --out NEW_DIR --resource-limits '{"inputBytes":67108864,"stepBytes":41943040,"totalStepBytes":50331648,"outputStepBytes":134217728}' < model.json
```

Rust callers can use `ResourceLimits`, `validate_with_resource_limits`, `build_revision_with_resource_limits`, and `step_assembly::inspect_with_resource_limits`. Existing entry points retain the default policy. Asset sizes and SHA checks run before checkpoint restoration, so a previous successful larger-budget build cannot bypass a stricter current policy.

This implementation still sends imported STEP text inline in JSON. Output uses staged file artifacts and atomic revision publication, but the pinned writer returns a complete String; this repair does not provide end-to-end streaming or unlimited memory. Geometry, shared definitions and the pinned vendor remain unchanged.

This backport is based only on PR #6 head `30d20a4cdb2f8c644ae10248e05c09fc0e431b5b`, whose upstream base is `c3200d59c326eb848bb76bfef1362e5899c78666`. No other PR is merged here. Native, SDK and type regressions are prepared for Windows; local compilation and execution have not run. Regenerate `kernel/dist` and build a fresh native binary before acceptance. The existing generated SDK in this source tree still belongs to the PR6 baseline.

Larger limits do not guarantee a complex CAD fits a memory/time quota. Small native controls cover single/assembly boundaries and restored checkpoints; public padded STEP bytes cover the reader independently. Neither establishes real Pico or complete consumer acceptance.
Run the public SDK controls with a freshly built native executable:

```sh
AGENT_CAD_BINARY=/absolute/agent-cad node --test test/resource-limits.test.mjs
node test/validation/resource-budgets.mjs /absolute/agent-cad NEW_OUTPUT 34603008
node test/validation/resource-budgets-occt.mjs crates/agent-cad/tests/fixtures/occt-drilled-block.step DELIVERED_STEP NEW_REPORT.json
```

The last command uses the repository's locked independent OCCT WASM dependency and records its SHA, validity, solid count, volume and bounds. It does not execute Colocad, modify the input or prove global material equivalence. The optional `resource-budgets.py NEW_OUTPUT` audit additionally compares material probes in an explicitly selected OCP environment; neither audit is a runtime geometry fallback.
