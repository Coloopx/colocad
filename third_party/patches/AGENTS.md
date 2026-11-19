# third_party/patches/
> L2 | 父级: ../AGENTS.md

成员清单
README.md: Downstream SolidCore changes
face37.patch: 固定配置/许可/构建输入；沿现有治理消费。
planar-conic-domain.patch: G2最小step-import补丁，锁定before/after及patch SHA。
validation-properties.patch: 固定配置/许可/构建输入；沿现有治理消费。

变更：2026-11-19 冷路径零支撑端点重建；端节点诊断仅失败路径。
变更：2026-11-19 球面sense外提且单壳几何表move-not-clone；Linux fuzz=0与锁定摘要同步。

periodic-seam-trims.patch: 在构建 STEP 圆柱面前同步有限参数域，修复周期接缝回读。

step-brep-end-poles.patch: 拓扑重建过程删除精确零支撑 B 样条端点基函数。
变更：2026-11-19 uv-metric-regularity 将 mapped-trim/regularity 移出主校验CGU。

periodic-seam-trims.patch: 在构建 STEP 圆柱面前同步有限参数域，修复周期接缝回读。

uv-metric-regularity.patch: 对实际毫米映射曲线重算原保守界；identity不分配映射克隆；mapped regularity 隔离于主校验函数。

[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md

step-spherical-face-sense.patch: 球面/圆锥 const 法向奇偶映射到 STEP face sense；单壳或单实体空腔共享几何表，不合并复合体。
