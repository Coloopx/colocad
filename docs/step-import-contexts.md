# STEP representation contexts

Colocad verifies every mapped body against an explicit representation context before accepting a file-wide multiple-context diagnostic. The selected context must agree with the mapper's already applied `unit_mm` and `uncertainty_mm`; a body referenced from incompatible contexts is rejected. This is a validation of the pinned mapper's per-body normalization contract, not a second scaling of the geometry.

This change supports multiple contexts sharing one length scale, including differences in plane-angle units and distance uncertainty. Geometry continues to be normalized by the pinned mapper; assembly frame directions remain dimensionless. The official inspect template retains definition sharing and the complete group/instance tree. Definition imports used by that template go through the same explicit-body coverage check.

Missing body context, conflicting ownership, skipped entities, dangling references, other diagnostics, and unapplied assembly transforms in ordinary part-import mode remain errors. An informational multiple-context diagnostic is accepted only after the body coverage checks. The standard assembly-count diagnostic is allowed only in the explicit definition/assembly modes that handle those instances.

Mixed length scales such as millimetres and centimetres remain unsupported in this PR. The narrow compatibility fix does not certify different length scales at CDSR endpoints or representation maps. Independent OCCT observations of the experimental mixed-placement inputs disagreed with the hand-derived expectations; those local inputs and receipts remain investigation evidence, not supported behavior or an acceptance test. Follow-up mixed-unit support needs independent endpoint, rotation and sharing evidence before removing this refusal.

No UV policy, geometric tolerance, STEP byte/entity/instance budget, or pinned vendor source changes are included. A source that now passes inspection may still fail strict geometry validation; that later error is retained rather than reported as successful CAD acceptance.

Targeted native regressions use synthetic source only. Set `COLOCAD_UNIT_EVIDENCE_DIR` to an absolute ignored directory when running `cargo test -p agent-cad --test step_units` to retain the generated STEP inputs and native output evidence. Private hardware source files are not checked in.

[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
