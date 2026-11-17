# scripts/
> L2 | 父级: ../AGENTS.md

成员清单
Dockerfile.native: 固定配置/许可/构建输入；沿现有治理消费。
build-kernel.mjs: 既有源码/说明，本轮行为不变。
build-viewer.mjs: 既有源码/说明，本轮行为不变。
check-release.mjs: 既有源码/说明，本轮行为不变。
publish-npm.mjs: usr/bin/env node；Release tooling only. Credentials stay in the child environment and are never；written to the repository, npmrc, command arguments or
schema.mjs: Generate public JSON/TypeScript contracts from the development native binary.
vendor.mjs: 验证归档、补丁前后文件及整树摘要；同目标有序补丁逐文件校验最终锁定SHA，原子准备及幂等重复消费。

变更：2026-11-15 G2补丁与Windows定向回归；不修改其它算法。

[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
