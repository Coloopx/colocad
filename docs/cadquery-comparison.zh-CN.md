# Colocad 与 CadQuery：覆盖和设计对比

核对日期：**2026-11-10**；Colocad **0.1.0-alpha.8**。[English / 完整 API 表](cadquery-comparison.md) · [逐项数据](cadquery-coverage.json) · [路线图](../ROADMAP.zh-CN.md)。

**Colocad 已有可用的机械建模、精确查询与几何驱动装配层，但还不能完整替代 CadQuery。** 我们的优势方向是显式参数图、可持久化工具闭环、轻量部署及特定负载性能；CadQuery 的通用几何 API、草图、交换格式和生态覆盖仍更广。两者不是同一套 API，不能直接执行对方的建模程序。

## 统计口径

按用户指定的 [CadQuery API Reference](https://cadquery.readthedocs.io/en/latest/apireference.html) 建立 **150 个不同入口**的清单，固定到官方源码提交 [b3f9afd](https://github.com/CadQuery/cadquery/blob/b3f9afd4f39cbd856d0c46aa30aa37b714b8f6cb/doc/apireference.rst)。JSON 中保存来源摘要、每个入口的映射、实现路径、代表测试和边界。`latest` 文档可能继续变化，也不等于某个已安装版本。

| 分类 | 数量 | 解释 |
| --- | ---: | --- |
| 有对应原生能力 | 36 | 受支持范围内有类似操作；不代表参数、所有选项或行为完全相同 |
| 部分支持 | 30 | 有相关能力，但存在实质选项、范围或语义缺口 |
| 可以组合表达 | 29 | 由调用方计算坐标或组合基础操作；没有通用等价 API |
| 尚无公开能力 | 29 | 当前公共契约未提供相应功能 |
| 使用不同抽象 | 26 | 用显式图、会话或 JSON 替代状态链；不能直接互换 |

这些是**源码/契约核对结果，不是 150 项配对测试的通过率**，不能加总成“覆盖率”。一个便利方法与复杂几何算法权重也不相同。该索引之外还有 Shape、底层拓扑、free functions 和插件生态；我们另外列出这些边界。

## 工程能力对照

| 能力 | Colocad 当前实现 | 相对 CadQuery 的边界 |
| --- | --- | --- |
| 参数和工作平面 | 命名参数表达式、显式正交 frame、特征依赖 | 不兼容 Workplane 的栈、父链和隐式上下文 |
| 基础截面 | 矩形、圆、椭圆、多边形、线/圆弧闭合轮廓 | 一般草图拓扑编辑、offset/hull/二维圆角等未公开 |
| 草图约束 | 点/线/圆约束、局部求解、残差与自由度 | 有变量/方程预算；不声称全局唯一或所有约束对齐 |
| 拉伸、旋转、布尔 | 对应原生操作和确定性特征 ID | 所有选项、鲁棒性和内核结果不等同于 OCCT |
| 孔和阵列 | 普通孔、面孔、规则孔阵列、沉孔选项 | 沉头可组合圆锥切除；没有完整沉头 API/识别保证 |
| 放样、扫掠、薄壁、圆角 | 已有受限原生操作 | 通用多导轨、扭转、任意曲面和复杂失败率仍需扩展 |
| 曲面 | Bezier、受限网络、连接、编辑、缝合、加厚 | 不等于通用 Shape/free-function 曲面能力或 Class-A 认证 |
| 通用选择 | 面/边/点/环、组合、邻接、排序、严格数量检查 | 无 wire/solid/shell/compound 等价通用选择；无最近点/box selector；半径可过滤但不能原生 nth 排序 |
| 选择驱动特征 | 建模侧支持一组命名/几何选择器 | 查询表达式比建模选择器丰富，不能直接给任意 fillet/shell 使用 |
| 基准与装配 | 重算基准、命名附件、配合、接口规则 | 求解器语义有差异，不能照名称映射 CadQuery Axis/Plane |
| 孔组与接口识别 | 解析圆柱孔、按平面分组、平面安装接口 | 属于特定工程层，复杂孔可能 partial；不是任意 STEP 自动装配 |
| 引用和改参 | 同一快照重开保持引用；改参重新解析语义规则 | 不是永远不变的 face ID；拓扑歧义必须显式处理 |
| 外部 STEP | 来源登记、刚体层级导入、查询和兼容替换 | 混合单位、非刚体映射和部分表示受限；不是任意 CAD 文件兼容 |
| 导出 | 精确 STEP、STL、GLB、PNG、参数源、会话 HTML | DXF/SVG/XCAF/XBF/3MF 等矩阵未对齐；STEP 不携带原生参数树 |
| Agent 和人类审阅 | SDK + CLI + tools + skill；CLI 启动持久化查看与导出 | 工具闭环是产品设计差异，不能据此推断几何全面领先 |

CadQuery 同样强调相对定位和设计意图，并提供无隐式状态的 free-function API，不能把它简化成“只有 Python 链式调用”。官方依据：[设计原则](https://cadquery.readthedocs.io/en/latest/designprinciples.html)、[装配约束](https://cadquery.readthedocs.io/en/latest/assy.html#constraints)、[free functions](https://cadquery.readthedocs.io/en/latest/free-func.html)、[导入导出](https://cadquery.readthedocs.io/en/latest/importexport.html)。更细的约束语义分析见[早期评审](../specs/reviews/0004-cadquery-2026-11-09.md)。

## 架构和部署

Colocad 的运行路径是 agent/harness → TS SDK 或 CLI → 本地 Rust executor → 固定版本 SolidCore。它们在同一个沙箱内运行。npm 包携带 macOS arm64、Linux arm64/x64 可执行文件，建模安装不需要 Python/Rust 工具链；独立 OCCT 回读属于验证环境。SolidCore 与 USD BRep 的上游关联不意味着 Colocad 使用 NVIDIA 原库、实现完全相同或有相同验证水平。

CadQuery 使用 Python/OCCT 生态；Colocad 使用自己的 JSON/TypeScript 契约。Colocad 的参数源、session、快照和展示文件分别承担可编辑性、审计、精确观察和人类审阅职责。原生商业 CAD 特征树是另外的能力，[通用可编辑 SOLIDWORKS 当前仍有缺口](native-solidworks.md)。

## 证据与结论边界

[G1–G3](../specs/experiments/0005/README.md) 验收的是已声明范围、冻结的 24 个案例和两条真实 Pi 任务；不是所有 CadQuery API，也不是陌生图纸重建成功率。alpha.7 后续发现的孔阵列查询缺陷保留了失败证据；[alpha.8 回归记录](evidence/release-alpha.8.json)专门验证修复、六个面方向、全局孔阵列、改参/旧引用/重开和独立几何回读。

本报告**没有新增 CadQuery 速度或内存实验**。历史 build123d 对照只适用于当时相同几何、精度、硬件与计时范围，不可外推成全面领先。后续按[路线图](../ROADMAP.zh-CN.md)分别做确定性执行、编辑装配、真实 agent 读图三条 benchmark，并先检查质量，再比较速度、内存和推理成本。
