# STEP representation contexts

Colocad verifies every mapped body against an explicit representation context before accepting a file-wide multiple-context diagnostic. The selected context must agree with the mapper's already applied `unit_mm` and `uncertainty_mm`; a body referenced from incompatible contexts is rejected. This is a validation of the pinned mapper's per-body normalization contract, not a second scaling of the geometry.

This change supports multiple contexts sharing one length scale, including differences in plane-angle units and distance uncertainty. Geometry continues to be normalized by the pinned mapper; assembly frame directions remain dimensionless. The official inspect template retains definition sharing and the complete group/instance tree. Definition imports used by that template go through the same explicit-body coverage check.

Missing body context, conflicting ownership, skipped entities, dangling references, other diagnostics, and unapplied assembly transforms in ordinary part-import mode remain errors. An informational multiple-context diagnostic is accepted only after the body coverage checks. The standard assembly-count diagnostic is allowed only in the explicit definition/assembly modes that handle those instances.

Mixed length scales such as millimetres and centimetres remain unsupported in this PR. The narrow compatibility fix does not certify different length scales at CDSR endpoints or representation maps. Independent OCCT observations of the experimental mixed-placement inputs disagreed with the hand-derived expectations; those local inputs and receipts remain investigation evidence, not supported behavior or an acceptance test. Follow-up mixed-unit support needs independent endpoint, rotation and sharing evidence before removing this refusal.

No UV policy, geometric tolerance, STEP byte/entity/instance budget, or pinned vendor source changes are included. A source that now passes inspection may still fail strict geometry validation; that later error is retained rather than reported as successful CAD acceptance.

Targeted native regressions use synthetic source only. Set `COLOCAD_UNIT_EVIDENCE_DIR` to an absolute ignored directory when running `cargo test -p agent-cad --test step_units` to retain the generated STEP inputs and native output evidence. Private hardware source files are not checked in.

After generating that evidence, run `node test/step-units-occt.mjs UNIT_EVIDENCE_DIR NEW_REPORT.json` for independent read-only OCCT WASM comparisons using the repository's locked dependencies. It includes the public OCCT nested assembly with only one uncertainty value changed: all coordinates, topology and placement records stay byte-identical, and both sources and delivered geometry must match the original world-space bounds. The optional Python alternative is `test/step-units-occt.py` with an explicitly selected OCP environment and covers the earlier four synthetic controls. The older hand-built nested mapped input remains diagnostic and is not asserted to match two importers' placement semantics.


## Windows validation (2026-11-15)

The unchanged production code at `f8d8d47260f25e7dc0924e859beaaad476f8d8db` completed the Windows/WSL native workspace suite: 146 passed, zero failed across 27 test executables. The SDK context regression and TypeScript checks passed. The serial Node suite completed 53/54; the remaining streaming migration `SESSION_BUSY` failure also occurs on the baseline and is outside this change.

A later test-only generation adds the public nested-assembly control. Changing only uncertainty record #394 from `1.E-07` to `1.E-06` fails on baseline `c3200d59c326eb848bb76bfef1362e5899c78666` with `IMPORT_INCOMPLETE`, then passes with the unchanged fixed native production code. All 13 context tests pass. Six independent OCCT WASM controls pass; the public source, native export and fresh readback each have two valid solids, volume 2000 mm³ and bounds `[[80,0,0],[100,30,5]]`. The earlier hand-built mapped source remains diagnostic only.

Every runtime used two CPUs, 3 GiB RAM, zero swap, with no OOM. Native full-suite compilation used 8 GiB and is separate from the 3 GiB runtime result; the later test-only compilation succeeded within 3 GiB. The full 146-test suite was not repeated after adding the thirteenth context test: unchanged native-library cache and production-binary SHA were verified, and the complete 13-test context executable was rerun.

One private source now passes inspection with 30 definitions but still fails a later instance-16 `SmCone` geometry check (Tests 3/7). This is not whole-source CAD acceptance. Mixed-length support, general curved angle normalization, engineering fit and manufacturing remain outside the verified scope.

[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
