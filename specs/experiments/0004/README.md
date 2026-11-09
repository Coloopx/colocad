# SDD 0004 implementation and experiment ledger

Date: 2026-11-09. Specification: [English revision 2](../../0004-engineering-query-and-assembly-interfaces.md), [中文](../../0004-engineering-query-and-assembly-interfaces.zh-CN.md).

The six review findings are incorporated in the normative spec. Work is local to Colocad. Runtime evidence below is independent of the existing harness CAD implementation. No npm release is implied by this ledger.

| Experiment | Hypothesis / gate | State |
| --- | --- | --- |
| E01 / D02 | Explicit mate direction and proper frame rotation support outward opposing surfaces without changing legacy defaults | Passed for explicit numeric attachments; implemented in Rust, schema and SDK types |
| E02 / Q04 | Exact native serialization preserves assigned topology identity and semantic names without using reloaded arena positions as identity | Definition-level feasibility passed; complete query manifest/reference acceptance pending |
| E03 / Q01/Q03 | Pinned kernel provides exact analytic descriptors and trim-aware evaluation | Planar area, trim, cylinder seam and reload probes passed; public descriptor/evaluation API pending |
| E04 / S01/D01 | A bounded selection can derive primitive datums without hole recognition | Pending |
| E05 / D03/C01 | Datum mates and measured checks establish basic assembly/clearance | Pending |
| E06 / A01–A06 | Independently authored replacement and KK60 assembly fixtures | Pending; requires preceding gates |

For each experiment retain the failing command/result, repair, passing command/result, source/native identity and scope. Use analytic assertions for geometry and independent OCCT readback for significant STEP deliveries. Deterministic tests are not live-agent success rates. No latency/memory advantage is claimed before Linux sandbox measurements.

Development native builds use `CARGO_TARGET_DIR` outside the repository, and SDK tests use `AGENT_CAD_BINARY`; packaged multi-platform binaries remain release artifacts. Never treat testing an older packaged binary as testing new source.

## Iteration 1 — implemented and verified

The reviewed baseline is `356aff8`; [runtime evidence](evidence/sdk-run.json) records source SHA-256 values, the tested native SHA-256, pinned kernel revision and host environment. This iteration ran on **macOS arm64**, not the target Linux sandbox.

| Check | Result | Evidence |
| --- | --- | --- |
| Rust regression | 87 passed, 0 failed, 0 ignored; includes 15 new tests | [Native log](evidence/native-tests.log) |
| Root Node regression | 29 passed; new SDK shaft/support test included | [Node log](evidence/node-tests.log) |
| Harness examples | 6 passed with the development native override; examples retain their pinned published alpha.3 SDK | [Integration log](evidence/integrations.log) |
| Generated contract | Rust JSON schema and TypeScript regenerated; `schema:check` and SDK compilation passed | [Validation record](evidence/validation.json) |
| Independent STEP | OCCT 7.9.3.1.1: 2 valid solids; expected analytic volumes; gap `0.1999999872045466 mm` versus `0.2 mm` expected | [Readback](evidence/occt-readback.json) |

**E01.** The [initial failure](evidence/e01-red.log) was five rejected `alignment` fields, with the legacy-default control passing. The implemented contract adds aligned/opposed/unoriented axis policies, aligned/opposed full-frame policies and explicit twist. Eight new Rust tests cover defaults, rejection, proper opposed rotations, perturbed-pose convergence, branch reporting and remaining DOF. Existing numeric frames serialize without extra default fields. Physical residuals are reported in mm/rad separately from weighted solver residuals. The local solver still does not prove global uniqueness.

The runnable [shaft/support fixture](shaft-support.json) starts with translation/rotation errors. Opposed concentric plus opposed plane constraints converge in three iterations and leave one rotational DOF. The independent readback checks transformed exact geometry, two volumes and radial clearance. Increasing shaft radius from 2.8 to 3.2 mm is rejected with `ASSEMBLY_COLLISION`. This fixture deliberately uses **explicit numeric frames**: it is not evidence of implemented geometry-backed datums, arbitrary part replacement, or the new clearance-query API.

**E02.** Three [initially failing tests](evidence/e02-red.log) now pass. The experimental Rust module writes exact native geometry and assigned entity tokens into native topology records, with names in a hash-bound sidecar. A fresh kernel recovers tokens from those records and checks kind/count/uniqueness; it never binds names using the reload traversal order. Named box and analytic bore fixtures retain geometry; mutated bytes, changed name bindings and incompatible versions are rejected. This proves the chosen serialization path on these fixtures. It does not yet implement occurrence manifests, host path confinement, lifecycle/versioned refs, hostile-payload parser validation or public SDK/CLI query entry points.

**E03.** The [initial kernel probe](evidence/e03-initial-probe.log) exposed an adapter assumption: a successful native face classifier returns `Unknown` for outside the trimmed face. The new wrapper maps that specific successful result to `outside_trim`; kernel errors still propagate and non-finite UVs fail. See the wrapper's [red phase](evidence/e03-red.log) and passing native log. The Q01 analytic plate has through, blind and counterbored openings: top area `6000 − 59π mm²`, one outer loop and three inner loops. Inside material, hole interior, boundary and outside-domain probes remain correct after exact reload. A cylinder's seam retains one edge used twice with opposite orientations. These tests do not perform hole recognition.

The probes also confirmed that the native surface closest-point/evaluation routines can clamp to their natural domain. The outside-plane probe therefore uses explicit infinite-plane UV projection and an out-of-bounds evaluation on a disposable surface copy before trim classification. Public `evaluate_face` must preserve this distinction rather than quietly change the requested point.

## Reproduce

From the repository root, prepare the pinned vendor sources with `node scripts/vendor.mjs` if absent. Use a fresh build and keep generated binaries outside the checkout if desired:

```sh
export CARGO_TARGET_DIR="$PWD/../../work/colocad-build/target"
cargo test --locked --release -p agent-cad
cargo build --locked --release -p agent-cad
export AGENT_CAD_BINARY="$CARGO_TARGET_DIR/release/agent-cad"
npm run schema
npm install --prefix kernel --ignore-scripts
npm run build --prefix kernel
npm run schema:check
npm test
node specs/experiments/0004/run.mjs out/experiments/0004
```

Run the independent validator in a separate environment containing `cadquery-ocp-novtk==7.9.3.1.1` (the Rust package has no Python runtime dependency):

```sh
python specs/experiments/0004/verify-step.py out/experiments/0004/run.json
```

`run.json` contains an output-relative artifact path for that run. The archived [sdk-run.json](evidence/sdk-run.json) preserves the historical receipt; regenerate outputs to reproduce the STEP rather than interpreting its path relative to the evidence directory. `verify-step.py` checks the STEP digest before geometry. Red logs capture actual initial failures; their stub line numbers are historical.

## Next iteration and gates

M0 is **partially complete**. Do not mark the whole milestone accepted before testing payload/traversal budgets and the Linux 1 vCPU / 512 MiB resource envelope. No new performance ratio is established by this iteration.

1. Complete snapshot authority: full manifest/digest vectors, ownership and definition/occurrence mappings, reopen/relocation and hostile/stale input rejection. Use the existing definition snapshot proof; do not substitute STEP import or preview meshes.
2. E04/M1: add bounded selection and primitive datum derivation. Start with `plane → inner loops → circle edges → centers`, a second plane exclusion, tied candidates, singular frames and a rotated imported part. Record failures before implementing; expose only supported operations through SDK/CLI/tool.
3. E05/M2: consume those datums in mates; test remaining DOF, point/plane/axis relations, 0.2 mm gap, touching, overlap and containment, then region-specific contact exceptions. The current numeric-frame fixture is a control, not a replacement for these tests.
4. E06/M3: independently authored flange/support STEP variants and KK60 mounting fixtures, with incompatible and symmetric controls. Source/license/hash and physical acceptance thresholds precede implementation or timing.

**中文结论：**六项评审意见已回归双语 spec；第一轮已实现配合方向修复，并验证精确快照和裁剪查询的基础路径。87 项 Rust、29 项 Node、6 项集成例子通过，独立 OCCT 回读确认 0.2 mm 间隙。通用选择、几何基准驱动装配、孔/接口识别、完整查询入口及 Linux 资源验收仍待后续迭代，SDD 0004 未整体完成，本次未发 npm。
