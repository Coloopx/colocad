# colocad

[English](README.md) · [npm 包](https://www.npmjs.com/package/@coloopx/colocad) · [下载单文件对比报告：含四份 STEP](https://github.com/Coloopx/colocad/raw/refs/heads/main/docs/benchmarks/comparison.html)

让沙箱内的 agent 用 JSON 描述可改参的机械零件，输出 STEP 实体、PNG 预览和 GLB/STL 网格。**npm 包自带编译好的 Rust CAD 内核**：Linux x64、Linux arm64 和 macOS arm64 各一份。agent 和 harness 在同一沙箱里调用 Node CLI 或 TypeScript API，CLI 直接启动本地 Rust 程序。安装后建模不需要网络、Python、Rust 工具链或额外服务。[Rust 源码](crates/agent-cad/src)和[内核依赖版本](third_party/solidcore.lock.json)在本仓库。

以 Linux arm64 为例，安装后可执行文件就在 `node_modules/@coloopx/colocad/kernel/native/linux-arm64/agent-cad`。npm 分发编译产物；要读或修改 `.rs` 源文件，请用上面的 GitHub 源码链接。

## 两个建模后端做出来的东西一样吗？

![左列 colocad，右列 build123d。上排为三槽传感器导轨，下排从底部看四盲孔歧管，可见孔列和矩形浅槽。](docs/benchmarks/step-comparison.png)

左右两列分别从相同参数独立建模、导出 STEP，再用同一套 OCCT 三角化参数、相机和光照出图。四份 STEP 回读都是有效的单实体。导轨各有 22 个面、56 条边；歧管各有 27 个面、54 条边。两个零件分别做 A−B、B−A，测得差集体积均为 **0 mm³**（布尔容差 0.000001 mm）。[逐件几何核对](docs/benchmarks/step-checks.json)

## 同样的 1 vCPU / 512 MiB，能省多少？

![同一 Linux arm64 沙箱预算下，从进程启动到交付 STEP 的时间和容器峰值内存。具体数值见下表。](docs/benchmarks/sandbox-performance.svg)

下表每格均为 **colocad / build123d**，对比的是 npm 已发布的 **0.1.0-alpha.2**（Node CLI + 随包 Rust 可执行文件）和 **build123d 0.11.1 / OCCT**。

| 零件 | 进程启动到写完 STEP | 交付加速比 | 沙箱峰值内存 | 仅几何构建 |
| :--- | ---: | ---: | ---: | ---: |
| [三槽传感器导轨](examples/sensor-rail.json) | 113 / 2,485 ms | **22.0×** | 20.0 / 259.3 MiB | 26.9 / 41.1 ms |
| [四盲孔歧管](examples/manifold.json) | 230 / 2,963 ms | **12.9×** | 20.7 / 258.7 MiB | 53.7 / 43.8 ms |

每件、每后端预热一次，再交替执行五轮，每次使用新进程和新容器。20 次正式调用全部成功。时间为中位数；内存取五轮容器 cgroup 峰值的最大值，包括计入该容器的文件页和临时输出。计时包括进程启动、导入、建模、各自的验证和 STEP 导出，不含安装、容器启动、渲染、agent 推理；此实验也没有把常驻 agent 的内存算进去。

**最适合的是每次工具调用都启动新进程的场景。** build123d 在两个样件上仅导入就分别用了 2.01 s 和 2.45 s（中位数）。若 Python 解释器常驻，这笔成本可以摊薄。歧管只看几何阶段，build123d 的 43.8 ms 比 colocad 的 53.7 ms 更快。[原始逐轮数据](docs/benchmarks/runs.json) · [复现实验](benchmarks/readme/README.md)

## 如果 agent 只是转动视角呢？

首次预览会保存网格。同一个零件从等轴视图切到俯视图，SDK 可以用 `observeArtifact` 重新出图，跳过精确建模和三角化。同一版 colocad 在同样沙箱预算下得到以下五轮中位数：

| 切到俯视图 | 完整重建并出图 | 复用网格出图 | 加速比 |
| :--- | ---: | ---: | ---: |
| 导轨 | 108.8 ms | 19.4 ms | **5.6×** |
| 歧管 | 223.8 ms | 13.3 ms | **16.8×** |

每对输出的 PNG、GLB、STL 三个文件均逐字节相同。这一组只比较 colocad 的两条路径；SDK 已加载，计时包括启动本地 Rust 进程和写出显示文件。改几何参数需要生成新版本。[预览耗时和文件哈希](docs/benchmarks/preview-runs.json)

## 试跑并改参

需要 Node.js 22+。输出目录必须是新目录。

```sh
npm install @coloopx/colocad@0.1.0-alpha.2
npx colocad build node_modules/@coloopx/colocad/examples/manifold.json ./manifold --preview
```

打开 `manifold/part/model.step` 检查实体；`preview.png`、`model.glb` 和 `model.stl` 用于查看。歧管由六边形拉伸、盲孔切除、上缘圆角、引用种子孔的线性阵列和矩形浅槽构成。将 `instances` 从 4 改到 3，少切一个 Ø6 × 8 mm 的盲孔，实测恢复材料 **226.195 mm³**，与圆柱体积一致。[验证记录](docs/evidence/scoped-runner.json)

```js
import { readFile } from 'node:fs/promises';
import { build } from '@coloopx/colocad';

const tree = JSON.parse(await readFile('./node_modules/@coloopx/colocad/examples/manifold.json', 'utf8'));
tree.parameters.instances = 3;
const result = await build(tree, './three-port-manifold', { preview: true });
console.log(result.step);
```

公开特征接口支持矩形、圆和多边形草图拉伸、切除、按 Z 平面选边的定半径圆角，以及沿 X/Y 的拉伸或切除阵列。[类型定义](src/index.d.mts) · [更底层的内核 API](kernel/src/client.ts)

**交付边界：** Linux 已验证 STEP、网格和改参。Linux 直接生成复杂、可在 SOLIDWORKS 里编辑和重建的原生 `.SLDPRT` 尚未完成。仓库中的 Windows 作者程序已针对 SOLIDWORKS 2024 接口编译，尚未在有授权的 SOLIDWORKS 上完成回读重建。[验证说明](docs/validation.md) · [从源码构建](README.md#build-and-verify-from-source)
