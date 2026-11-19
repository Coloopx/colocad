# scripts/
> L2 | 父级: ../AGENTS.md

成员清单
Dockerfile.native: 固定配置/许可/构建输入；沿现有治理消费。
build-kernel.mjs: 既有源码/说明，本轮行为不变。
build-viewer.mjs: 既有源码/说明，本轮行为不变。
check-release.mjs: 既有源码/说明，本轮行为不变。
publish-npm.mjs: usr/bin/env node；Release tooling only. Credentials stay in the child environment and are never；written to the repository, npmrc, command arguments or
schema.mjs: Generate public JSON/TypeScript contracts from the development native binary.
vendor.mjs: Verify the pinned archive and extract it atomically. Apply and verify the single reviewed downstream patch.

变更：2026-11-19 Dockerfile.native 同步球面补丁 fuzz=0 前后摘要。

[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
