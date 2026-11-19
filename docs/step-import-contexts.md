# STEP representation contexts

Colocad verifies every mapped body against an explicit representation context before accepting a file-wide multiple-context diagnostic. The selected context must agree with the mapper's already applied `unit_mm` and `uncertainty_mm`; a body referenced from incompatible contexts is rejected. This is a validation of the pinned mapper's per-body normalization contract, not a second scaling of the geometry.

This change supports multiple contexts sharing one length scale, including differences in plane-angle units and distance uncertainty. Geometry continues to be normalized by the pinned mapper; assembly frame directions remain dimensionless. The official inspect template retains definition sharing and the complete group/instance tree. Definition imports used by that template go through the same explicit-body coverage check.

Missing body context, conflicting ownership, skipped entities, dangling references, other diagnostics, and unapplied assembly transforms in ordinary part-import mode remain errors. An informational multiple-context diagnostic is accepted only after the body coverage checks. The standard assembly-count diagnostic is allowed only in the explicit definition/assembly modes that handle those instances.

The length-consistency gate considers representation contexts used by geometry or placements. An unreferenced plain `REPRESENTATION` containing only simple `DESCRIPTIVE_REPRESENTATION_ITEM` strings used exclusively by that representation is proven metadata and excluded, regardless of its context dimension. Unknown items, shape representations, and representations referenced by any entity (including maps and relationships) retain the length-unit check. Unknown consumers of the context or its descriptive items also prevent this exclusion. This applies to declared `GLOBAL_UNIT_ASSIGNED_CONTEXT` records; unitless UV parametric contexts keep their existing handling. Unused context declarations cannot supply a fallback for a body: explicit ownership and mapper normalization are still checked first.

After explicit body ownership is verified, files whose declared unit contexts all describe one valid length scale skip the inbound-reference scan. When the declaration can affect the result, inbound references are collected only for unit contexts and candidate descriptive metadata. Body ownership only indexes mapped bodies. Unknown consumers and incompatible length scales still fail the same checks.

The public `metadata-2d.step` regression is exactly the maintainer's drilled-block transformation: remove the single global uncertainty assignment and append three angular metadata entities. Removing those same three entities gives the control; the original public fixture remains unchanged. Both must retain one 20 × 16 × 10 mm solid and volume `3200 - 40π` mm³ through public SDK inspection, template build and independent OCCT WASM readback. Negative controls exercise geometry and unknown items in a 2D context, linked descriptive metadata, unknown references to the context/item/representation, placement-only mixed length and non-rigid mapping.

Mixed length scales such as millimetres and centimetres remain unsupported in this PR. The narrow compatibility fix does not certify different length scales at CDSR endpoints or representation maps. Independent OCCT observations of the experimental mixed-placement inputs disagreed with the hand-derived expectations; those local inputs and receipts remain investigation evidence, not supported behavior or an acceptance test. Follow-up mixed-unit support needs independent endpoint, rotation and sharing evidence before removing this refusal.

No UV policy, geometric tolerance, STEP byte/entity/instance budget, or pinned vendor source changes are included. A source that now passes inspection may still fail strict geometry validation; that later error is retained rather than reported as successful CAD acceptance.

Targeted native regressions use synthetic source only. Set `COLOCAD_UNIT_EVIDENCE_DIR` to an absolute ignored directory when running `cargo test -p agent-cad --test step_units` to retain the generated STEP inputs and native output evidence. Private hardware source files are not checked in.

After generating that evidence, run `node test/step-units-occt.mjs UNIT_EVIDENCE_DIR NEW_REPORT.json` for independent read-only OCCT WASM comparisons using the repository's locked dependencies. It includes the public OCCT nested assembly with only one uncertainty value changed: all coordinates, topology and placement records stay byte-identical, and both sources and delivered geometry must match the original world-space bounds. Append `metadata-2d,metadata-removed` as the third argument to audit the SDK-produced pair separately from the native suite outputs. The optional Python alternative is `test/step-units-occt.py` with an explicitly selected OCP environment and covers the earlier four synthetic controls. The older hand-built nested mapped input remains diagnostic and is not asserted to match two importers' placement semantics.


## Windows validation (2026-11-15)

The unchanged production code at `f8d8d47260f25e7dc0924e859beaaad476f8d8db` completed the Windows/WSL native workspace suite: 146 passed, zero failed across 27 test executables. The SDK context regression and TypeScript checks passed. The serial Node suite completed 53/54; the remaining streaming migration `SESSION_BUSY` failure also occurs on the baseline and is outside this change.

A later test-only generation adds the public nested-assembly control. Changing only uncertainty record #394 from `1.E-07` to `1.E-06` fails on baseline `c3200d59c326eb848bb76bfef1362e5899c78666` with `IMPORT_INCOMPLETE`, then passes with the unchanged fixed native production code. All 13 context tests pass. Six independent OCCT WASM controls pass; the public source, native export and fresh readback each have two valid solids, volume 2000 mm³ and bounds `[[80,0,0],[100,30,5]]`. The earlier hand-built mapped source remains diagnostic only.

Every runtime used two CPUs, 3 GiB RAM, zero swap, with no OOM. Native full-suite compilation used 8 GiB and is separate from the 3 GiB runtime result; the later test-only compilation succeeded within 3 GiB. The full 146-test suite was not repeated after adding the thirteenth context test: unchanged native-library cache and production-binary SHA were verified, and the complete 13-test context executable was rerun.

One private source now passes inspection with 30 definitions but still fails a later instance-16 `SmCone` geometry check (Tests 3/7). This is not whole-source CAD acceptance. Mixed-length support, general curved angle normalization, engineering fit and manufacturing remain outside the verified scope.

## Maintainer regression validation (2026-11-17)

Local fix source commit: `77f2a4aef74ae4e7e8f131dcbcad21365ccda8d8`. Linux x64 native SHA256: `ac2b6908aed3e238b9499aa0874680db454ba4675bde04fc6b313ca1381bbdd5`. The Windows frozen source manifest matches all 1134 tracked files in that commit. Validation evidence and the unsent reply draft are retained locally under `out/review-20261117/`.

The original PR native binary (production code identical to `be5e0875`) reproduces the maintainer's exact metadata failure at inspect, while removing the same three metadata entities passes inspect and template build. The fixed public SDK path passes both byte-identical inputs. Independent pinned OCCT WASM confirms one valid solid, 20 × 16 × 10 mm and `3200 - 40π` mm³ for both sources and SDK exports.

All 20 context tests pass, including the original 13 retained byte-for-byte and seven added regressions. Four SDK context tests, eight native OCCT control groups, the separate two SDK OCCT controls, SDK TypeScript compilation and query type checks pass. The older hand-built mapped source remains diagnostic only; its output is checked without claiming that its raw source placement agrees between importers.

The complete Rust workspace release suite passes 154 tests across 27 executables, with no failures, ignored tests or filtering. The workspace doc-test command passes with zero doc-tests. The serial SDK suite passes 56/57; the sole streaming-migration `SESSION_BUSY` failure is reproduced once with the original PR binary and unchanged test. The migration catalog executable is byte-identical before/after; this separate failure is retained, not counted as a full SDK pass.

All compilation and runtime jobs used the existing shared owner/lane gate, two CPUs, 3 GiB RAM and zero swap, with no OOM. Compilation used a private copy of the existing B4 warm target and registry; no other repair cache was modified. This is Windows/WSL Linux x64 validation, not native Windows PE, macOS, CI or manufacturing acceptance. Mixed length and broader angle/UV semantics remain outside this narrow fix. No private CAD, paid model requests, vendor changes, remote publication or GitHub writes are part of this validation.

## Rebased release 0.1.1 validation (2026-11-17)

The sections above retain historical pre-rebase evidence; their native SHA and 154-test count are not evidence for this generation. The old complete branch is preserved at `backup/pr8-before-release011-rebase-7359068-1117` and the old 390-file evidence manifest remains unchanged.

The complete PR branch was rebased from `c3200d59c326eb848bb76bfef1362e5899c78666` onto `4124bcb769825895313900a4be9cb1e3610d9512`. Tested source commit: `542428bee4248e7dc45ef53bd66a5e90dfcb6c1d`. New Linux x64 native SHA256: `6b4cc116ed69dd438c2070531ca9ce112b5fa5f1eb74a70216d72a3f05aefb22`. All 1205 frozen tracked files match this commit; evidence is separate under `out/review-20261117-release011/`.

Conflict resolution keeps the release Writer/outputStepBytes code and inspect host-limit entry byte-identical. Definition imports keep the release ImportCache admission by verified SHA and mode; only a cache miss delegates to the context-checked definition mapper. Resource/executor/main/incremental/cache/SDK/session code and Cargo/package/vendor locks remain identical to the new base. The vendor is freshly derived from the pinned archive plus only the three release-locked patches, verified by the official vendor script; pending PR3/10 changes are not included.

On this new source, all 20 context tests, four SDK context tests, eight native OCCT groups, the separate two SDK OCCT controls, and SDK/query/resource/output TypeScript checks pass. The full release workspace executes 188 tests across 33 executables, with zero failures, ignored or filtered tests. The workspace doc-test command passes with zero doc-tests. New byte-budget, cache and planar-conic regressions remain in the full suite.

The single new-source serial SDK run is 91/92. Its only failure is the streaming-migration SESSION_BUSY assertion. Test/session/catalog source and Cargo lock match the new main commit; the local catalog SHA is `c69e2246bd7447a08a0b99a78d9387ed488eb9826a8cf638ccdf66fb177b7e1a`. The older native control may be cited only as older-source evidence: no new main runtime baseline was run, and no SESSION_BUSY retry was made. Formal main binary equivalence is not asserted. The local evidence contains the complete source/hash matrix and a separate static published-package comparison.

All compilation/runtime jobs remain on Windows/WSL under the shared owner gate at 2 CPUs, 3 GiB and zero swap, without OOM or timeouts. Mac performs only editing, textpacks, hashes and Git. All PR8 jobs have exited; mixed length, broader angle/UV semantics, native Windows PE/macOS and remote CI remain outside the verified scope. No push, GitHub write, paid model request, private CAD test or fallback engine is part of this validation.

[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
