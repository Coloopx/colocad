# Colocad

![HIWIN KK60 线性执行器：真实建模与装配过程的四视图回放](showcase/hiwin-kk60/modeling-alpha11-rc1.gif)

**面向 agent 的参数化 CAD：本地 Rust 几何内核、TypeScript SDK、CLI，以及可追溯的建模过程。**

[English](README.md) · [文档](docs/README.zh-CN.md) · [npm](https://www.npmjs.com/package/@coloopx/colocad) · [更新日志](CHANGELOG.md) · [Releases](https://github.com/Coloopx/colocad/releases) · [封面模型](examples/hiwin-kk60/README.md)

Colocad 把明确的建模指令转成可编辑源、精确 STEP 和可视化产物。Agent、harness 与 CAD 程序都在**同一个沙箱内部**。npm 包自带编译好的 Rust 内核，安装后建模无需 Python、Rust 工具链、网络或独立 CAD 服务。

**最新正式版为 [0.1.0](https://github.com/Coloopx/colocad/releases/tag/v0.1.0)。** 五款产品实测中，Colocad 的新进程占用更少内存，CadQuery 的已加载进程完整重建更快。Linux 沙箱里，Colocad 连续保存 256 次 KK60 改图的内存峰值为 **92.5 MiB**。[五款产品对照](#五款产品的外观建模) · [逐版内存曲线](#连续保存-256-版的内存)。

## 可以用来做什么

- **零件和装配体：** 参数化特征、重复零件实例、精确 STEP，以及 PNG/GLB/STL 预览。
- **Agent 建模：** SDK 工具与随包 skill，支持尺寸解释、特征规划、改参和结果检查。
- **人类审阅：** 保存成功与失败的构建，检查源和回执，输出单文件离线 HTML、四视图 MP4/GIF。

SDK 是基础，CLI 和工具适配器负责接入 harness，skill 指导建模决策。图纸与图片由 agent 作为证据解释；colocad 执行明确的几何指令。

```mermaid
flowchart LR
  A[Agent + skill] --> H[现有 harness]
  H --> C[CLI / SDK 工具]
  C --> K[本地 Rust 内核]
  K --> F[STEP + 源 + 回执 + 网格]
  F --> V[CLI 查看器 / HTML / 动画]
```

## 安装

需要 **Node.js 22+**，支持 **Linux x64、Linux arm64、macOS arm64**。

```sh
npm install @coloopx/colocad
```

npm 的 `latest` 指向 `0.1.0`。如需锁定版本，使用 `npm install --save-exact @coloopx/colocad@0.1.0`。发布验收从公共注册表安装，再用 CLI 导出歧管，OCCT 回读得到一个 90 × 45 × 12 mm 的有效实体。 [安装与回读记录](https://github.com/Coloopx/colocad/releases/download/v0.1.0/registry-proof.json)。

安装包包含 JS/TS API、CLI、原生可执行文件、查看器和完整 skill。Rust 源码位于 [crates/agent-cad](crates/agent-cad)，npm 包携带编译产物。[模型与产物格式](docs/concepts.md)。

## 快速开始

在你的 Node 项目中运行：

```sh
# 构建随包歧管示例，保存源、回执和几何。
npx colocad record node_modules/@coloopx/colocad/examples/manifold.json ./history \
  --label "Build the manifold"

# 启动交互式查看器。
npx colocad view ./history

# 导出单文件 HTML，其中包含最终 STEP 下载。
npx colocad replay ./history ./review.html
```

示例包含拉伸、盲孔切除、圆角、线性阵列和口袋。修改 JSON 后再次 `record` 到同一个 history 目录，即可追加版本；失败尝试也会保留。`view` 展示启动时的快照，新增版本后需重新打开。[首个零件与改参教程](docs/getting-started.md)。

也可以通过 SDK 使用相同的本地内核：

```js
import { ModelingSession } from '@coloopx/colocad/session';

const session = await ModelingSession.create('./sdk-history');
const result = await session.build({
  version: 1, units: 'mm', parameters: { length: 60 },
  features: [{ id: 'stock', op: 'box', origin: [0, 0, 0],
    size: [{ param: 'length' }, 30, 8] }],
  output: 'stock',
});
if (!result.ok) throw new Error(JSON.stringify(result.error));
await session.edit({ length: 80 });
```

无需记录会话时，直接使用 `CadClient`。[SDK、CLI 与工具 API](docs/api.md)。

## 工程更改单

给三个后端相同的 STEP，要求改长、加厚 KK60 相机板，新增走线窗口，再用实际孔位适配坐标平移的替换件。另一组任务要求从旋转过的电机法兰重新求安装方向，遇到孔距不符或缺孔必须拒绝。

![十条工程更改单的输入和三个后端实际输出的 STEP 回读](docs/benchmarks/alpha11-rc1/engineering-changes.gif)

**Colocad、CadQuery 和 build123d 都交付了七个有效装配，并正确拒绝三个不兼容输入。** Colocad 的 SDK/CLI 自带改参、版本历史和快照查询；实测六个旧版本的孔距时，Colocad 直接查询原生快照，几何重建次数为 0；两个 Python 适配器重开六份 STEP 完成了相同测量。Colocad 的高层盲孔识别在重导入滑台上仍返回部分覆盖，实测用精确圆边找到了安装位置。[任务、几何验收和限制](docs/benchmark-alpha11-rc1.zh-CN.md)。

## 连续保存 256 版的内存

每次都修改真实几何并持久化可编辑源、精确 STEP 和网格预览。四个会话共 1,024 份改图装配全部独立回读通过。每场 1 vCPU、512 MiB、无交换。

![按已完成改图数对齐的沙箱当前内存和累计峰值](docs/benchmarks/alpha11-rc1/revision-memory.gif)

| 工作流 | 峰值 MiB | 第 256 版当前 MiB | 后 128 版净增 |
|---|---:|---:|---:|
| Colocad 0.1.0-alpha.11-rc.1 | **92.5** | **73.0** | 6.8 MiB |
| CadQuery 2.8.0 + 持久化适配器 | 223.0 | 218.8 | **0.9 MiB** |
| build123d 0.13.0 + 持久化适配器 | 271.4 | 267.1 | 6.9 MiB |
| Colocad 0.1.0-alpha.10 | 512.0 | 503.2 | 已触及限额 |

候选版的绝对内存峰值最低，CadQuery 的后半段增量最小。两个 Python 适配器均采用 SQLite、文件同步及干净页释放，Colocad 还保留原生检查点和查询快照。[实测方法和逐版证据](docs/benchmark-alpha11-rc1.zh-CN.md)。

## 有机外壳与外部网格

从轮廓标志点做光顺放样与法向偏置，将外部扫描网格和精确 STEP 机构放入同一装配。GLB 和离线查看器显示实例材质，干涉收据给出实际零件的交叠体积与算法。[使用说明、导出格式与验收](docs/hybrid.zh-CN.md)

```sh
npx colocad build node_modules/@coloopx/colocad/examples/hybrid-enclosure/model.json ./housing-build
```

## 从 CLI 启动可视化与导出

![CLI 启动的建模历史查看器：KK60 装配体](showcase/hiwin-kk60/viewer-alpha11-rc1.png)

查看时间线、源、特征、回执与选中的零件；切换镜头、显示剖切/爆炸视图、下载产物。离线 HTML 内嵌渲染代码和几何。需要集成到现有界面时，可使用 `@coloopx/colocad/viewer`。

| 命令 | 输入 | 结果 |
| --- | --- | --- |
| `colocad plan TREE.json` | `colocad/1` 特征树 | 编译并检查原生建模任务 |
| `colocad build TREE.json NEW_DIR --preview` | 特征树 | STEP、可编辑源和预览 |
| `colocad record MODEL.json SESSION` | 特征树或 kernel JSON | 构建并保存建模里程碑 |
| `colocad view SESSION` | 已保存会话 | 本地交互查看器 |
| `colocad replay SESSION review.html` | 已保存会话 | 单文件离线 HTML |
| `colocad render SESSION modeling.mp4` | 已保存会话 | 四视图动画，也支持 `.gif` |

用 `colocad head SESSION` 取当前有效的几何版本，`colocad inspect SESSION REQUEST.json` 查询并记录安装面和孔组，`colocad edit SESSION PARAMETERS.json` 从有效模型改参。四孔板示例演示了如何改成六孔，并重新测量孔径。[操作步骤](skills/colocad/references/modeling-loop.md#inspect-and-edit-from-a-shell)

无桌面的沙箱使用 `colocad view ./history --no-open --port 8787`，配合 harness 现有端口转发；监听地址为 `127.0.0.1`。

```sh
# 可选的视频依赖；还需在宿主安装 FFmpeg。
npm install -D playwright
npx playwright install chromium
npx colocad render ./history ./modeling.mp4 --width 1280 --height 800 --fps 15
```

动画读取**实际保存的 CAD 版本**，真实调用耗时与剪辑后的播放时长分别显示。构建几何、导出 HTML 无需 Chromium 或 FFmpeg。[完整历史与渲染指南](docs/modeling-history.md)。

## 接入你的 agent

复制完整 skill，包含 references、scripts 和可编辑形状资产：

```sh
# Codex 项目技能；Claude Code 改用 .claude/skills。
mkdir -p .agents/skills
cp -R node_modules/@coloopx/colocad/skills/colocad .agents/skills/colocad
```

已有同名 skill 时，更新前保留本地定制。然后给 agent 一个具体任务：

> 使用 colocad skill，构建 100 × 50 × 6 mm 安装板，中心对称布置 3 × 2 个 Ø6 通孔，两向孔距 32、30 mm。保存可编辑源与建模会话，检查尺寸，通过 CLI 导出 STEP 和离线 HTML。

| Harness | 接入方式 |
| --- | --- |
| Codex | [项目 skill 与 CLI](docs/integrations/codex.md) |
| Claude Code | [项目 skill 与本地工具](docs/integrations/claude-code.md) |
| Pi core / coding agent | [类型化工具适配与扩展](docs/integrations/pi.md) |
| 自建 harness | [SDK schema、状态和产物生命周期](docs/integrations/overview.md) |

[完整建模指南](docs/agent-modeling-handbook.zh-CN.md)涵盖材料预备、尺寸不确定性、工具抽象、提示词、建模闭环、验证和调优。[可运行工具示例](examples/integrations/README.md)执行真实建模、数值核对和失败恢复；决策序列是脚本化的，不是 LLM 自主建模成功率测试。

## 实例：完整线性执行器

封面依据厂家图纸重建 **HIWIN KK60，导轨 300 mm、导程 5 mm、行程 210 mm**，包含 **52 个最终特征、81 个命名实体实例、16 个建模里程碑**。丝杠螺旋槽是实际切出的几何，另有导轨沟槽、安装孔、滑台、轴承和联轴器。

```sh
node node_modules/@coloopx/colocad/examples/hiwin-kk60/run.mjs ./actuator
npx colocad view ./actuator/session
npx colocad render ./actuator/session ./modeling.mp4
```

独立 OCCT 检查覆盖 16 份 STEP、64 个材料探针和指定位置的导轨/滑台间隙。未公开内部结构明确标为近似或省略；这是产品目录重建，不是可直接生产的完整图纸。动画也不是速度基准。

[MP4](showcase/hiwin-kk60/modeling-alpha11-rc1.mp4) · [可编辑模型](showcase/hiwin-kk60/model.json) · [验证结果](docs/benchmarks/alpha11-rc1/kk60-readback.json) · [尺寸与假设](examples/hiwin-kk60/sources.json) · [完整离线交付包](https://github.com/Coloopx/colocad/releases/download/v0.1.0-alpha.4/hiwin-kk60-delivery.zip)

## 性能

### 五款产品的外观建模

在 macOS arm64 上，每次启动新进程构建这五款产品时，Colocad 的进程树峰值 RSS 为 **69–83 MiB**，CadQuery 为 **481–495 MiB**。宿主进程已加载时，CadQuery 在五例完整重建中都更快。以 Mori 为例，两者从新进程到 STEP 分别用时 **2.24 / 2.29 秒**，从已加载的宿主重建则是 **2.12 / 0.38 秒**。

![五款外观装配的四个视角，均来自重新打开的 Colocad STEP 文件](docs/benchmarks/appearance-0.1.0/step-views.gif)

表中每格依次为 **Colocad / CadQuery**。每款产品的两个后端执行同一份重建规格，耗时取三次中位数。模型推理、视觉评审和独立 STEP 回读均在计时外。

| 产品 | 新进程到 STEP（秒） | 新进程峰值 RSS（MiB） | 已加载后重建（秒） |
|---|---:|---:|---:|
| 蘑菇桌面机器人 | 0.92 / 2.12 | 71.1 / 484.6 | 0.85 / 0.10 |
| Flipper Zero | 0.58 / 2.03 | 75.9 / 482.0 | 0.49 / 0.11 |
| SenseCAP Watcher | 0.54 / 2.02 | 73.5 / 482.6 | 0.45 / 0.10 |
| StackChan | 0.40 / 2.00 | 69.2 / 480.7 | 0.33 / 0.07 |
| Mori | 2.24 / 2.29 | 82.7 / 495.0 | 2.12 / 0.38 |

SenseCAP 一行在两端都采用圆形底座。Colocad 对指定的近圆椭圆拉伸校验失败，改成圆形后一个方向相差 0.0228 mm。五款产品共 139 个有明确数值的分件尺寸，其中 109 个已验证落在本次 0.25 mm 对照阈值内。[实测方法、几何验收与逐次数据](docs/benchmark-appearance.zh-CN.md)。

### Linux 沙箱机械用例

**11 月 13 日的候选版复测**中，Colocad 在 **22/23** 个机械用例取得最低沙箱峰值，在 **16/23** 个用例最快交付 STEP。每个后端、每个用例新起进程跑三次，207 份 STEP 全部通过独立检查。

![重新测得的 STEP 交付时间和沙箱峰值](docs/benchmarks/alpha11-rc1/scorecard.svg)

同一零件放置 128 次，Colocad / CadQuery / build123d 耗时 **0.62 / 2.51 / 3.64 秒**。64 种独立尺寸板件分别用 **11.12 / 2.28 / 3.51 秒**。在 1,024 孔单实体上，两个 Python 后端都更快、内存也更低。[完整规模曲线](docs/benchmarks/alpha11-rc1/memory-scaling.svg)。

三个后端各跑一个 100 秒队列，分别交付 **45 / 51 / 41** 份通过验收的 43 实体灵巧手。采样内存峰值为 **167.1 / 481.0 / 280.9 MiB**，动图按实测完成时间 10 倍速播放。

![灵巧手实际交付数量和采样内存峰值](docs/benchmarks/alpha11-rc1/hand-window.gif)

[单文件离线 HTML，内含动图](docs/benchmarks/alpha11-rc1/scorecard.html) · [方法与数据](docs/benchmark-alpha11-rc1.zh-CN.md) · [复现步骤](https://github.com/Coloopx/colocad/blob/main/benchmarks/readme/candidate/README.md)。

## 支持范围与限制

- 原生包支持 Linux x64/arm64、macOS arm64，要求 Node.js 22+。[三平台发包验收](https://github.com/Coloopx/colocad/releases/download/v0.1.0/release-validation.json)。
- 支持可编辑 JSON/程序和精确 STEP；STEP 不保存商业 CAD 软件的原生特征树。
- Linux 通用可编辑 SOLIDWORKS 输出尚未完成。实验性 Windows author 需要授权 SOLIDWORKS，软件内重建验收仍未通过验证。[原生导出状态](docs/native-solidworks.md)。
- 几何有效、读图正确、可编辑、能配合和可制造需要各自的证据。[验证范围](docs/validation.md)。
- CAD 的 512 MiB 测试不代表常驻 agent、浏览器和视频编码器的整体内存预算。
- alpha.8 修复 `face_hole_grid` 与 `hole_grid` 接缝曲线的精确查询快照。[回归与安装包证据](docs/evidence/release-alpha.8.json)。

## 能力覆盖与路线图

[CadQuery 设计与 API 覆盖报告](docs/cadquery-comparison.zh-CN.md)逐项映射其 API 索引的 150 个入口，列出缺口和证据。[ROADMAP](ROADMAP.zh-CN.md)总结架构、G1–G3 验收范围以及后续 benchmark/RSI 路线；G4 自动执行器尚未实现。

## 文档与帮助

从[外部 STEP 示例](examples/engineering-queries/README.md)可以上手[精确查询与几何驱动装配](docs/engineering-queries.zh-CN.md)。[四阶段目标](specs/0005-capability-reliability-and-optimization.zh-CN.md)依次为功能覆盖、几何可靠性、工程可用性和基准测试驱动的优化。SDK、CLI、工具 schema 和三平台原生内核同步交付。查询接口采用 Colocad 自身的 API。

G1–G3 已按[明确的验收范围](specs/experiments/0005/README.md)通过：24/24 几何案例独立验证、127 项 Rust / 41 项 Node 测试，以及 Linux arm64 **1 vCPU / 512 MiB** 内的两条真实 Pi 任务。语料执行加最小调度器的 cgroup 峰值约 **256 MiB**；驻留 Pi/harness 加 82 实体执行器改参和预览约 **371 MiB**，无 OOM。这是正确性与资源验收，不是新一轮 build123d 性能比较，也不代表全部工业场景对等；G4 尚未启动。


[入门](docs/getting-started.md) · [API](docs/api.md) · [完整建模指南](docs/agent-modeling-handbook.zh-CN.md) · [Skill](skills/colocad/SKILL.md) · [故障排查](docs/troubleshooting.md) · [全部文档](docs/README.zh-CN.md)

问题和功能建议请[提交 issue](https://github.com/Coloopx/colocad/issues)，附上版本、平台、最小模型和相关回执，并移除凭证及私密资料。

## 开发与贡献

```sh
git clone https://github.com/Coloopx/colocad.git
cd colocad
npm ci
npm run build:kernel
npm test
```

源码构建需要 Rust 工具链，并拉取固定版本的 SolidCore。查看器、视频和 Pi 接入测试另有依赖；安装、测试和发布方法见 [CONTRIBUTING.md](CONTRIBUTING.md)。欢迎提交 issue 和 pull request。

## 许可证与致谢

Colocad 应用代码使用 [MIT 许可证](LICENSE)。内核基于 [SolidCore](https://github.com/tangency-labs/solidcore)，其中包含源自 NVIDIA USD BRep / SMLib 的工作；查看器使用 Three.js。依赖许可证与归属见 [NOTICE.md](NOTICE.md) 和 [kernel/licenses](kernel/licenses)。工业实例与 HIWIN 无隶属或背书关系。
