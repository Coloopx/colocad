# 五款产品外观的 macOS arm64 建模实测

2026 年 11 月 17 日用 npm 正式包 **Colocad 0.1.1** 和 **CadQuery 2.8.0** 重新执行两种构建模式。从原压缩包核对了全部 15 个输入哈希。[English](benchmark-appearance.md) · [软件包指纹和镜像](benchmarks/stable-0.1.1/observations.json)。

## 实验口径

五款产品各有概念图、产品定义和分件表，15 份文件均与历史记录中的原始哈希匹配。根据这些输入和历史产出几何重建每款产品的规格，再交给两个后端执行。SenseCAP 在两端使用相同的圆形近似底座。原任务的模型推理、重试和视觉评审在本轮计时之外。

| 执行方式 | 计时范围 | 内存口径 |
| --- | --- | --- |
| 新进程 | 从启动 Node 或 Python，到加载依赖与模型、校验几何、导出 STEP 并退出。系统文件缓存已预热。 | 约每 10 ms 采样进程树 RSS 总和，取三次运行各自峰值的中位数。 |
| 已加载的宿主 | 从解析完毕的建模指令开始，复用已加载的 SDK 或 Python 模块，完整重建后导出到新目录。Colocad 的每次 SDK 构建仍启动原生子进程，两端都重新生成几何。 | 只测耗时。 |

每种模式、每个产品、每个后端均预热一次，再测三次。新进程测试交替执行两个后端。共 80 份 STEP 在计时结束后由 OCCT 独立回读。测试机未设 Linux cgroup 内存上限。[逐次耗时、新进程 RSS、输入标识与 STEP 哈希](benchmarks/stable-0.1.1/appearance.json)。

## 结果与几何检查

在 macOS arm64 上，新进程构建五款产品时，Colocad 0.1.1 的进程树峰值 RSS 中位数为 **71–84 MiB**，CadQuery 2.8.0 为 **479–492 MiB**。从进程启动计时，Colocad 有 **5/5** 例更快。从加载完毕的宿主开始，两端都重新建模，CadQuery 有 **5/5** 例更快。 [逐项数据](../README.zh-CN.md#五款产品的外观建模)。

一套五款产品共 67 个命名实体，回读检查了实体数量、有效性和 23 处材料或空气探针。139 个有指定数值的分件尺寸中，123 个可映射到独立实体，其中 109 个在 0.25 mm 以内，14 个超出阈值。其余 16 个缺少对应的独立实体读数。0.25 mm 是本次数字外观的对照阈值。StackChan 的 STEP 缺少屏幕表情与顶面印刷，Mori 仍有分件尺寸偏差。

## SenseCAP 底座

指定的截面为 49.9858 × 49.963 mm 椭圆，拉伸深度为 12.0991 mm。最小 Colocad 模型在原生实体验证时返回 `GEOMETRY_VALIDATION_FAILED`，CadQuery 生成的精确实体通过回读。两端计时都使用直径 49.9858 mm 的圆形底座，短轴方向因此增加 0.0228 mm。[最小复现模型](benchmarks/stable-0.1.1/mount-base-ellipse.json) · [回归结果](benchmarks/stable-0.1.1/appearance.json)。

将上面的模型保存为 `mount-base-ellipse.json`，在已安装 `@coloopx/colocad@0.1.1` 的 Node.js 22+ 项目中运行。使用新的输出目录。

```sh
node node_modules/@coloopx/colocad/skills/colocad/scripts/build.mjs \
  mount-base-ellipse.json ./ellipse-check
```

为 Linux agent 沙箱做资源预算时，可查看每个任务一核、按 cgroup 测内存的独立实验。[机械用例和连续改图实测](benchmark-stable.zh-CN.md)。
