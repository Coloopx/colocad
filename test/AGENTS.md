# /test
> L2 | 父级: ../AGENTS.md

hybrid.test.mjs: real loft wall, cubic tool, shell opening, closed offset, articulated clearances and a penetrating control；STL and multicolour GLB round trip, mesh CSG, exact import and faceted B-rep conversion 等公开行为回归。
kk60.test.mjs: K1/K2: catalog actuator is a complete product with named native parts；K4: stroke endpoints remain editable and within the catalog guide envelope 等公开行为回归。
lifecycle.test.mjs: semantic named-face lifecycle: split, merge, deletion and old-ref rejection；foreign assembly replacement and rejected replacement retain accepted session across relocation 等公开行为回归。
mates-direction.test.mjs: SDK solves opposed shaft/support mates, retains rotation and rejects overlap 等公开行为回归。
publish-config.test.mjs: release preflight isolates npm config and does not persist the credential；wrong identity or a different immutable archive prevents publishing and tag updates 等公开行为回归。
query-grid.test.mjs: bundled face grid survives SDK query, dimension/count edits, relocation and CLI replay 等公开行为回归。
query.test.mjs: independent OCCT STEP preserves analytic top face and five inner-circle centers；SDK reopens exact snapshot, exposes schema/tool, preserves refs and confines host paths 等公开行为回归。
record-cli.test.mjs: Codex/Claude Code宿主技能布局、命令记录及离线历史导出。
render-options.test.mjs: R1/R2: deterministic film timing is separate from recorded build times；R3: invalid movie options fail before rendering 等公开行为回归。
render.browser.mjs: 浏览器渲染和录制呈现验收。
session-catalog.test.mjs: CLI migration preserves ledger, source, exact query refs and pagination across relocation；a malformed legacy tail cannot publish a partial catalog and a corrected retry succeeds 等公开行为回归。
session-query-cli.test.mjs: CLI records exact inspections, edits the accepted part and rebinds after relocation 等公开行为回归。
session.test.mjs: S1: immutable native history survives reopening and relocation；S2: failed attempt persists and edits start from last successful source 等公开行为回归。
tool-summary.test.mjs: compact tools keep executable schemas, exact measurements and recovery state；response compaction never erases degradation, errors or capability limits 等公开行为回归。
tree.test.mjs: one feature compiler accepts two distinct histories and preserves the nonadjacent cut seed；invalid operations, stale references and resource limits fail before geometry 等公开行为回归。
turbofan.test.mjs: E1/E2: real geometry milestones lead to a shared-definition turbofan assembly；E3: editable tip radius changes exact engine bounds 等公开行为回归。
view-cli.test.mjs: CLI launches local viewer and exports the same offline history；CLI refuses an invalid viewer port 等公开行为回归。
viewer.browser.mjs: 真实原生网格查看器交互与离线约束。

validation/: 原生与打包入口的独立几何/集成审计。
evidence/: 已保存的历史失败文本。
types/: 公开查询和宿主资源配置的编译合同与负例。

[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
resource-limits.test.mjs: 公开SDK注册/UTF-8/JSON/聚合字节、宿主预算及原生摘要/回读守卫回归。
