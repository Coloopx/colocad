# Execution progress and inactivity supervision

A native command may finish after 120 seconds. `wallMs` accepts a positive Node timer integer (up to 2147483647 ms); it no longer has a package-specific 120-second ceiling. Existing callers without `idleMs` keep the 30-second wall default.

```js
const cad = new CadClient({
  outputRoot: './out',
  idleMs: 600000,
  signal: controller.signal,
  onProgress: event => record(event),
});
```

With `idleMs` and no `wallMs`, there is no total deadline. Each newly ordered native event renews the inactivity window. An explicit `wallMs` additionally imposes a total deadline. These are host settings; the model cannot set them. `signal` cancellation returns `CANCELLED`. A stalled command returns `IDLE_TIMEOUT`, with the last progress event and feature identity when available. This receipt does not certify that repeating the same model will help.

`--progress` enables native stderr lines prefixed `COLOCAD_PROGRESS `, schema `colocad.progress/1`, increasing `seq`, phase `command`, `feature` or `check`, and state `started` or `completed`. Feature/check events carry `feature_id`. Stdout remains the existing single JSON receipt. The SDK rejects malformed, oversized, repeated or out-of-order events as progress; ordinary logs do not renew the timer. Observer errors do not alter CAD results. There are no periodic heartbeats.

A single expensive primitive can remain silent until it finishes. This change exposes feature and exact-check boundaries; it does not provide progress inside arbitrary SolidCore operations. Hosts must choose an inactivity window that admits their supported primitives. Resource budgets, output byte limits, sandbox isolation and cancellation remain independent.

Collision checks with fixed placements run when both declared source features exist. They use the same exact material intersection, allowed-pair policy and overlap threshold as final assembly validation. Passing pair results are reused at final validation. A failing receipt includes both part IDs, source features, overlap and limit. The scheduler does not infer collisions from intermediate union/cut operands. Models with mates retain post-solve checking because solved placements may change.

Tests use real child processes for supervision, deterministic later failures to establish early-check order, and actual native geometry for overlap, rigid placement, allowed pairs and mate fallback. Build the native executable from the same checkout before running SDK tests.

## CLI and harness integration in 0.1.3

Native CLI build, record, edit, inspect and query, and the bundled native build skill, use an inactivity window of 120,000 ms by default. Set `COLOCAD_IDLE_MS` in the host environment to admit a longer single primitive. Set `COLOCAD_WALL_MS` only when the host requires a total deadline. Set `COLOCAD_PROGRESS=1` to forward native events on stderr. Tool adapters can pass `executionPolicyFromEnv(process.env)` into `CadClient` or `ModelingSession`. Existing explicit SDK wall budgets continue to be enforced.

A harness should route `ASSEMBLY_COLLISION` to model repair with the part IDs, measured overlap and threshold. On `IDLE_TIMEOUT` or `TIMEOUT`, show the failed stage and the applied host policy. Offer a changed budget or an edited recipe. Preserve host cancellation as a separate outcome. The native CLI build error includes the structured SDK receipt alongside its existing error string.
