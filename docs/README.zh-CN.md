# 开发与接入指南

[实际操作、输入输出与效果提升方法](modeling-workflow.zh-CN.md) · [CLI 查看器与离线导出（开发分支）](modeling-history.md)

colocad 的分层是 **SDK 为基础，CLI / 工具负责调用，skill 指导建模流程**。Agent、harness 和 Rust CAD 进程都在同一个沙箱内，共享项目目录。现有 harness 负责模型推理、权限、会话和产物保存，不需要额外部署 CAD 服务。

本文对应 `@coloopx/colocad@0.1.0-alpha.3`，要求 Node.js 22+；预编译内核支持 Linux x64/arm64、macOS arm64。工程仍处于 alpha 阶段。

## 先跑通一个零件

在已有 Node 项目根目录执行；没有 `package.json` 时先运行 `npm init -y`：

```sh
npm install --save-exact @coloopx/colocad@0.1.0-alpha.3
./node_modules/.bin/colocad build \
  node_modules/@coloopx/colocad/examples/manifold.json ./manifold-v1 --preview
```

查看 `manifold-v1/part/preview.png`，交付 `model.step`，保留 `feature-tree.json` 和 `report.json`。输出目录必须是新目录。这个示例包含拉伸、盲孔切除、圆角和阵列。

常用结构可以从 skill 的参数模型开始：

```sh
node node_modules/@coloopx/colocad/skills/colocad/scripts/build.mjs \
  node_modules/@coloopx/colocad/skills/colocad/assets/plate-grid.json \
  ./plate-v1 --set length=100 --set xCount=3 --set xPitch=32 --preview
```

命令返回实际 `directory`，其中有 `model.json`、STEP 和预览。这里是 kernel model；上面的歧管是 feature tree。两种格式的入口不同，不能混用。[格式与产物说明](concepts.md) · [完整入门](getting-started.md)

## 接入哪个 harness

| 环境 | 推荐方式 | 安装位置 / 调用 |
| --- | --- | --- |
| Codex | 完整 skill + 自带终端和图像工具 | 项目 `.agents/skills/colocad`；`$colocad` |
| Claude Code | 完整 skill + Bash/Read | 项目 `.claude/skills/colocad`；`/colocad` |
| Pi coding agent | skill；需要直接工具时加 extension | `.agents/skills/colocad` 或 `--skill`；`/skill:colocad` |
| Pi agent core / 自建 TS harness | `createCadTools` 适配到现有工具注册机制 | 模型、鉴权、输出目录由现有 harness 提供 |

以 Codex 为例，在项目根目录复制完整 skill：

```sh
mkdir -p .agents/skills
test ! -e .agents/skills/colocad && \
  cp -R node_modules/@coloopx/colocad/skills/colocad .agents/skills/colocad
```

已有 skill 时先检查本地修改，再替换整个目录。Claude Code 将目标目录换为 `.claude/skills/colocad`。不要只复制 `SKILL.md`：脚本、资产和参考文档都需要。

可直接给 agent 的任务：

> 用 colocad 做一块 100 × 50 × 6 mm 的安装板，中心对称布置 3 × 2 个 Ø6 通孔，两向孔距 32、30 mm。保留可改参 JSON，导出 STEP，检查俯视和等轴视图，说明采用的基准与推断尺寸，返回产物路径。

各环境的完整命令和验收方法见 [Codex](integrations/codex.md)、[Claude Code](integrations/claude-code.md)、[Pi core / coding agent](integrations/pi.md)。Pi 示例固定到 1.5.0；不要为了接入 CAD 自动升级已有 harness 的 Pi 依赖。旧版本可先采用 skill + CLI，并对自定义工具适配单独回归。

## SDK 接入需要处理的事情

实例化 `new CadClient({ outputRoot, wallMs: 30000 })`，按顺序执行 `validate → build/preview → 检查 → 接受版本`。`outputRoot` 由 harness 决定。`createCadTools` 提供 schema 和执行函数；Pi 适配示例将失败回执变成错误工具结果，并将 PNG 作为图像传回模型。

每个独立建模会话使用自己的 client 和输出根目录；同一会话顺序执行，便于复用特征检查点和网格。`edit` 不会修改传入的模型对象，接受修改后必须保存新 `model.json` 或合并参数，避免下一次改参回退到旧版本。

仅转视角使用缓存预览。观察产物目录可能只有预览；STEP 在 `source_artifact_directory` 指向的几何版本中。沙箱销毁前，用现有 harness 的产物机制保存 JSON、STEP、回执和预览。SDK 没有内置云存储或会话数据库。

[架构与生命周期](integrations/overview.md) · [SDK / CLI / 工具 API](api.md) · [可运行示例](../examples/integrations/README.md)

## 建模最佳实践

- 从控制外形的截面或毛坯开始；板、轴套、角码、壳体优先参考[快速搭建指南](../skills/colocad/references/quick-build.md)，按实际图纸改参数和特征。
- 先确定单位、基准、接口尺寸和未知项；稳定特征 ID，保留每次接受的参数源。四视图看不到的内部结构要明确标注推断。
- 重复零件只定义一次，用装配实例定位；物理上独立的关节不要随意融合成一个实体。
- `validate` 通过只代表预检查。验收还要看实体数、边界、体积、孔深和内部区域；重要交付独立回读 STEP。
- 失败要报告具体特征。不要为了生成图片静默跳过切孔或圆角；显式降级也应保留在交付说明中。
- 性能分别测冷启动到 STEP、连续改参、缓存预览、包含推理的完整 agent 周期。CAD 的 512 MiB 基准不等于整个 agent/harness 也只需 512 MiB。

[详细最佳实践](best-practices.md) · [常见故障](troubleshooting.md) · [贡献与回归](../CONTRIBUTING.md)

## 当前验证边界

接入示例用已发布 npm 包执行实际 Rust 建模，并经过 Pi core 工具循环、错误恢复、扩展加载和 skill 发现测试。模型回复采用确定性脚本，因此这是接入回归，不是未知图纸上的自主建模成功率测试。Claude Code 尚未做实机推理验收。

参数 JSON 可重建，STEP 是精确几何；二者不等于 SOLIDWORKS 原生特征树。Linux 通用可编辑 `.SLDPRT` 尚未完成，Windows author 尚缺授权软件内的完整回读重建验收。[接入验证范围](integrations/verification.md) · [几何与原生输出验证](validation.md)
