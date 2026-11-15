# G2 — 平面 conic 连续参数域

Rust + 固定 SolidCore + SHA 校验下游补丁。
基线 c3200d59；分支 fix/step-planar-conic-extrema。

third_party/: 上游固定补丁、before/after SHA、版权记录；见局部地图。
crates/agent-cad/tests/: 自有Circle/Ellipse连续域与严格负例。
kernel/licenses/: 原版权许可与G2修改声明，见局部地图。
test/validation/: 独立OCP完整闭合实体控制；只在Win执行，见局部地图。
scripts/: vendor提取与Docker相同补丁集合；本机不执行Node或构建。
out/: 私有源探针、源码snapshot、Windows统一队列交接；忽略提交。

仅G2 planar conic；不含G1/BSpline。G3-B只交叉验证，不预先声明覆盖。
本机只源码/文档/静态分析。Win实际回执绑定snapshot后才允许本地Git提交；不push/PR/Issue。
[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
