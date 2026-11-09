# SDD 0005 — Four ordered goals for an independent agent CAD stack

Status: **G1–G3 implemented and accepted within the declared domains/workloads; G4 not started**. Date: 2026-11-09. [中文](0005-capability-reliability-and-optimization.zh-CN.md).

Colocad aims to independently support the specified modeling and assembly workloads through a local Rust kernel, TypeScript SDK, CLI and skill. This is not a claim of complete CadQuery/build123d equivalence. Agent, harness and the CAD package execute inside the same sandbox. SDD 0004 remains the detailed query/datum/interface contract; this document governs the order of the wider program.

## 1. Ordered gates

| Goal | Required outcome | Exit evidence |
| --- | --- | --- |
| **G1 — functional coverage** | Complete the supported SDD 0004 query, composable selection, primitive datum, mate, hole/group and installation-interface contracts through SDK/CLI/tools. Publish supported domains and explicit unsupported outcomes. | Executable positive/negative fixtures for every advertised capability; generated schemas; one end-to-end external STEP → selection → datum/interface → assembly → exact check. Numeric placement fixtures alone do not pass. |
| **G2 — geometric reliability** | Exercise complex booleans, blends, thin walls, surfaces and externally authored STEP across a source-backed corpus, including valid difficult cases and expected invalid cases. | Frozen corpus and acceptance targets; independent geometry/placement/material checks; documented success/failure denominators by family; all required cases pass, expected failures are correctly classified, and no required feature is silently degraded. |
| **G3 — engineering usability** | Preserve semantic intent across parameter edits and source replacement; reject stale observations; diagnose failures; prove declared resource envelopes in the sandbox. | Edit/replacement sequences, reopen/relocation and ambiguity controls, session recovery, real agent/tool traces, Linux arm64 1 vCPU / 512 MiB CAD resource measurements with explicit resident-process accounting. |
| **G4 — benchmark and retrospective optimization loop** | Repeatedly identify bottlenecks/failures, make general improvements and retain only independently verified gains. | The reproducible loop in §3–5 runs only after G1–G3 have accepted evidence. Baselines, rejected candidates, regressions and held-out outcomes remain inspectable. |

The order is **G1 → G2 → G3 → G4**. A later goal does not become accepted just because an earlier implementation incidentally contains one of its features. TDD, input bounds, independent spot checks and failure recording continue during every goal; they are necessary construction practices, not a premature G4 performance campaign. Fixing a correctness failure takes precedence over optimizing its running time.

SDD 0004 M0–M3 organize implementation modules within G1, while its lifecycle/replacement/resource acceptance feeds G3. Its M4 release requires the relevant functional/reliability/usability evidence. Existing experiments are retained; their successful subsets do not waive a gate. A new unsupported domain requires an explicit scope revision, not silently dropping a hard case.

## 2. Corpus and acceptance before tuning

Each case has a stable ID and source pack: provenance/license/content hashes; units; geometry/assembly requirements; editable parameters; accepted representation/degradations; exact outputs; independent verifier/version and tolerances. Separate observed dimensions, derived values, assumptions and unknowns. Unavailable evidence stays unresolved.

Use three sets: development reproducers, a frozen regression set, and held-out products/parameter combinations. Record the split before tuning. Include analytic parts, imported foreign STEP, assemblies with repeated definitions, and whole-product workflows. Every relevant family contains boundary/invalid cases so that returning an empty result or refusing everything cannot improve the reported score.

G2 chooses the concrete required corpus and family thresholds before execution. G3 adds parameter-edit, topology-change, symmetry, part replacement and resource sequences. Do not expand a fixture's tolerances or remove required details after a candidate fails merely to obtain a pass.

## 3. G4: the self-optimization cycle

“Self-optimization” means an agent-operated, auditable **code/skill/tool improvement loop**; it does not mean automatic model weight training or an unverified claim that the model learns from a benchmark.

1. **Freeze a baseline.** Pin source commit, native/SDK hashes, kernel revision, compiler, dependencies, hardware/sandbox limits, fixtures, verifier and agent/model/skill/tool versions where inference participates.
2. **Run paired workloads.** Compare Colocad with **build123d** on equivalent required geometry, precision, validation and outputs. Use OCCT as an independent verifier; label verification cost separately. Scripted execution and live-agent modeling are different tracks.
3. **Localize a result.** Preserve failure receipts and per-stage/per-feature traces. Attribute cost or error to source interpretation, feature construction, selection, datum resolution, assembly solving, validation, serialization, STEP export, tessellation, rendering or host overhead. Never infer the cause from total duration alone.
4. **Trace backward.** Replay the last accepted parameter/source state, identify the first failing feature/constraint or regressing commit, and minimize a reproducer while retaining the original failure. Preserve the full product case as a regression; the small reproducer is not its substitute.
5. **Change one hypothesis.** Record expected mechanism, affected workloads and risks, then implement a bounded candidate in a separate change. A fast path requires a geometric eligibility predicate and a correct general fallback; no case-name/template dispatch.
6. **Apply correctness gates first.** Run the reproducer, affected family, frozen regression set and independent readback. A missing feature, changed fit, weakened tolerance, swallowed ambiguity or hidden degradation rejects the candidate regardless of speed.
7. **Measure and challenge.** Re-run the paired benchmark, including cold/warm behavior and held-out cases. Require reproducible gains that exceed observed run noise. Report regressions and tradeoffs; do not choose only a favorable model or best run.
8. **Accept or revert, then retain evidence.** Keep the candidate only if all applicable correctness/resource gates pass and its stated objective improves. Record rejected candidates too. An accepted improvement becomes the next immutable baseline; never rewrite old measurements.

## 4. Metrics and promotion rules

Report success/expected-rejection/unsupported/crash/timeout/OOM counts with denominators before performance. Track reference invalidation, dimensional/material/placement error, missing required features and false-positive interface acceptance. For agent runs also report tool/repair calls, inference tokens/cost, unsupported claims and task outcome under the same budget.

Separate cold process startup, feature build, snapshot creation/load, selection/recognition, mate solving, exact checks, STEP export, tessellation and rendering. Report time-to-STEP and time-to-preview independently. A descriptor-only query must not rebuild or export; a cached observation must not be compared to fresh modeling.

Use matched environments, a disclosed warmup policy, at least five fresh-process samples and five repeated-operation samples, with median, sample count, spread and maximum. Tail percentiles from tiny samples must be labelled as such. Memory includes peak cgroup usage, OOM counters, charged descendants/cache pages and artifact sizes; sampled RSS is a supplementary measurement, not an interchangeable number. Separate CAD-only and end-to-end resident agent/harness budgets.

Promote only after unchanged acceptance targets and no required correctness regression. Evaluate speed, peak memory, failure rate and agent cost as a set; a faster but memory-infeasible candidate fails. No fixed 10×/10%-memory or complete-parity claim is inherited from historical plate benchmarks. If no robust gain exists, retain the old baseline and report a rejected hypothesis.

## 5. Persistent optimization record

Each iteration stores: `iteration_id`, parent baseline, goal/gate state, hypothesis, fixture split/hashes, environment, commands, source/native/skill hashes, raw runs, feature/phase traces, minimized failure, independent verification, candidate diff, regression/held-out results and `accepted | rejected | inconclusive` with rationale. Link findings to a regression test and commit. Resource/inference failures retain their partial evidence.

The future runner must refuse G4 promotion when any G1–G3 gate is missing/stale/failed. A source, fixture, verifier or acceptance change invalidates the relevant evidence rather than silently reusing a green badge. This protocol is currently specified, not implemented or executed as a completed optimization service.

## 6. Current work

G1 is accepted within the supported SDD 0004 domains; the [G1 ledger](experiments/0004/README.md) retains functional, foreign STEP and whole-product evidence. G2 passes all 24 required frozen cases, including 18 positive geometries and six correct rejections. G3 passes semantic lifecycle/replacement, real process recovery, cache-policy invalidation, bounded parser probes and two live Pi agent tasks. The final regression is 127 Rust, 41 Node tests on each of macOS and limited Linux, and six scripted integration tests. Linux-produced corpus and live-task STEP outputs pass independent OCCT readback. See the [G2/G3 acceptance record](experiments/0005/README.md).

The agent, local harness, SDK and native execute together within Linux arm64 1 vCPU / 512 MiB; the 82-solid product task peaks at 370.57 MiB with no OOM. This is measured workload acceptance, not a universal input memory bound or complete industrial CAD equivalence. G4 has not started. Packaged npm alpha.6 does not yet include these development changes; source acceptance is separate from publication.

## Publication update: alpha.7

The accepted supported domains now ship in the alpha.7 SDK, CLI and three native bundles. Historical acceptance records remain unchanged; see the [changelog](../CHANGELOG.md) and [installed-archive evidence](../docs/evidence/release-alpha.7.json). G4 has not started.
