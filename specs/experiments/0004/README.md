# SDD 0004 implementation and experiment ledger

Date: 2026-11-09. Specification: [English revision 2](../../0004-engineering-query-and-assembly-interfaces.md), [中文](../../0004-engineering-query-and-assembly-interfaces.zh-CN.md).

The six review findings are incorporated in the normative spec. Work is local to Colocad. Runtime evidence below is independent of the existing harness CAD implementation. No npm release is implied by this ledger. [SDD 0005](../../0005-capability-reliability-and-optimization.md) governs the four ordered acceptance goals; G1 remains active.

| Experiment | Hypothesis / gate | State |
| --- | --- | --- |
| E01 / D02 | Explicit mate direction and proper frame rotation support outward opposing surfaces without changing legacy defaults | Passed for explicit numeric attachments; implemented in Rust, schema and SDK types |
| E02 / Q04 | Exact native serialization preserves assigned topology identity and semantic names without using reloaded arena positions as identity | Definition-level feasibility and public manifest/reopen tests passed; broader parser/resource acceptance pending |
| E03 / Q01/Q03 | Pinned kernel provides exact analytic descriptors and trim-aware evaluation | Probes plus public descriptor/evaluation subset passed; specialized coverage remains incomplete |
| E04 / S01/D01 | A bounded selection can derive primitive datums without hole recognition | Core slice implemented; relative datum filters and remaining descriptor contract pending |
| E05 / D03/C01 | Datum mates and measured checks establish basic assembly/clearance | Datum-driven foreign STEP fixture and OCCT 0.2 mm readback passed; full contact/clearance API acceptance pending |
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

## Iteration 1 handoff (historical)

M0 is **partially complete**. Do not mark the whole milestone accepted before testing payload/traversal budgets and the Linux 1 vCPU / 512 MiB resource envelope. No new performance ratio is established by this iteration.

1. Complete snapshot authority: full manifest/digest vectors, ownership and definition/occurrence mappings, reopen/relocation and hostile/stale input rejection. Use the existing definition snapshot proof; do not substitute STEP import or preview meshes.
2. E04/M1: add bounded selection and primitive datum derivation. Start with `plane → inner loops → circle edges → centers`, a second plane exclusion, tied candidates, singular frames and a rotated imported part. Record failures before implementing; expose only supported operations through SDK/CLI/tool.
3. E05/M2: consume those datums in mates; test remaining DOF, point/plane/axis relations, 0.2 mm gap, touching, overlap and containment, then region-specific contact exceptions. The current numeric-frame fixture is a control, not a replacement for these tests.
4. E06/M3: independently authored flange/support STEP variants and KK60 mounting fixtures, with incompatible and symmetric controls. Source/license/hash and physical acceptance thresholds precede implementation or timing.

**中文结论：**六项评审意见已回归双语 spec；第一轮已实现配合方向修复，并验证精确快照和裁剪查询的基础路径。87 项 Rust、29 项 Node、6 项集成例子通过，独立 OCCT 回读确认 0.2 mm 间隙。通用选择、几何基准驱动装配、孔/接口识别、完整查询入口及 Linux 资源验收仍待后续迭代，SDD 0004 未整体完成，本次未发 npm。


## Iteration 2 — query artifacts, selection and persistent datums

G1 is **in progress**; this is a tested implementation slice, not full M1/M2 or G1 acceptance. [Source/native hashes and environment](iteration-2/validation.json) identify the tested working tree based on `b67d4ba`. Packaged native binaries and npm alpha.6 are unchanged. Logs normalize local path prefixes only.

| Check | Result | Evidence |
| --- | --- | --- |
| Rust regression | 105 passed, 0 failed, 0 ignored (18 new tests) | [Native log](iteration-2/native-tests.log) |
| Root Node regression | 34 passed, 0 failed (5 new tests) | [Node log](iteration-2/node-tests.log) |
| Agent harness examples | 6 passed; pinned published example SDK with development native override | [Integration log](iteration-2/integrations.log) |
| Offline browser | Query step retains scene; geometry step remains reachable; controls, downloads, mobile layout and no remote requests pass | [Browser log](iteration-2/viewer.log) |
| Generated contracts | Rust schemas, SDK compilation, positive/negative TypeScript query fixture and `schema:check` pass | [Validation](iteration-2/validation.json) |
| Independent OCCT | 2 valid exported solids, analytic volumes, minimum gap **0.2000000000040704 mm** | [Readback](iteration-2/occt-readback.json) |

**Queryable artifact authority.** A requested snapshot is part of atomic build output. The versioned manifest binds exact native payloads, topology/name bindings, final definition ownership, solved occurrence placements, model identity and declared datum results. Fresh process and relocated-directory queries preserve refs; changed geometry rejects old refs; corrupt payloads, escaping paths and symlinks fail. A [fixed encoding/SHA vector](fixtures/manifest-vector.json) tests compact serializer bytes. Legacy artifacts require explicit queryable rebuild. [Initial red tests](iteration-2/artifacts-red.log) failed because the CLI did not expose snapshot/query entry points.

**Selection and evaluation.** The public API supports face/edge/vertex/loop descriptors, bounded set expressions and typed filters, shared-child adjacency, tied ranks, explicit cardinality and cursor identity. `plane → inner loops → circles → centers` excludes the opposite face. Tests exposed an adapter bug: native face-use normals point into their owning region, so the material-side sign must be inverted for engineering outward normals. The [red selection log](iteration-2/selection-red.log) and top-plane/foreign-STEP regressions retain that finding. `evaluate_face` distinguishes trimmed material, hole interior and outside parameter domain, and avoids fabricated zero-tangent normals.

**Persistent datum intent.** [Initial tests](iteration-2/datums-red.log) rejected `Model.datums`; the source now accepts bounded `selected`, explicit `design_point`, `point_difference` and right-handed `frame` recipes. Earlier named dependencies are required, revision-local refs are prohibited in persistent recipes, and primitive cardinality is exactly one. Attachments consume datum kinds appropriate to the mate. The shaft/support fixture follows a height edit and retains one rotational DOF; fixed frame tests constrain all six DOF and reject degeneracy. Wrong kind, ambiguity, missing dependencies, mutually exclusive frame/datum and explicitly disabled snapshots fail clearly.

**Foreign STEP closure.** The [runnable example](../../../examples/engineering-queries/README.md) uses the [independently authored OCCT flange](fixtures/occt-flange.json), not a Colocad-exported input. Bore and plane recipes drive the shaft placement from perturbed seed poses. An oversized shaft fails `ASSEMBLY_COLLISION`. The [run receipt](iteration-2/run.json) and independent OCCT readback validate the resulting exact geometry. The read-only named query has zero geometry build/export/tessellation calls. This small analytic fixture does not establish general hole/interface recognition or an industrial success rate.

**SDK, CLI and session.** `CadClient.query`, generated query schema, capability-gated `cad_query`, queryable build/record CLI and immutable receipt output are available in development source. Sessions retain query input/receipt hashes, pin accepted source artifact hashes, preserve queryability through edits and keep failed queries separate from accepted source. [Guide](../../../docs/engineering-queries.md) / [中文](../../../docs/engineering-queries.zh-CN.md).

### Reproduce iteration 2

After the development build/schema/SDK preparation above:

```sh
cargo test --locked --release -p agent-cad
npm test
npm run test:integrations
npm run test:viewer
node examples/engineering-queries/run.mjs out/engineering-queries
python examples/engineering-queries/verify.py out/engineering-queries/run.json
```

`AGENT_CAD_BINARY` must identify this checkout's freshly built native binary for all Node/browser/example commands. Run the last command in the independent OCCT environment. Archived paths in `iteration-2/run.json` are historical; rerun the example to produce artifacts to inspect. Type-contract check: `tsc --noEmit --module NodeNext --target ES2022 --skipLibCheck test/types/query.mts` with Node types installed.

### Remaining ordered work

G1 still requires named-datum-relative predicates, the full specialized descriptor/trim contract, hole/counterbore recognition, planar hole groups, installation interfaces and contact/clearance semantics. Broader parser/resource/reference replacement evidence remains open. G2 then establishes a frozen difficult/foreign industrial corpus; G3 accepts engineering lifecycle and Linux 1 vCPU / 512 MiB limits; G4 begins the specified benchmark/trace-back/minimize/fix/regress/held-out/accept-or-reject loop only after their evidence passes. No speed or memory ratio is claimed from these macOS correctness tests.

**中文状态：**第二轮完成了可查询精确快照、核心通用选择、持久化几何基准与配合、SDK/CLI/session 接入，并用独立 OCCT STEP 完成最小装配闭环。105 项 Rust、34 项 Node、6 项集成测试和离线浏览器验收通过；独立回读确认 0.2 mm 间隙。G1 尚未整体完成，G2–G4 继续受顺序闸门约束。

## Iteration 3 — G1 accepted within the supported contract

**G1 functional coverage is accepted. G2 and G3 remain separate gates.** This acceptance covers the explicitly supported SDD 0004 domains and exact failure outcomes, not complete CadQuery/build123d parity. [Validation and source/native hashes](iteration-3/validation.json) bind the tested development tree; published alpha.6 bundles are unchanged.

| Acceptance rows | Evidence and outcome |
| --- | --- |
| Q01–Q03 / Q06–Q07 | Exact descriptors, seams/pcurves, orientation, through/blind/counterbore recognition, split axial/angular foreign bores; shafts excluded, side-open/intersecting/conical/NURBS recognition partial. Rust `query_kernel_probe`, `query_artifacts`, `foreign_queries`. |
| Q04 / Q09 / S01 | Snapshot identity, relocated artifacts, corruption/stale refs, tied cardinality, stable pagination and composable topology traversal pass. Edit/split/replacement sequences are extended under G3. |
| Q08 / Q10 | Rectangular/circular/linear groups, extra holes, requested-size linear subsets, symmetry, native declarations vs foreign recognition. Native and imported descriptor facts agree in SDK tests. |
| S02 / D01–D04 | Relative datum predicates, coplanar selection, geometry-derived keyed orientation after arbitrary import rotation, typed mates, proper opposed frames, missing/ambiguous/degenerate controls and remaining DOF. |
| A01–A05 | Same geometry-backed recipe accepts base/housing/rotated foreign sources; wrong pitch, missing holes, incorrect normal/register and unspecified symmetric full-frame orientation are rejected. All-hole physical correspondence and exact pair checks retained. |
| A06 / C01–C02 | Complete KK60 + external bracket: 82 solids, three pairs at three travel positions independently verified. Separated/contact/overlap/containment controls and supported planar contact proof pass; collision elsewhere is never exempted. |
| I01–I02 / P01 baseline | SDK/CLI/tool/session entry points, generated schemas/types, immutable query history, actual native-ref overlays, offline browser pass. Queries report zero feature rebuild/export/tessellation. Linux admission/resource acceptance remains G3. |
| R01 | **124 Rust, 36 Node, 6 scripted integration tests**, offline browser and generated-contract checks pass. [Rust](iteration-3/native-tests.log), [Node](iteration-3/node-tests.log), [integrations](iteration-3/integrations.log), [viewer](iteration-3/viewer.log). |

The [external run](iteration-3/external-runs.json) has three positives and two expected rejections, independently checked by [OCCT](iteration-3/external-occt-readback.json). The [whole-product run](iteration-3/kk60-run.json) and [OCCT readback](iteration-3/kk60-occt-readback.json) retain nine actual geometry samples. These are correctness runs on macOS, with no new speed/memory claim and no live-agent success-rate claim.

### Failures retained and general repairs

- [Linear subset red](iteration-3/linear-subset-red.log): a five-hole line returned no four-hole candidate. The recognizer now enumerates bounded requested-length windows, preserving excluded holes and symmetry.
- [Plane-scope red](iteration-3/scoped-recognition-red.log): unrelated lower recesses incorrectly blocked a valid top installation group. Coverage now follows requested support adjacency; a recess reaching that support still prevents complete recognition. Diagnostics outside the support remain visible.
- [Coplanar red](iteration-3/coplanar-red.log): same-normal support and blind-hole bottom were ambiguous. A named-plane coplanarity predicate checks both position and angle; no first-face fallback was added.
- [Contact](iteration-3/contact-red.log), [display](iteration-3/display-red.log) and [viewer](iteration-3/viewer-red.log) failures precede the corresponding implementations. Contact certification requires a complete polyhedral support proof; guides are sampled display data bound to native tokens and hashes, not geometric authority.
- The initial foreign NURBS volume oracle used OCCT default quadrature, which differed from the known analytic cylinder volume. The verifier now explicitly calls adaptive integration with keyword `Eps=1e-10` and checks the analytic oracle. The frozen relative-volume tolerance **1e-6** and existing STEP hashes were not relaxed. This is an oracle correction, not a kernel accuracy improvement.

Unsupported required claims fail: curved contact-region certification, region-restricted relationships, composite mounting planes, unsupported hole geometries or precision below the published numerical floor. Definition AABBs transformed into assembly space are explicitly conservative. No thread/load/OEM or continuous-motion certification follows from these fixtures.

Reproduce the full suite with the development native override as above, then follow [the external/KK60 example](../../../examples/engineering-interfaces/README.md). G2 now freezes a broader geometry corpus and independent acceptance targets before executing it. G3 follows with lifecycle, malformed input, real inference/tool traces and actual Linux cgroup accounting.

**中文验收：**G1 在明确的支持域内完成并通过：124 项 Rust、36 项 Node、6 项脚本集成测试及离线浏览器；外部 STEP 三个成功/两个正确拒绝；完整 KK60 加附件 82 实体、九项接触/间隙独立回读通过。G2/G3 不能由这些结果代替，仍按顺序推进。
