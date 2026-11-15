# Colocad — PR3/6/12/14 本地基准集成树
Rust + TypeScript + Node.js + SHA固定SolidCore。

crates/agent-cad/src/: 公共UV曲面度量、宿主ByteBudgets及调用内ImportCache；保持Level1与几何容差。
crates/agent-cad/tests/: 四个公开PR的原生回归及输入夹具。
kernel/: 公共SDK、跟踪dist与原生构建边界，见kernel/AGENTS.md。
src/: 公共session及当前宿主字节授权。
test/: 公共SDK/CLI和独立消费验证。
docs/: 原接口、资源合同和各源分支的历史验证边界。
third_party/: SHA固定源码归档、补丁及顺序锁；vendor产物不提交。
scripts/: 原生构建入口；本轮不执行。
out/: 忽略的本地集成来源、冲突与哈希manifest。

## 实验约束
以c3200d59为唯一base，依次合入PR3@1b3f1bc、PR6@7693333、PR12@632d061及PR14@7a4a0d3原提交；不含PR8/10/G3或本轮UV性能修复。
公共executor保持uv_trim_checks_on_surface=true；ImportCache与输入/STEP输出/report字节预算同时保留。模型不能抬高宿主预算。
本轮只进行Git、文本与哈希核对，不构建、不运行Node/Rust/几何、不推送；源分支历史验收不视为组合版本通过。Windows基准另行执行。

## 变更日志
2026-11-17：创建独立本地四PR集成基线；保留各源提交及逐次merge来源，冲突合并功能与局部地图。

[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
