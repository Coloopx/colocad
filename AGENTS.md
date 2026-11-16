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

PR6输出预算回移：仅增加独立outputStepBytes（默认32MiB/硬顶512MiB）、原生序列化/回放与固定SHA产物reader和session/HTML输出读取；不合入其它PR。独立Windows验收已取得默认release opt3的compile-only 2CPU/5GiB/no-swap成功（21m32s），2CPU/3GiB执行28个Rust test exe共147/147；SDK full 87/88，唯一SESSION_BUSY为既有baseline；types/schema仍在运行。

PR6最终容量回移：outputStepBytes默认32MiB/顶512MiB；reportBytes默认8MiB/顶128MiB/最小1024B。native发布前分别精确计量compact回执+newline及pretty报告；SDK/reader/session统一当前host预算。旧checkpoint>2MiB只冷建。保留两次cold 3GiB编译OOM（opt3、opt1/64），不宣称默认cold 3GiB通过。提交/PR同步由主线程按既有授权执行；本轮不更新platform binary/release.json、不发布npm。实际recipe与验收边界见docs/resource-budgets.md。
