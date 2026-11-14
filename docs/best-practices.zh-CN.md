# 建模、查询与装配最佳实践

内存紧张时，相同零件共享定义；遇到大量独立毛坯或密集孔阵列，先用一个有代表性的完整 STEP 和实际检查点设定测资源，再放大规模。[三方复测](benchmark-alpha9.zh-CN.md)同时列出领先和落后的工作负载。

示例使用 Colocad 0.1.0。[English](best-practices.md) · [完整建模手册](agent-modeling-handbook.zh-CN.md) · [查询契约与边界](engineering-queries.zh-CN.md)。

## 从资料与控制结构开始

先锁定产品型号、配置、单位、整机基准和交付要求。图纸/PDF、照片、BOM、参考 STEP 是证据；agent 用 harness 的阅读工具解释它们，再生成明确的参数和特征指令。给定、推导、假设和未知尺寸分别记录，关键接口未明确时保留未验证状态。

板件从毛坯和孔阵列开始，套筒从内外径与长度开始，壳体从壁厚、底厚和开口关系开始。复用[快速结构资产](../skills/colocad/references/quick-build.md)，但按实际产品修改。先完成主体与功能孔腔，再增加次要圆角；重复零件共享定义，尺寸关系写成参数表达式。

## 按当前问题选择工具

| 当前问题 | 建议操作 |
| --- | --- |
| 只换镜头或图片尺寸 | 同一 client 的 `preview` 或保存网格的 `observeArtifact` |
| 改尺寸 | 从接受的源 `edit`，再检查受影响接口 |
| 已有零件的面、孔、轴或安装接口是什么 | 对接受版本的精确快照执行 `query` |
| 替换一份兼容 STEP | 重算语义基准/接口，求解配合，再跑必需检查 |
| 新建精确几何 | `build` 或 CLI `record` 保存新的里程碑 |
| 人类审阅/影片 | CLI `view`、`replay`、`render`，使用真实记录的结果 |

常规建模可先注册 capabilities/validate/build/preview/edit；测量和装配工作加入 capability-gated `cad_query`，按需启用 STEP 检查和草图求解。SDK 的完整 schema 与 `$defs` 保留。Agent、harness、SDK、native 同处一个沙箱。

## 查询 → 设计意图 → 装配 → 改参复验

1. **构建可查询版本。** 普通模型按需设置 `{queryable:true}`；声明 `Model.datums` 或 `Model.interfaces` 时自动启用。保存回执中的 `artifact_directory` 与 `query_revision_id`。定义局部坐标用 `definition_id`；装配实例用 `instance_path`，并明确 `space` 和容差。
2. **按安装意图缩小范围。** 先选安装平面，再找该面的开口和孔组。同法向可能同时选中盲孔底面；位置有关时用命名平面与 `planar_coplanar_with`。检查数量、额外孔、竞争候选和 coverage，不自动选第一项。
3. **持久化选择规则。** 将受支持的选择/推导写入 `Model.datums`、`Model.interfaces`。快照 ref 是当次观察，不是跨改参的永久标识；不能塞进持久规则或绑定偶然的面编号。对称孔组需要设计给定或几何推导的方向，才能用于定向配合。
4. **检查求解后的总成。** 使用命名附件、完整孔对应的 `interface_compatibility`，以及相关零件对的必需 `geometric_relationship`。正的边界距离不一定代表净空：包含关系也可能有正距离；同时看关系类型、重叠量和间隙阈值。
5. **改参/替换后重查。** 从接受源重建，取得新版本并重跑规则和检查。缺失、歧义、不兼容作为失败留档，不靠更换面编号强行通过。用兼容与不兼容来源挑战同一装配规则。
6. **交付可复核证据。** 保存参数源、完整 session、STEP、数值结果和未覆盖项；由 CLI 导出审阅。离散姿态检查不能证明连续全行程，爆炸动画也不能证明运动无碰撞。

完整的外部 STEP 工作流见[安装接口例子](../examples/engineering-interfaces/README.md)：三种兼容替换、两种正确拒绝，使用同一规则，并有独立 OCCT 验证。

## 可以直接执行的改参示例

在新项目安装 `@coloopx/colocad@0.1.0`，将[英文页的完整脚本](best-practices.md#run-the-editquery-loop-against-the-installed-package)保存为 `query-workflow.mjs`，执行：

```sh
node query-workflow.mjs
./node_modules/.bin/colocad replay ./query-session ./query-review.html
./node_modules/.bin/colocad view ./query-session
```

脚本用 `box` 和 `face_hole_grid` 构造 80×50×6 mm 四孔板及精确快照，查询四个 Ø6 通孔，将孔径改为 Ø8，确认旧版本请求被拒绝，再检查新孔径、孔中心保持和解析体积。预期的旧引用错误保留在会话中。首次运行时 `query-session` 和 `query-review.html` 应尚不存在。

**版本说明：**alpha.8 已修复孔阵列精确快照，示例直接使用 `face_hole_grid`。升级 SDK 和配套 native；调预览精度不能修复 alpha.7 的问题。[原始失败](evidence/alpha.7-query-guidance.json) · [修复回归](evidence/release-alpha.8.json)。

它验证的是尺寸件的实际 SDK/CLI 闭环，不是未知图纸重建成功率。

## 返回足够决策的信息，保留完整证据

简短工具反馈保留成功/失败、feature/query ID、`query_revision_id`、目标/坐标系、相关测量、coverage/歧义、时间、全部降级与完整回执路径。不要把成功调用的 partial 结果解释成“完整识别”或“没有孔/碰撞”；完整 STEP、网格和 trace 留在文件系统中。

失败先分类：资料解释、frame/单位、选择歧义、几何操作、引用过期或预算。针对一个原因修复，再复查相关要求。查询版本过期应从最新接受版本重查；不要仅替换旧 ref 上的版本字符串。具体错误见[故障排查](troubleshooting.md)。

尺寸、材料/孔腔、装配和可编辑性分别验收；重要交付用独立内核回读 STEP。渲染、总体积或零退出码不能独立证明全部正确。锥形/相交/侧开/样条孔识别可能为 partial；曲面接触认证、局部区域限定测量仍有明确支持边界。

## 调优时保持测量口径

分开记录建模、快照创建/加载、查询、STEP、三角化、渲染与推理。只为读现有几何时避免重建；先缩小查询范围和结果页，再考虑提高预算。

内存看 cgroup 峰值、OOM、子进程与缓存页，RSS 作为补充。[alpha.7 安装包验收](evidence/release-alpha.7.json)包含 npm 安装，[G3 实验](../specs/experiments/0005/README.md)测的是另外的 CAD/驻留 agent 负载，不能混成同一指标。历史灵巧手对照也不能外推为全部复杂模型的性能保证。G4 benchmark/回溯优化尚未启动。
