# Colocad — UV度量修复工作树
Rust + TypeScript + Node.js + SHA固定SolidCore。
crates/agent-cad/src/: 原生几何、终态校验及能力回执；不在JS复制几何逻辑。
crates/agent-cad/tests/: 原生几何与公开build入口回归。
docs/: 开发指南与验证边界。
test/: 公开SDK、CLI和独立OCCT消费验证。
third_party/: 固定依赖及已核验补丁；本轮不变更依赖或容差。
out/: 本轮本地计划/复现与验证，不提交生成资产。
范围：只修UV参数空间与曲面毫米度量的混用，不包含1%间隙策略。
[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
