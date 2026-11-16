# Colocad — source navigation
Rust + TypeScript + Node.js + SHA-pinned SolidCore.
crates/agent-cad/src/: bounded native modeling, STEP exchange, validation and receipts.
crates/agent-cad/tests/: native public API and geometry regressions.
kernel/: public SDK, tracked Windows-generated distribution and rebuilt-native boundary; see kernel/AGENTS.md.
test/: public SDK/CLI and independent validation.
docs/: API and verification boundaries.
third_party/: pinned source archive and reviewed patches; generated vendor files are not committed.
out/: ignored local reproductions, plans and validation evidence.
[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md

宿主输入、STEP输出和报告分别设定有限预算；Rust resources与SDK resource-limits维持同一默认/硬上限。模型与producer不能授权，session及文件读取使用当前host快照。原生发布前分别检查实际stdout与report文件，旧超大checkpoint只冷建；几何/容差/原件SHA检查保持独立。

SDK接口变更同步Windows生成的跟踪dist；原生程序须匹配源码并显式构建。此源补丁不修改平台发行摘要或发布npm。实际复现、编译与运行配额及已知基线缺口见docs/resource-budgets.md。
