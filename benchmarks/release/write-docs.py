"""Render the bilingual release report and the matching README sections."""
import json,statistics,sys
from pathlib import Path
repo=Path(__file__).resolve().parents[2];dest=repo/'docs/benchmarks/alpha9';stats=json.loads((dest/'observations.json').read_text())
lookup={r['case']:r['backends'] for r in stats['summary']};bs=['colocad','cadquery','build123d'];windows=stats['windows'];assert all(windows[b]['status']=='ok' for b in bs)
controls=json.loads((dest/'controls.json').read_text())
def peak(label):return statistics.median(r['peak_bytes']/2**20 for r in controls['queries']['runs'][label])
def wins(metric):
 return sum(v['colocad'][metric]['median']<min(v[b][metric]['median'] for b in bs[1:]) for key,v in lookup.items() if '/' in key)
def fmt(k,metric,dec=2,denom=1):
 return ' / '.join(f"{lookup[k][b][metric]['median']/denom:,.{dec}f}" for b in bs)
def win(k,n):
 return ' / '.join(f"{windows[b][k]:.{n}f}" for b in bs)
cases=[('sensor-rail','Sensor rail','传感器导轨'),('manifold','Port manifold','歧管'),('finger','Proximal link','近端指节'),('hand','Complete hand, 43 solids','完整灵巧手，43 个实体'),('repeated/128','128 shared instances','128 个共享实例'),('unique/64','64 distinct plates','64 个独立零件'),('part/1024','1,024 holes','1,024 孔'),('chain/64','64 sequential cuts','64 次连续切孔')]
def table(zh):
 title='| 任务 | 交付 STEP，秒 | 峰值内存，MiB |' if zh else '| Task | Time to STEP (s) | Peak memory (MiB) |'
 return '\n'.join([title,'|---|---:|---:|']+[f'| {ct if zh else en} | {fmt(k,"wall_s")} | {fmt(k,"peak_mib",1)} |' for k,en,ct in cases])
dataurl='https://github.com/Coloopx/colocad/releases/download/v0.1.0-alpha.9/scorecard.html'
order='Colocad / CadQuery / build123d'
for zh in (False,True):
 if zh:
  report=f'''# alpha.9：什么时候该选 Colocad

这轮测试支持两个不同的结论：在新的沙箱进程中，小零件和大量**相同定义**的实例很适合 Colocad；密集孔、长布尔链和大量**独立定义**并不是它的速度强项。以下三项数值始终按 **{order}** 排列。[下载单文件交互报告]({dataurl}) · [English](benchmark-alpha9.md)。

{table(True)}

**读图。** 主矩阵是 4 类模型 × 3 个规模 × 3 个后端 × 3 次，共 108 份 STEP。每个规模按中位数比较，Colocad 在 {wins("peak_mib")}/12 个规模点内存最低，在 {wins("wall_s")}/12 个规模点用时最短。npm 安装后可直接打开 `node_modules/@coloopx/colocad/docs/benchmarks/alpha9/scorecard.html`。每个规模各算一票，这不是实际业务的加权平均。另外 11 个同输入产品/改参案例，各后端三次，共 99 份 STEP。全部 207 份都经过独立的几何回读。耗时从启动 CAD 进程算到校验、导出并退出；主矩阵 2 GiB，产品案例 512 MiB，均为 Linux arm64、1 vCPU、无网络、无 swap。不计安装、渲染、推理、常驻 agent/harness。复测在共享主机运行，容器隔离不等于冷文件缓存或独占主机。

![耗时与峰值内存](benchmarks/alpha9/scorecard.svg)

**I/O 应看哪一列？** 歧管最终 STEP 分别为 **{fmt('manifold','step_bytes',1,1024)} KiB**，全部保留产物为 **{fmt('manifold','retained_bytes',1,1024)} KiB**，进程逻辑写入为 **{fmt('manifold','wchar',1,1024)} KiB**。Colocad 默认保存检查点和回执；即使只数最终 STEP，不同编码器的大小也不同。逻辑写入来自 Linux `/proc/PID/io`，包括已等待的子进程及标准输出，不是 SSD 写入量。文件缓存和共享挂载会使设备计数空缺，不能推出物理盘吞吐更高。

![构建 I/O 三种口径](benchmarks/alpha9/build-io.svg)

**100 秒灵巧手。** 同一参数队列，均从零建模。三个后端分别交付 **{win('count',0)}** 个回读有效的 43 实体 STEP；首份交付 **{win('first_s',2)} 秒**；25 ms 采样的最高内存 **{win('peak_sample_mib',1)} MiB**。每个后端只测一个窗口。更低内存没有换来这个产品的更高吞吐。GIF 的内存线为从零时刻起的运行峰值；测量数据保留瞬时值及下降段。

**改参、选择与装配。** 三方都完成并验证了导轨槽数 3→2、节距 19→17 mm；歧管盲孔 4→3、直径 6→7 mm。灵巧手按相同队列改变导孔和腱径。Colocad 的价值在于 `head → inspect → edit → inspect` 把输入、精确测量、改参和过期引用绑定在同一个可回放会话里。[CLI 操作实例](../skills/colocad/references/modeling-loop.md#inspect-and-edit-from-a-shell)。这些是工程任务验收，不是 API 灵活性排名。既有 [CadQuery API 审计](cadquery-comparison.zh-CN.md)的 150 项中，36 项在限定域有对应实现、30 项部分、29 项可组合、29 项没有对应契约、26 项采用不同抽象。它不是 150 个差分测试，也不提供 build123d 全覆盖率。

**能验证的写入优化。** 同一 256 孔外部 STEP 连续记录 24 次精确检查，与已经有索引但尚未共享源文件的内部基线相比，25 条记录对应的独立输入由 25 份（25,945,225 B）减到 1 份（1,037,809 B），三次中位峰值从 {peak("before"):.2f} 降到 {peak("after"):.2f} MiB。结果摘要一致，零次重建、STEP 导出或三角化。这是 Colocad 内部对照，不是与 Python 的比较。

**下一步。** 优先看独立零件 STEP 的重复编码、检查点写入量和密集孔的几何阶段；固定模型、输出契约和回读阈值再做消融。索引历史和对同一输入重复查询的收益不能推广到整个 B-rep 的内存复杂度。2,048 个独立件在 CadQuery/build123d 的 4 GiB 配额下成功，降到 2 GiB 后均 OOM；Colocad 的单文件规模受特征、实例及 32 MiB STEP 等限额约束。拒绝、超时、OOM 以及分块项目都不算一次成功的完整装配 STEP。

[原始观测及范围](benchmarks/alpha9/observations.json) · [复现脚本](../benchmarks/release/README.md) · [扩展压力实验](benchmarks/alpha9/controls.json) · [本版 KK60 回读](benchmarks/alpha9/kk60-readback.json) · [逐次原始采样包](https://github.com/Coloopx/colocad/releases/download/v0.1.0-alpha.9/raw-observations.tar.gz)。KK60 四视图来自 16 个真实阶段；没有在另外两套库上对 KK60 计时。'''
 else:
  report=f'''# Alpha.9 benchmark: choose by workload

Colocad is a good fit for short tool calls and assemblies with many **occurrences of the same definition**. Independent definitions, dense perforations and long Boolean chains change the result. Each cell below is **{order}**. [Download the standalone HTML]({dataurl}) · [中文](benchmark-alpha9.zh-CN.md).

{table(False)}

**Reading the results.** Four geometry families × three sizes × three backends × three repetitions produced 108 STEPs. Across those 12 sizes Colocad had the lowest median peak in {wins("peak_mib")} and the shortest median wall time in {wins("wall_s")}. The npm tarball includes the complete offline report at `node_modules/@coloopx/colocad/docs/benchmarks/alpha9/scorecard.html`. These equally weighted points do not represent a production workload mix. Eleven additional product/parameter cases × three backends × three repetitions produced 99 STEPs. All 207 were independently reopened. Every common-case solid was checked for validity, placement, bounds, volume and bore geometry; product checks matched solids and material, with two-way material differences on the first repeat. Time includes process startup, construction, backend checks and export, excluding installation, inference, the resident harness, post-run verification and rendering. Native Linux arm64, one vCPU, no network/swap: 2 GiB for the scaling matrix and 512 MiB for product fixtures. Fresh processes on a shared host do not imply cold file caches or dedicated hardware.

![Time to STEP and peak memory in eight workloads](benchmarks/alpha9/scorecard.svg)

**Build I/O.** On the manifold, final STEP sizes were **{fmt('manifold','step_bytes',1,1024)} KiB**, retained outputs **{fmt('manifold','retained_bytes',1,1024)} KiB**, and logical writes **{fmt('manifold','wchar',1,1024)} KiB**. Colocad persists checkpoints and receipts by default. Even the final STEP differs in size across writers. Linux `/proc/PID/io` counts calls made by the process and its waited-for children, including stdout. It does not measure SSD traffic or fsync latency. Host bind mounts and warm cache make device counters incomplete.

![STEP, retained artifacts and logical writes](benchmarks/alpha9/build-io.svg)

**100 seconds of tendon-hand edits.** From the same ordered parameter queue the backends delivered **{win('count',0)}** independently reopened 43-solid STEPs. First handoff: **{win('first_s',2)} s**; highest 25 ms sampled cgroup memory: **{win('peak_sample_mib',1)} MiB**. One window per backend. Colocad used less memory and delivered fewer full hands. The GIF plots peak-so-far; instantaneous samples, including dips, are retained in the JSON.

**Editing and assembly.** All three backends passed the rail change (three slots to two; pitch 19→17 mm) and manifold change (four blind bores to three; diameter 6→7 mm). Each hand queue changes guide diameter and tendon radius. Colocad provides a recorded `head → inspect → edit → inspect` workflow with exact measurements and stale-reference checks. [Runnable CLI example](../skills/colocad/references/modeling-loop.md#inspect-and-edit-from-a-shell). The experiment establishes edit behavior, not a general API flexibility ranking. The earlier [CadQuery audit](cadquery-comparison.md) mapped 150 API entries: 36 analogous within a supported domain, 30 partial, 29 recipes, 29 unsupported and 26 different abstractions. It was neither 150 differential tests nor a build123d coverage study.

**Where writes actually fell.** For 24 recorded inspections of one imported 256-hole STEP, 25 ledger entries referenced 1 input (1,037,809 B), down from 25 copies (25,945,225 B) on the already-indexed internal baseline. Median cgroup peak fell from {peak("before"):.2f} to {peak("after"):.2f} MiB across three runs. Measurement digests matched, with zero rebuilds, STEP exports or tessellations. This is a Colocad ablation, not a Python comparison.

**Next optimization targets.** Measure duplicate STEP encoding and checkpoint traffic on distinct parts, and isolate construction on dense grids. Hold delivery and readback constant. Indexed history and shared query inputs do not make whole-model memory logarithmic. The 2,048-distinct-part baseline completed on CadQuery/build123d at 4 GiB and both hit OOM at 2 GiB; Colocad's monolithic path has feature, instance and 32 MiB STEP limits. Rejection, timeout, OOM and an indexed folder of shards are not successful equivalents of one complete STEP.

[All observations and ranges](benchmarks/alpha9/observations.json) · [Reproduction](../benchmarks/release/README.md) · [Extended stress observations](benchmarks/alpha9/controls.json) · [KK60 readbacks](benchmarks/alpha9/kk60-readback.json) · [Raw traces](https://github.com/Coloopx/colocad/releases/download/v0.1.0-alpha.9/raw-observations.tar.gz). The KK60 cover visualizes 16 actual Colocad stages; there is no comparative KK60 timing.'''
 (repo/'docs'/('benchmark-alpha9'+('.zh-CN' if zh else '')+'.md')).write_text(report+'\n')
 name=repo/('README.zh-CN.md' if zh else 'README.md');s=name.read_text();start='## 性能' if zh else '## Performance';end='## 支持范围与限制' if zh else '## Support and limits'
 url='https://raw.githubusercontent.com/Coloopx/colocad/v0.1.0-alpha.9/docs/benchmarks/alpha9/'
 md=(f'''**alpha.9 三方复测。** 针对相同几何，207 份 STEP 均通过独立回读。在主矩阵的 12 个规模点里，Colocad 有 **{wins("peak_mib")} 个峰值内存最低、{wins("wall_s")} 个用时最短**。小零件、共享实例占优；大量独立件、密集孔和长布尔链的耗时由其他后端领先。

![三后端构建时间与内存]({url}scorecard.svg)

**同一只 43 实体灵巧手，100 秒连续改参：** 按 Colocad / CadQuery / build123d 顺序，交付 **{win('count',0)}** 份有效 STEP，最高采样内存为 **{win('peak_sample_mib',1)} MiB**。Colocad 内存最低，两套 Python 后端交付更多。

![100 秒真实队列十倍速回放]({url}hand-window.gif)

默认检查点换来了可检查的构建产物，也会增加写入量。[方法、构建 I/O 图与改参验收](docs/benchmark-alpha9.zh-CN.md) · [下载单文件 HTML]({dataurl}) · [历史实验](docs/performance.md)。''' if zh else f'''**Alpha.9 compared with CadQuery 2.8.0 and build123d 0.13.0.** All 207 STEP files from the head-to-head fixtures passed independent readback. Across the 12 scaling points Colocad had the **lowest median peak memory in {wins("peak_mib")}** and the **shortest median time in {wins("wall_s")}**. Small parts and shared instances favor Colocad; dense bores, long cut chains and many distinct definitions favor the Python backends on elapsed time.

![Three-backend time to STEP and memory measurements]({url}scorecard.svg)

**The same 43-solid hand, 100 seconds of edits:** Colocad / CadQuery / build123d delivered **{win('count',0)}** verified assemblies at sampled peaks of **{win('peak_sample_mib',1)} MiB**. Colocad used the least memory; the Python backends delivered more complete hands.

![Ten-times replay of actual hand deliveries and sampled memory peaks]({url}hand-window.gif)

Checkpoints support inspection and add build I/O. [Methods, I/O chart and edit acceptance](docs/benchmark-alpha9.md) · [Download standalone HTML]({dataurl}) · [Earlier experiments](docs/performance.md).''')
 name.write_text(s[:s.index(start)]+start+'\n\n'+md+'\n\n'+s[s.index(end):])
