# Colocad — source navigation
Rust + TypeScript + Node.js + SHA-pinned SolidCore.
crates/agent-cad/src/: bounded native modeling, STEP exchange, validation and receipts.
crates/agent-cad/tests/: native public API and geometry regressions.
kernel/src/: public SDK, host configuration and isolated native process boundary.
test/: public SDK/CLI and independent validation.
docs/: API and verification boundaries.
third_party/: pinned source archive and reviewed patches; generated vendor files are not committed.
out/: ignored local reproductions, plans and validation evidence.
[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md

PR6输出预算回移：仅增加独立outputStepBytes（默认32MiB/硬顶512MiB）、原生序列化/回放与固定SHA产物reader和session/HTML输出读取；不合入其它PR。当前仅静态准备，必须Windows重生成SDK/native并验证。
