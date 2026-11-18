# /docs
> L2 | 父级: ../AGENTS.md

README.md: Developer documentation
README.zh-CN.md: 开发与接入指南
agent-modeling-handbook.md: Agent CAD handbook
agent-modeling-handbook.zh-CN.md: Agent CAD 完整工作指南：资料、工具、建模闭环与调优
agent-modeling-research.md: Research basis and evidence boundaries
api.md: SDK/CLI、宿主字节/计数、cache单定义边界与0.1.1分代验证记录。
benchmark-alpha10.md: Colocad alpha.10 performance and geometry
benchmark-alpha10.zh-CN.md: Colocad alpha.10 性能与几何复测
benchmark-alpha11-rc1.md: Engineering edits, memory growth and time to STEP
benchmark-alpha11-rc1.zh-CN.md: 连续改图、装配换件和 STEP 交付
benchmark-alpha9.md: Alpha.9 benchmark: choose by workload
benchmark-alpha9.zh-CN.md: alpha.9：什么时候该选 Colocad
benchmark-stable.md and benchmark-stable.zh-CN.md: public npm release performance and verified revision history.
benchmark-appearance.md: Five product models on macOS arm64
benchmark-appearance.zh-CN.md: 五款产品外观的 macOS arm64 建模实测
benchmark-bounded-candidate.md: Three-backend measurements and assembly validation costs
benchmark-bounded-candidate.zh-CN.md: 三后端复测与 64 件装配耗时分析
best-practices.md: Modeling and integration best practices
best-practices.zh-CN.md: 建模、查询与装配最佳实践
bounded-builds.md: Bounded writes, assembly working sets and Level-1 validation
cadquery-comparison.md: Colocad and CadQuery: coverage and design
cadquery-comparison.zh-CN.md: Colocad 与 CadQuery：覆盖和设计对比
concepts.md: Models, revisions and artifacts
engineering-queries.md: Exact queries and geometry-backed assembly
engineering-queries.zh-CN.md: 精确查询与几何基准装配
field-history.md: What still grows after persisting the index?
field-history.zh-CN.md: 索引持久化以后，哪些内存还在涨？
getting-started.md: Getting started
hybrid.md: Sculpted parts and precision mechanisms
hybrid.zh-CN.md: 自由曲面与网格混合装配
indexed-sessions.md: Indexed sessions
indexed-sessions.zh-CN.md: 会话索引与紧凑回执
memory-persistence-report.md: CAD memory limits and persistence study
memory-persistence-report.zh-CN.md: CAD 内存增长、沙箱极限与持久化优化报告
modeling-history.md: Modeling history: CLI first
modeling-workflow.zh-CN.md: 实际建模操作、数据流与完整产品的验收
native-solidworks.md: Native SOLIDWORKS author (experimental)
performance.md: Performance evidence and reproduction
revision-cache.md: Saving 256 geometry revisions under a 512 MiB limit
revision-cache.zh-CN.md: 连续保存 256 个几何版本的内存
troubleshooting.md: Troubleshooting
validation.md: Release validation
resource-budgets.md: SDK/native一致的显式宿主字节预算、默认/硬顶和实际验收边界。

benchmarks/: 可复现性能输入与运行脚本。
evidence/: 固定版本测试与性能回执。
integrations/: Agent宿主及建模接入说明。

step-import-contexts.md: 同长度/metadata上下文证明、release预算/cache合并边界及分代Windows验收。

[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
本轮resource-budgets.md仅扩展PR6输出合同，静态回移不继承组合版本验收。

resource-budgets.md: input/STEP默认保持，新增reportBytes默认8MiB/顶128MiB/最小1024B及精确stdout/file计量；PR6静态容量回移不继承组合版本接受。

resource-budgets.md公共正文只保留资源合同、primary exporter链接和独立验收规则；内部批次/产品/控制器记录在忽略交接目录，实际buildprofile与counts待独立完成后填写。
2026-11-15 api.md增加宿主STEP计数策略：默认/硬顶、SDK/native/Rust入口、结构化拒绝、checkpoint身份与不支持的下游容量。
revision-memory-attribution.zh-CN.md: 0.1.1 revision-memory attribution, identical-byte layout experiments and proposed release gates.

[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
execution-progress.md: 宿主停滞/显式墙钟/取消合同、原生事件与固定实例碰撞早检的边界。
