# 五款产品外观的 macOS arm64 建模实测

2026 年 11 月 14 日对比了 Colocad 0.1.0-alpha.11-rc.1 和 CadQuery 2.8.0。正式版 0.1.0 所含 SDK、CLI 和原生可执行文件与实测版本逐字节一致。公共 npm 的正式包另经干净安装，并实跑了随包歧管。[English](benchmark-appearance.md) · [版本比对](https://github.com/Coloopx/colocad/releases/download/v0.1.0/release-validation.json) · [公共安装回执](https://github.com/Coloopx/colocad/releases/download/v0.1.0/registry-proof.json)。

## 实验口径

五款产品各有概念图、产品定义和分件表，15 份文件均与历史记录中的原始哈希匹配。根据这些输入和历史产出几何重建每款产品的规格，再交给两个后端执行。SenseCAP 在两端使用相同的圆形近似底座。原任务的模型推理、重试和视觉评审在本轮计时之外。

| 执行方式 | 计时范围 | 内存口径 |
| --- | --- | --- |
| 新进程 | 从启动 Node 或 Python，到加载依赖与模型、校验几何、导出 STEP 并退出。系统文件缓存已预热。 | 约每 10 ms 采样进程树 RSS 总和，取三次运行各自峰值的中位数。 |
| 已加载的宿主 | 从解析完毕的建模指令开始，复用已加载的 SDK 或 Python 模块，完整重建后导出到新目录。Colocad 的每次 SDK 构建仍启动原生子进程，两端都重新生成几何。 | 只测耗时。 |

每种模式、每个产品、每个后端均预热一次，再测三次。新进程测试交替执行两个后端。共 80 份 STEP 在计时结束后由 OCCT 独立回读。测试机未设 Linux cgroup 内存上限。[逐次耗时、新进程 RSS、输入标识与 STEP 哈希](benchmarks/appearance-0.1.0/measurements.json)。

## 结果与几何检查

Colocad 的新进程峰值为 69–83 MiB，CadQuery 为 481–495 MiB。Mori 的新进程耗时最接近，两端分别为 2.24 秒和 2.29 秒。复用已加载的宿主时，CadQuery 在五例的完整重建中均快 4.5–8.6 倍。Mori 的 CadQuery 为 0.38 秒，Colocad 为 2.12 秒。[五例逐项数据与 STEP 四视图](../README.zh-CN.md#五款产品的外观建模)。

一套五款产品共 67 个命名实体，回读检查了实体数量、有效性和 23 处材料或空气探针。139 个有指定数值的分件尺寸中，123 个可映射到独立实体，其中 109 个在 0.25 mm 以内，14 个超出阈值。其余 16 个缺少对应的独立实体读数。0.25 mm 是本次数字外观的对照阈值。StackChan 的 STEP 缺少屏幕表情与顶面印刷，Mori 仍有分件尺寸偏差。

## SenseCAP 底座

指定的截面为 49.9858 × 49.963 mm 椭圆，拉伸深度为 12.0991 mm。最小 Colocad 模型在原生实体验证时返回 `GEOMETRY_VALIDATION_FAILED`，CadQuery 生成的精确实体通过回读。两端计时都使用直径 49.9858 mm 的圆形底座，短轴方向因此增加 0.0228 mm。同一个精确椭圆在 Colocad alpha.9 和 alpha.10 也会触发此错误。[最小复现模型](benchmarks/appearance-0.1.0/mount-base-ellipse.json) · [回归结果](benchmarks/appearance-0.1.0/measurements.json)。

将上面的模型保存为 `mount-base-ellipse.json`，在已安装 `@coloopx/colocad@0.1.0` 的 Node.js 22+ 项目中运行。使用新的输出目录。

```sh
node node_modules/@coloopx/colocad/skills/colocad/scripts/build.mjs \
  mount-base-ellipse.json ./ellipse-check
```

为 Linux agent 沙箱做资源预算时，可查看每个任务一核、按 cgroup 测内存的独立实验。[机械用例和连续改图实测](benchmark-alpha11-rc1.zh-CN.md)。
