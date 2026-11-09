# G2 / G3 acceptance record

[Ordered contract](../../0005-capability-reliability-and-optimization.md) · [中文](../../0005-capability-reliability-and-optimization.zh-CN.md) · [G1 evidence](../0004/README.md)

G1 was accepted at `4e479f6`. G2 uses the independently generated, frozen [24-case corpus](corpus-v1/manifest.json), committed before any native corpus execution at `3d0221f`. All cases are required; no degraded result is allowed. This is a bounded public reliability corpus, not an estimate of success on every industrial CAD model.

| Family | Positive / expected rejection | What is independently checked |
| --- | --- | --- |
| Boolean | 3 / 1 | Three intersecting bores and pocket at three scales; required disjoint cut must fail |
| Blend | 2 / 1 | All twelve box edges rounded plus four functional bores; impossible radius rejected |
| Thin wall | 2 / 1 | 0.25 mm closed cavity and 0.4 mm open housing; protruding concave cavity rejected |
| Surface | 2 / 2 | Two nonplanar Bezier sheets, 81 independent points each; ragged control net and inconsistent crossings rejected |
| Foreign STEP | 8 / 1 | Independently authored housing, arbitrary rotation, split bores, NURBS, conical and intersecting bores, bracket; digest mismatch rejected |
| Whole product | 1 / 0 | Catalog-backed KK60 with accessory, 82 solids, four mounting-hole correspondences and nine explicit pair/pose samples |

Development/regression cases run before the six held-out parameter/source cases. The OCCT verifier independently reads exact STEP, checks validity, solid count, adaptive volume/area and bounds, then symmetric material difference or sheet sample distance. Volume tolerance is `max(1e-6 mm³, 1e-6 × expected volume)`; linear/sample tolerance is `1e-5 mm`. Catalog reconstruction uses the existing `0.01 mm / 1e-4 rad` scope. Hash-bound native receipts and failed attempts are retained.

## Reproduce

Build the current native as documented for G1. Python and OCCT are verifier dependencies only:

```sh
export AGENT_CAD_BINARY="$PWD/../../work/colocad-build/target/release/agent-cad"
python specs/experiments/0005/run.py out/g2-regression regression
# Continue only after that pass; output directories must be new.
python specs/experiments/0005/run.py out/g2-held-out held_out
```

Use `cadquery-ocp-novtk==7.9.3.1.1`. `freeze.py` constructs the OCCT source/reference pack without calling Colocad, and refuses to overwrite a frozen manifest. Existing corpus inputs should normally be reused, not regenerated. `run.py NEW_DIR SPLIT PRIOR_RUN_DIR` independently revalidates existing, digest-checked artifacts and records their original native/source identity; it is not a new native execution.

## Findings and preserved corrections

1. The first verifier mistakenly treated the presence of `degradation: null` as a degradation. It now rejects non-null degradations. Revalidation preserved the original native/artifact identity; all numerical targets remained unchanged.
2. Ordinary `cut` previously accepted a disjoint tool and emitted unchanged stock. It now uses the same explicit material-removal check as feature cutting tools. Disjoint and exactly touching tools reject with `NO_MATERIAL_REMOVED` and the failing feature ID.
3. One held-out input used edge-selector syntax for a legacy face selector and omitted the required tolerance. The syntax was corrected to the public FaceSelector contract with the already-frozen `1e-5 mm` tolerance. Original input/manifest and failed run remain archived. This changes neither geometry nor its acceptance oracle.
4. That corrected fixture exposed a real shared-validator defect: selection tolerance was checked by the minimum feature-size rule, rejecting valid `1e-5 mm` precision. Selection tolerance now has its own finite numerical range; geometry size limits are unchanged. Zero, negative and below-floor tolerances still reject.
5. The digest-rejection code was corrected against `validation.rs` before any native corpus execution; this preflight correction and original manifest are preserved.

No case was removed, no oracle geometry simplified and no acceptance tolerance widened. The frozen held-out set has now been observed during acceptance; it must not be described as unseen in a future G4 optimization campaign.

## G3 planned acceptance (after G2)

- Parameter edits and semantic named face split/merge/delete: new refs or explicit missing/ambiguity; old refs never rebound heuristically.
- Independently authored source replacements, arbitrary rotation, wrong pitch/missing holes and symmetric orientation controls; failed attempts preserve the accepted model.
- Real process termination, session reopen/relocation, source/artifact integrity, cache policy invalidation and bounded malformed-query/native-snapshot cases.
- Build Linux arm64 native from the tested source. Run inside one cgroup with `--cpus=1 --memory=512m --memory-swap=512m`; record actual `cpu.max`, `memory.max`, `memory.peak`, OOM counters, anonymous/file/cache accounting and descendants. Measure CAD plus small supervisor separately from resident Node agent/harness plus native. Do not substitute sampled RSS for cgroup peak.
- A live Pi agent uses local Colocad tools in that same sandbox, persists the actual inference/tool trace, builds from dimensions, queries and edits, then passes deterministic and independent readback checks. Scripted Pi tests do not count as live inference. This is a package adapter acceptance, not a claim to have deployed the production Abelien workflow runtime.

G4 benchmark/retrospective optimization is gated until G1–G3 have accepted evidence.

## G2 acceptance

**Accepted:** all **24/24** required cases pass (**18 positives, 6 correct rejections**), including all six originally held-out cases. [Final per-case and per-family evidence](g2-evidence/accepted.json), [source/native identities](g2-evidence/validation.json), [native regression](g2-evidence/native-final.log) and [Node regression](g2-evidence/node-final.log): **126 Rust and 36 Node tests pass**. Failed iterations and corrected verifier/fixture attempts remain in `g2-evidence/`. The numerical thresholds and geometric requirements are unchanged.

The no-material cut and selection-tolerance fixes are general engine-wrapper changes. The product case and all positive STEP artifacts pass independent OCCT validation. This closes the declared G2 corpus; G3 is now active. No performance comparison or G4 promotion has run.

**中文：**G2 冻结语料 24/24 通过，其中 18 个正例、6 个正确拒绝；原六个保留例也全部通过。126 项 Rust、36 项 Node 回归通过。保留失败和校正记录；容差与几何要求未放宽。此结论只覆盖公开语料，不代表所有工业 CAD 都可靠。继续 G3 生命周期与真实沙箱验收。
