# Colocad — UV 曲面度量修订分支
Rust + TypeScript + Node.js + SHA 固定 SolidCore。

crates/agent-cad/src/: 公共 UV 曲面度量、宿主 ByteBudgets 和调用内 ImportCache，保持几何容差。
crates/agent-cad/tests/: 原生功能、负例和保守曲线证明回归，见局部地图。
kernel/: 公共 SDK、跟踪 dist 和原生构建边界，见 kernel/AGENTS.md。
src/: 公共 session 及宿主预算。
test/: 公共 SDK/CLI、构建幂等性及独立消费验证。
docs/: 接口、资源合同和版本化验收边界。
third_party/: 固定归档、顺序补丁及前后摘要；vendor 产物不提交。
scripts/: 原生构建和依赖准备入口，同目标多补丁必须可重复验证。
out/: 忽略的 Windows 源快照、原始失败、资源回执及本地回复草稿。

## 活跃约束
基于上游 0.1.1 的 4124bcb；保留已合入的预算、源复用及平面圆锥补丁。
公共执行启用 uv_trim_checks_on_surface；映射曲线使用自身的毫米尺度保守界，证明失败仍走原求解器。
宿主预算、曲线/环/拓扑/流形拒绝和原容差保持；本修订不消费未合入的单位上下文或定义提取 PR。
Node/Rust/几何验收只在 Windows/WSL 执行。编译和运行限额分别记录；静态核对不代替执行。
性能对照单独冻结四个原 PR 的组合基线；实验开关仅存在于忽略的测试副本，不进入生产源码。

## 变更日志
2026-11-17：响应 PR3 性能反馈，追加毫米映射证明、负例及同目标补丁幂等回归，并对齐上游 0.1.1。

[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
