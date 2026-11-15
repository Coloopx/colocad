# crates/agent-cad/tests/
> L2 | 父级: ../../../AGENTS.md

成员清单
assembly_tree.rs: 既有源码/说明，本轮行为不变。
capabilities.rs: 既有源码/说明，本轮行为不变。
datums.rs: 既有源码/说明，本轮行为不变。
fixtures/: 公开STEP原件及只在Win运行的17度seam正/负源生成器，见局部地图。
foreign_queries.rs: 既有源码/说明，本轮行为不变。
geometry.rs: 既有源码/说明，本轮行为不变。
grid.rs: [test]
hollow.rs: [test]
interfaces.rs: 既有源码/说明，本轮行为不变。
level1_nesting.rs: Exercise the native predicate on corrupted planar trims, not a feature-level rejection.
mates.rs: 既有源码/说明，本轮行为不变。
mates_alignment.rs: SDD 0004 E01: outward surface normals are not automatically aligned mate axes.
naming.rs: [test]
network.rs: [test]
parity.rs: 既有源码/说明，本轮行为不变。
preview.rs: [test]
query_artifacts.rs: 既有源码/说明，本轮行为不变。
query_kernel_probe.rs: SDD 0004 E03: kernel feasibility assertions, not acceptance of a public query API.
regions.rs: 既有源码/说明，本轮行为不变。
relationships.rs: 既有源码/说明，本轮行为不变。
sketch.rs: 既有源码/说明，本轮行为不变。
snapshot_adversarial.rs: Deterministic parser probes with valid recomputed metadata hashes: these reach；the native parser rather than stopping at the public integrity check.
step_planar_conic_delivery.rs: 完整闭合STEP import/build/export/readback与严格负例，等价需Win独立OCP。
step_planar_conic_domain.rs: G2平面conic连续域与严格UV gap负例；只在Windows验收。
surfaces.rs: 既有源码/说明，本轮行为不变。
validation_properties.rs: Compare the original solver and the scoped assembly optimization on the same inputs.

变更：2026-11-15 G2补丁与Windows定向回归；不修改其它算法。

[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
