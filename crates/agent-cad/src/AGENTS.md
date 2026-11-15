# /crates/agent-cad/src
> L2 | 父级: ../../../AGENTS.md

assembly.rs: Named instances and exact, explicitly requested assembly checks.
capabilities.rs: Kernel policy and advertised implementation capabilities.
compound.rs: Preserve independent solids, including overlapping copies; never fuse a pattern.
distance.rs: Exact topology distance adapter; no sampled-mesh distance fallback.
engineering.rs: Product structure operations: exact offsets, openings and fitting lids.
error.rs: 稳定错误码、来源字段和SolidCore错误适配。
exchange.rs: STEP boundary conventions, isolated from the pinned kernel sources.
executor.rs: Deterministic feature execution, exact validation and exports.
grid.rs: Direct analytic topology for proven rectangular through-hole stock.
hollow.rs: Closed cavities with the facade's explicit affine fallback policy and measured clearance.
incremental.rs: 版本内几何缓存、传递依赖摘要及恢复校验。
lib.rs: A bounded, replayable feature IR over a pinned exact B-rep kernel.
main.rs: 原生CLI命令分发、JSON回执和原子产物写出。
mates.rs: Local six-DOF rigid assembly solving, followed by exact B-rep checks.
model.rs: Declarative input contract and bounded scalar evaluation.
naming.rs: Semantic topology names carried by kernel copy/split/merge notifications.
network.rs: Tensor-product Gordon interpolation of intersecting polynomial guide curves.
operations.rs: Geometric operations with explicit units, frames and re-evaluated selectors.
placement.rs: Rigid placement shared by model instances, STEP and constraint solving.
regions.rs: Lazy exact inspection regions propagated along the feature DAG.
relationships.rs: Exact solid relationships. Boundary distance and material overlap are independent predicates.
sketch.rs: Parametric 2D constraints. Initial coordinates select a local solution branch.
solve.rs: Bounded deterministic Levenberg–Marquardt with numerical rank diagnostics.
step_assembly.rs: AP203/AP214 product instances with exact local B-rep definitions and rigid placements.
surfaces.rs: Tensor-product Bézier patches and exact parametric C2 bridge construction.
validation.rs: Schema, reference and resource validation, without kernel execution.

preview/: 原生显示网格、视图与预算处理。
query/: 精确几何快照、选择器、datum与查询合同。

[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
