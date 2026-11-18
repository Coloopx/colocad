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
operations.rs: 显式单位/坐标/选择器的几何操作；definition复用完整几何/单位准入，assembly与普通导入保留完整位姿门禁。
placement.rs: Rigid placement shared by model instances, STEP and constraint solving.
regions.rs: Lazy exact inspection regions propagated along the feature DAG.
relationships.rs: Exact solid relationships. Boundary distance and material overlap are independent predicates.
sketch.rs: Parametric 2D constraints. Initial coordinates select a local solution branch.
solve.rs: Bounded deterministic Levenberg–Marquardt with numerical rank diagnostics.
step_import_limits.rs: 宿主显式STEP实体/定义/occurrence/group策略、硬顶、结构化拒绝、query实际叶祖先链513路径预检、同步作用域及with_import_limits公共API。
step_assembly.rs: 共用实体/定义、逐 body 单位上下文和诊断准入；单定义止于本地几何，整装配另核 occurrence/group 并按表示上下文展开位姿。
surfaces.rs: Tensor-product Bézier patches and exact parametric C2 bridge construction.
validation.rs: Schema, reference and resource validation, without kernel execution.

preview/: 原生显示网格、视图与预算处理。
query/: 精确几何快照、选择器、datum与查询合同。

step_assembly/: body归属/归一证明和representation尺度下的刚体解析，见step_assembly/AGENTS.md。

[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
resources.rs: 宿主显式字节策略、固定硬上限和结构化超限回执；模型不能抬高预算。
outputStepBytes贯通resources、executor、assembly及step_assembly，main同目录原子发布model.step.sha256；capabilities仅增加输出合同，原PR6几何/数量/单位不变。

reportBytes：resources有界计量，main发布前双准入，executor记录checkpoint_recovery；incremental超大旧manifest只冷建。query/engine只在PR6原算法替换报告容量，不带D1 progress/preflight/计数策略。

import_cache.rs: 单次构建mode+verifiedSHA单槽准入；definition只取有界本地定义，part仍完整展开/拒绝，逐件body选择不缓存。
2026-11-15：外部STEP统一预算，内部生成预算独立；checkpoint绑定计数策略，扩展plain装配不扩大128快照/显示及8128精检对容量。
progress.rs: 显式启用的stderr结构化特征/阶段事件，不发送定时心跳。

early_checks.rs: 固定位姿下依赖就绪的精确碰撞早检，mate模型仍在求解后检查，最终收口复用原结果。
