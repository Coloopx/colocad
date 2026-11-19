# Colocad — source navigation
Rust + TypeScript + Node.js + SHA-pinned SolidCore.
crates/agent-cad/src/: bounded native modeling, STEP exchange, validation and receipts.
crates/agent-cad/tests/: native public API and geometry regressions.
kernel/: public SDK, tracked TypeScript distribution and rebuilt-native boundary; see kernel/AGENTS.md.
kernel/src/: public SDK, host configuration and isolated native process boundary.
test/: public SDK/CLI and independent validation.
docs/: API and verification boundaries.
third_party/: pinned source archive and reviewed patches; generated vendor files are not committed.
out/: ignored local reproductions, plans and validation evidence.
[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md

宿主输入、STEP输出和报告分别设定有限预算；Rust resources与SDK resource-limits维持同一默认/硬上限。模型与producer不能授权，session及文件读取使用当前host快照。原生发布前分别检查实际stdout与report文件，旧超大checkpoint只冷建；几何/容差/原件SHA检查保持独立。

Rebuild the TypeScript distribution when changing the SDK. Test native changes with a binary built from the same sources. See docs/resource-budgets.md for the resource experiments.

scripts/: local and Docker builds apply the pinned vendor patches.
See third_party/patches/README.md for the pinned planar conic domain patch and crates/agent-cad/tests/step_planar_conic_domain.rs for its regression cases.

Adjacent STEP definition imports reuse one admitted source in the native build. See crates/agent-cad/src/import_cache.rs and crates/agent-cad/tests/step_import_cache.rs.

UV 曲面度量在公共执行中启用；identity 度量不分配映射克隆，mapped regularity 隔离于主校验函数。顺序 vendor 补丁均验证最终摘要，曲线保守证明失败时沿用原求解器。参见 docs/validation.md。
