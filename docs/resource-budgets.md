# Host-controlled STEP byte budgets

`CadClient` accepts an optional **host configuration** named `resourceLimits`. It is not a model field or an agent tool argument. Existing defaults remain unchanged; opt-in limits must be positive integers within the finite ceilings below.

| Host setting | Default | Ceiling | What it measures |
| --- | ---: | ---: | --- |
| `inputBytes` | 32 MiB | 512 MiB | Actual UTF-8 bytes of the serialized native request, including JSON escaping |
| `stepBytes` | 24 MiB | 256 MiB | UTF-8 content of each registered STEP asset |
| `totalStepBytes` | 24 MiB | 512 MiB | Sum of STEP asset content in one model |

`stepBytes <= totalStepBytes <= inputBytes` is required. Raising registration alone does not raise the native budgets. Query requests retain their independent 2 MiB ceiling; count, nesting, topology, geometry, output, preview and wall-clock limits retain their existing meanings.

```js
import {CadClient, attachStep} from '@coloopx/colocad/kernel';

const MiB = 1024 * 1024;
const resourceLimits = {
  inputBytes: 64 * MiB,
  stepBytes: 40 * MiB,
  totalStepBytes: 48 * MiB,
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

The host policy is validated and snapshotted when creating the client. `cad.resourceLimits` exposes a frozen view. `readModelFile` uses that client's input limit and STEP registration limit; JSON and embedded-model file exports use its input/container limit. Ordinary STEP, mesh and image export limits remain independent. A source that satisfies the byte budgets can still fail parsing, count, unit or geometric checks.

Successful native inspection, validation and build receipts include `resource_limits` and `resource_usage`. A rejected request uses `RESOURCE_LIMIT` with `details.resource`, `limit_bytes`, `observed_bytes` and `observed_is_lower_bound`. SDK serialization knows the full input size. Native oversized stdin reads stop at the configured limit plus one byte and explicitly report a lower bound. Invalid host options use `INVALID_RESOURCE_LIMITS`.

The native CLI accepts one host-owned option:

```sh
agent-cad build --out NEW_DIR --resource-limits '{"inputBytes":67108864,"stepBytes":41943040,"totalStepBytes":50331648}' < model.json
```

Rust callers can use `ResourceLimits`, `validate_with_resource_limits`, `build_revision_with_resource_limits`, and `step_assembly::inspect_with_resource_limits`. Existing entry points retain the default policy. Asset sizes and SHA checks run before checkpoint restoration, so a previous successful larger-budget build cannot bypass a stricter current policy.

This implementation still sends inline STEP text in JSON. Larger limits are not a guarantee that a complex CAD fits a memory/time quota. Size-only regressions use valid public STEP geometry padded with comments, distinct from real vendor assemblies. Deployments must measure their actual source geometry under their sandbox limits; a budget rejection is not evidence of damaged CAD or geometric impossibility.

Run the public SDK controls with a freshly built native executable:

```sh
AGENT_CAD_BINARY=/absolute/agent-cad node --test test/resource-limits.test.mjs
node test/validation/resource-budgets.mjs /absolute/agent-cad NEW_OUTPUT 34603008
node test/validation/resource-budgets-occt.mjs crates/agent-cad/tests/fixtures/occt-drilled-block.step DELIVERED_STEP NEW_REPORT.json
```

The last command uses the repository's locked independent OCCT WASM dependency and records its SHA, validity, solid count, volume and bounds. It does not execute Colocad, modify the input or prove global material equivalence. The optional `resource-budgets.py NEW_OUTPUT` audit additionally compares material probes in an explicitly selected OCP environment; neither audit is a runtime geometry fallback.
