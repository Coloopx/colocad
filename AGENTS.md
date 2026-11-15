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

Native parse accounting/reuse is invocation-local and bounded to one recently admitted part/definition source keyed by mode and verified SHA; selected bodies/BReps are never cached here. All compilation and tests run on Windows/WSL2; local work is static only.
