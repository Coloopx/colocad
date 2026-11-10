import argparse,json
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('docs',type=Path);a=p.parse_args()
e=a.docs/'evidence/memory-2026-11-11';j=json.loads((e/'memory-study.json').read_text());verified=json.loads((a.root/'verification.json').read_text());manifest=json.loads((a.root/'verification-manifest.json').read_text())
assert len(verified)==len(manifest) and all(v['passed'] for v in verified)
def summary(c,b,f,n,v=''):
    return next(s for s in j['summaries'] if (s['campaign'],s['backend'],s['family'],s['n'],s['variant'])==(c,b,f,n,v))
def med(c,b,f,n,m='peak_mib',v=''):return summary(c,b,f,n,v)[m]['median']
def table(headers,rows):return '| '+' | '.join(headers)+' |\n| '+' | '.join(['---']*len(headers))+' |\n'+''.join('| '+' | '.join(map(str,r))+' |\n' for r in rows)
bs=['colocad','cadquery','build123d']
common=table(['Workload / 负载','Colocad','CadQuery','build123d'],[[label,*[f"{med('main',b,f,n):.1f} MiB / {med('main',b,f,n,'wall_s'):.2f} s" for b in bs]] for f,n,label in [('part',1024,'1 solid, 1,024 holes'),('unique',64,'64 distinct 16-hole parts'),('repeated',128,'128 shared-definition instances'),('chain',64,'64 sequential cuts')]])
boundary=table(['Backend','Cap MiB','Outcome','Peak MiB','Last completed phase'],[[r['backend'],r['memory_mib'],r['status'],f"{r['peak_mib']:.1f}",r['last_phase']] for r in j['rows'] if r['campaign']=='boundary'])
control=table(['Backend','Family','N','Outcome','Peak MiB','Last completed phase'],[[r['backend'],r['family'],r['n'],r['status'],f"{r['peak_mib']:.1f}",r['last_phase']] for r in j['rows'] if r['campaign']=='compositional'])
history=table(['Entries / 条目','SDK JSON peak MiB','Indexed Node peak MiB','SDK outcome'],[[n,f"{med('history','colocad','history',n,v='sdk'):.2f}",f"{med('history','colocad','history',n,v='indexed-node'):.2f}",'/'.join(sorted(set(summary('history','colocad','history',n,'sdk')['statuses'])))] for n in [1000,10000,100000,500000]])
chunks=table(['Backend','Parts','Peak MiB','Anon MiB','Seconds','STEP output MiB'],[[b,n,f"{med('sharded',b,'unique',n):.2f}",f"{med('sharded',b,'unique',n,'anon_mib'):.2f}",f"{med('sharded',b,'unique',n,'wall_s'):.2f}",f"{med('sharded',b,'unique',n,'step_mib'):.2f}"] for b in bs for n in [32,128,512]])
native=table(['Native mode','Peak MiB','Anon MiB','Seconds'],[[r['variant'],f"{r['peak_mib']:.2f}",f"{r['anon_mib']:.2f}",f"{r['wall_s']:.2f}"] for r in j['rows'] if r['campaign']=='native-control'])
links='[Single-file HTML](evidence/memory-2026-11-11/memory-study.html) · [JSON](evidence/memory-2026-11-11/memory-study.json) · [CSV](evidence/memory-2026-11-11/memory-study.csv) · [Raw evidence](evidence/memory-2026-11-11/raw-evidence.tar.gz) · [Reproduction](../benchmarks/memory/README.md)'
cn='''# CAD 内存增长、沙箱极限与持久化优化报告

2026-11-11 · 基线：Colocad alpha.8 / native 0.5.1；目标版本 alpha.9 仍为计划。[English](memory-persistence-report.md)。

'''+links+'''

立项：[issue #1](https://github.com/Coloopx/colocad/issues/1) · [alpha.9 milestone](https://github.com/Coloopx/colocad/milestone/1)。

## 结论

**2 GiB OOM 已复现；Colocad 也没有普遍的低内存或 O(log N) 保证。** 独立零件、共享实例、单个大实体、历史长度是不同的增长轴。此次定位到整装配验证和完整 STEP 导出两类峰值，也证实当前会话 JSON 账本会随历史长度线性增加内存。

下版本应采用**磁盘索引 + 有界活动几何 + 流式产物 + 分页查看器**。已完成索引和分块持久化实验原型；正式 SDK 迁移、跨块配合/碰撞、大型单实体处理与单文件 STEP 流式导出尚未交付。实验控制和失败样本都保留，不把原型写成已发布能力。

## 口径与有效性

- 原生 Linux arm64，1 vCPU，无网络、无 swap；正常/极限 CAD 任务 2 GiB，2,048 件边界对照另测 4 GiB；历史和分块原型 512 MiB。
- Colocad 使用实际 alpha.8 SDK 与捆绑 native；CadQuery 2.8.0 / OCP 7.9.3.1.1，build123d 0.13.0 / OCP-novtk 8.0.1.1.0，隔离 Python 环境，NumPy 均为 2.5.4。它们支持的 OCCT 版本不同，差异不能全部归因于语言或单个内核。
- 内存主指标为 cgroup v2 的内核峰值，包含监控器、全部后代进程和计入容器的文件页；另外每 20 ms 记录 anon、file、kernel 和进程 RSS。几个分量各自的最大值不一定同时发生，不能相加充当总峰值。计费/限制口径依据 [Linux cgroup v2](https://docs.kernel.org/admin-guide/cgroup-v2.html)。
- 不包含安装、推理、常驻 agent/harness、浏览器、动画与独立验证器。正式产品仍要额外验收整个 agent+harness 在 2 GiB 中的表现。当前运行结构始终是它们处于同一沙箱，没有新增远程 worker。
- 常规范围共 108 次，三个后端、四类负载、三个规模，各重复三次并预先随机顺序。图中区间是观测最小/最大值，不是置信区间。每次新进程，但主机镜像页缓存已预热。
- 这是共享开发环境，主机上历史夹具生成曾与常规运行短暂重叠；耗时仅作描述，不能用于小幅提速的正式验收。计量 CAD campaign 串行运行。扩展/控制实验多为单次，512 件分块也是单次，128 件分块重复三次。
- 正常孔阵列使用合理的直接截面拉伸构造，未强迫 Python 做 N 次布尔；连续切除另列。独立件宽度随索引变化，不能把它们当相同零件去重。共享实例分别使用原生装配、CadQuery Assembly 和 build123d Compound(children)。全部要求精确 STEP 与解析体积。

'''+f"计划 {j['scheduled']} 项，实际执行 {j['executed']} 项；另外 6 项按预定失败边界规则未运行。173 项成功、10 项 cgroup OOM、3 项 V8 heap OOM、4 项策略拒绝、4 项超时。成功项包含元数据查询，不能称为 173 个工业模型。独立 OCCT 回读 **{len(verified)} 份代表性交付**，检查每个实体有效性/尺寸/位姿、实体数、全部孔轴/半径/中心和解析体积；15 个分块目录的全部 chunk hash、连续覆盖和 catalog integrity 均通过。\n"+'''
验证器在单独的 4 GiB 环境中运行，逐实体检查，避免重新引入被测的整个 Compound 临时分配。对 SolidCore 是另一内核回读；对 CadQuery 是同族 OCCT 的独立导入加解析检查，不能称为独立内核证明。分块几何独立回读抽取每个后端最大案例的首/中/末块，其余块保留构建期检查和全量 hash。原始记录见 [回读结果](evidence/memory-2026-11-11/verification.json)和[目录检查](evidence/memory-2026-11-11/catalog-verification.json)。

## 正常范围：内存优势确实会随负载改变

以下为三次中位数，内存/总耗时。完整统计和波动见 JSON。

'''+common+'''
![Memory scaling](evidence/memory-2026-11-11/memory-scaling.svg)

共享实例是 Colocad 当前的强项；独立定义在本夹具中增长更快，64 件时已经高于另外两套实现。1,024 孔单实体也更慢、占用更高。原生回执显示大型孔板及独立装配约 85% 的时间在 validation 分组，它包含有效性、流形、质量属性、包围盒和材料统计，不能未经细分就指定为其中某一个函数。此前灵巧手/导轨的性能结果仍只适用于其原始负载。

## 2 GiB 边界与因果控制

4,096 独立件和 4,096 共享实例在两套 Python 基线均出现 cgroup OOM。1,024 独立件仍完成：CadQuery 约 1,238 MiB，build123d 约 1,268 MiB。16,384 孔单实体和 1,024 步连续切除先触发 120 秒超时，**不能把超时算作 OOM 或完成后的内存点**。

对同一个 2,048 独立件任务提高上限，再降回 2 GiB：

'''+boundary+'''
细分事件证明：2,048 件时有效性、体积和实体枚举已经完成，失败在 STEP 导出调用期间。4 GiB 完成时实际峰值已超过 2 GiB。最初 4,096 件日志只能定位到验证分组；不能单凭那份粗日志确定内部函数。

随后仅改变验证策略：独立定义先验有效性，再做已知刚体位姿组合，仍计算整体体积/数量并导出完整装配 STEP。

'''+control+'''
共享 4,096 实例通过了独立回读，说明全装配重复有效性验证是这组负载的重要峰值来源。但独立定义仍在完整 STEP 导出期间 OOM：仅改验证不足以解决导出峰值。不能把这个优化机械套到任意相交布尔、畸形装配或需要跨件干涉检查的任务。

Colocad 对 4,096 孔、128 个两特征独立件、256 实例、128 次切孔分别给出 RESOURCE_LIMIT。当前边界包括 128 特征/实例、1024 切孔以及 32 MiB STEP 预算。**这是策略拒绝，不是完成了大模型，也不能据此声称不会超过 2 GiB。**

## 持久化原型：量化收益和代价

### 历史索引

用一个真实 accepted entry 的结构扩展成合成元数据，保持相同 Node 22.23.3 和 SDK 导入，只把整本 JSON head 查询改为 SQLite 索引 head 查询：

'''+history+'''
10 万条从约 313 MiB 降至约 18.3 MiB；50 万条索引查询也完成。SDK 50 万条发生 V8 heap OOM（SIGABRT），cgroup `oom_kill=0`，与 Python CAD 的内核 OOM 区分。旁路 Python/SQLite 控制约 13 MiB，但主要结论采用同 Node 对照，避免混入运行时固定差异。**这不是 50 万次真实建模或追加吞吐测试**。页缓存配置 2 MiB 是局部配置，不能当进程总上限；依据 [SQLite cache_size](https://www.sqlite.org/pragma.html#pragma_cache_size)和[索引查询](https://www.sqlite.org/queryplanner.html)。

### 分块项目

每批 8 个独立零件，子进程结束释放几何，STEP/hash/fsync 后事务登记，按索引定位块，并对已完成的自身文件做 advisory page-cache release。三后端使用相同几何和同一分块交付契约：

'''+chunks+'''
32→512 件时匿名峰值基本不增长。Colocad 512 件约 97.7 MiB 总峰值、60.1 MiB anon；另外两套也保持各自的平台期。它验证的是**有界工作集策略**，不是某种语言独占的能力。时间和磁盘仍增长：Colocad 512 件约 253.7 秒、241.5 MiB STEP；另外两套约 38.1 MiB STEP。本夹具的原生 STEP 较大，也是下一版导出设计需要调查的对象，不凭文件大小推断几何精度优劣。

这里输出可索引的完整分块项目，**不是一个完整装配 STEP 文件**；只包含已定义的分离零件，尚未验收跨块配合/碰撞或任意工业曲面。目录可以重开，事务中断不提交半条记录的单测通过；整任务自动续跑、源/索引迁移和生产并发仍属后续。

### 当前 native 的缓冲开销

同一直接 native 入口和 64 件模型，单次消融（不与 SDK 入口直接当成优化比例）：

'''+native+'''
关闭 checkpoint 只能去掉一部分峰值；全部独立定义、交换表示与 STEP 缓冲仍在。checkpoint 的 12 MiB 和查询几何的 64 MiB 限额只约束局部，不是整体内存保证。

![Persistence controls](evidence/memory-2026-11-11/memory-controls.svg)
![Storage tradeoffs](evidence/memory-2026-11-11/memory-storage.svg)

## 理论模型：把 O(log N) 放在正确位置

设 D 为独立定义数、N 为实例数、H 为历史长度、Gᵢ 为定义的精确几何大小，W 为当前操作依赖前沿。

| 方案 | 当前常驻/临时状态 | 增长边界 |
| --- | --- | --- |
| Colocad alpha.8 | 最后使用后转移/释放中间 body；最终定义、交换表示、STEP 字符串和快照仍集中；session 整本 JSON | 独立定义通常 O(ΣGᵢ)，实例元数据 O(N)，head 查询 O(H)；连续追加总元数据 IO O(H²) |
| CadQuery | Workplane parent 链、OCCT 活动形状、装配和验证/导出临时结构 | 主动保留增长中的全部历史可能达 O(ΣG_revision)，最坏可二次；结构共享会缓解，不能把最坏界当实测。此实验独立件峰值近线性 |
| build123d | 当前形状/Builder、Compound 实例、OCCT 验证与导出结构 | 未人为保存每个版本时，不等于自动保留全部历史；独立几何和全量输出仍通常随规模增长 |
| 目标持久化 | 磁盘索引 + 固定页缓存 + 有界 W + 流式写入 | 索引查找 O(log N/H)，常驻缓存可有界；一般未压缩记录的磁盘下界 Ω(ΣGᵢ + N + H) |

源依据：[native executor](../crates/agent-cad/src/executor.rs)、[assembly](../crates/agent-cad/src/assembly.rs)、[STEP writer](../crates/agent-cad/src/step_assembly.rs)、[session](../src/session.mjs)、[CadQuery Workplane](https://cadquery.readthedocs.io/en/latest/workplane.html)、[build123d assemblies](https://build123d.readthedocs.io/en/latest/assemblies.html)。[OCCT Analyzer](https://github.com/Open-Cascade-SAS/OCCT/blob/V7_9_3/src/BRepCheck/BRepCheck_Analyzer.hxx)保留子形状检查结果；这里的复杂度是结合源码推导，不是对所有模型的数学性能认证。

合理目标是 `M_runtime + M_agent/harness + M_geometry(W) + M_operation_temporaries + B_index + B_io + M_charged_cache`。规则阵列可用 count/spacing 压缩描述，展开后的精确 B-rep、任意全局布尔和完整展示仍有几何信息下界。单个操作的最小工作集过大时，需要算法级改进、明确分解或资源拒绝；把整个系统称为 O(log N) 不成立。

## alpha.9 立项与未完成项

[SDD 0006](../specs/0006-bounded-memory-persistence.zh-CN.md)和 [ROADMAP](../ROADMAP.zh-CN.md)定义 M0–M5：本次基线/原型 → Rust catalog 与 SDK 迁移 → 字节预算/落盘/artifact sink → 大装配单 STEP 流式导出 → 查看器分页 → 原生双架构及完整 harness 验收。首批目标是 CAD 工作集 512 MiB、全 sandbox 2 GiB，分别验收；不是本次已经达到的全面承诺。

当前完成的是专题实验与原型，不修改已发布 alpha.8 的运行实现，也没有发布 alpha.9。正式验收还需要真实追加、kill/recover、跨平台索引、随机定义回读、改参/引用稳定性、单文件 STEP、真实整机与未见几何族。原始记录保留了两个后处理问题：direct native 目录映射修正，以及 macOS 对关闭 WAL catalog 的只读打开问题；验证器确认无待提交 WAL 后使用 immutable snapshot 读取，不能据此跳过未来 live catalog 的平台测试。

这是一轮可审计的 benchmark→定位→控制实验→原型验证；通用 G4/RSI 自动执行器仍未整体验收。候选保留完整失败、相同几何、资源和交付边界，不能靠删验证、减少产物或只挑成功样本提升分数。
'''
(a.docs/'memory-persistence-report.zh-CN.md').write_text(cn)
en='''# CAD memory limits and persistence study

2026-11-11 · Colocad alpha.8/native 0.5.1 · [完整中文报告](memory-persistence-report.zh-CN.md).

'''+links+'''

Tracking: [issue #1](https://github.com/Coloopx/colocad/issues/1) · [alpha.9 milestone](https://github.com/Coloopx/colocad/milestone/1).

## Findings

**The 2 GiB failures are reproducible, and Colocad is not uniformly lower-memory.** Working-set management, whole-assembly validation, STEP export and history indexing are separate problems. Indexed and chunked prototypes demonstrate bounded working sets for the tested contracts; they are not shipped alpha.9 functionality or proof of O(log N) total exact-CAD memory.

The common range (three repetitions, median peak/elapsed time):

'''+common+'''
![Memory scaling](evidence/memory-2026-11-11/memory-scaling.svg)

Shared definitions are currently a Colocad strength. Distinct parts and the large perforated solid exceed the Python backends here. Native validation accounts for roughly 85% of heavy-case time, but that bucket combines several checks and measurements. Historical hand/rail speedups remain scoped to their original workloads.

## Experimental boundary

Native Linux arm64, 1 vCPU, no network/swap, fresh processes and warmed host image pages. CadQuery 2.8.0 uses OCP 7.9.3.1.1; build123d 0.13.0 uses OCP-novtk 8.0.1.1.0. Separate environments, NumPy 2.5.4. Differences cannot all be attributed to language or the kernel. cgroup kernel peak includes the supervisor, descendants and charged file pages; anon/file/kernel/RSS are sampled at 20 ms. Installation, inference, resident agent/harness, viewer and independent verification are excluded. See [cgroup accounting](https://docs.kernel.org/admin-guide/cgroup-v2.html).

108 common-range runs were fixed and ordered before execution. Three-run bands are observed ranges, not confidence intervals. This is a shared host; metadata-fixture preparation briefly overlapped the main campaign outside the CAD container. Times are descriptive, unsuitable for narrow speed-regression thresholds. No timed CAD campaigns ran concurrently. Extreme/control arms mostly run once; 128-part chunked projects run three times, 512 parts once.

Bulk holes use a direct perforated-profile extrusion on the Python backends; serial Booleans are a separate family. Distinct parts differ in width and cannot be legitimately geometry-deduplicated. All successful CAD paths check analytic volume and deliver exact STEP. This is a controlled plate/assembly stress corpus, not an industrial-model population success rate.

'''+f"There are {j['scheduled']} scheduled observations, {j['executed']} executed and six unrun after predefined failure boundaries: 173 successes (including metadata lookups), ten cgroup OOMs, three V8 heap OOMs, four policy rejections and four timeouts. **{len(verified)} representative STEP deliveries** passed separate OCCT readback, every-solid validity/bounds/placement, count, analytic volume and every bore's axis/radius/center. All 15 chunk catalogs passed complete hash/range/integrity checks.\n"+'''
The verifier runs separately at 4 GiB and validates individual STEP-read solids to avoid the whole-compound temporary allocation under investigation. OCCT is an independent kernel for SolidCore output, but CadQuery readback is same-family OCCT plus analytic checks. Chunk geometry readback samples the first/middle/last chunk of each largest backend case; every other chunk retains build checks and digest verification. [Readback](evidence/memory-2026-11-11/verification.json), [catalog checks](evidence/memory-2026-11-11/catalog-verification.json), [environment](evidence/memory-2026-11-11/environment.json).

## The failure has more than one cause

Both Python baselines hit cgroup OOM with 4,096 unique parts and 4,096 repeated instances. Single solids with 16,384 holes and 1,024 serial cuts time out at 120 s; those are not completed memory measurements. The same 2,048-part task at different caps:

'''+boundary+'''
Fine-grained events prove that the 2,048-part failures happen during STEP export, after validity, mass and solid-count checks. The 4 GiB completion measures actual demand above 2 GiB. Earlier 4,096-part coarse logs only locate failure to the verification group.

A control validates definitions before known rigid placement, keeps aggregate mass/count checks and exports the full assembly STEP:

'''+control+'''
Repeated instances now complete and independently reopen, identifying whole-compound validity overhead for this fixture. Distinct definitions still OOM in STEP export. Compositional checks cannot be blindly substituted for arbitrary interacting Booleans or inter-part collision requirements.

Colocad rejects the extreme inputs at policy limits: 128 features/instances, 1,024 cutters, and a separate 32 MiB STEP budget. Each unique fixture part has two features. Refusal is not a successful low-memory large-model build.

## Persistence controls

Same Node 22.23.3 and SDK import, full JSON head lookup versus indexed SQLite lookup:

'''+history+'''
The 500k SDK failures are V8 heap aborts, not kernel OOM kills. These are synthetic metadata records seeded from one real accepted entry, not 500k CAD edits or a write-throughput result. The auxiliary Python/SQLite control stays near 13 MiB; the main indexing comparison uses the same Node runtime. The configured 2 MiB page cache is not a whole-process limit. [SQLite cache configuration](https://www.sqlite.org/pragma.html#pragma_cache_size) and [index planning](https://www.sqlite.org/queryplanner.html) support the design rationale.

Eight-part chunks, local process completion, hash/fsync/catalog commit and advisory release of completed file pages:

'''+chunks+'''
Anonymous working sets plateau from 32 to 512 parts for all three backends. This is a strategy, not a language-exclusive property. Output is a complete **indexed chunked project**, not a single merged assembly STEP. Latency and disk remain linear; Colocad's larger STEP output in this fixture is a separate export concern. Cross-chunk mates/collisions, arbitrary surfaces, whole-job resumption and product migration remain unaccepted.

The same direct native invocation, 64 parts (single-run ablation):

'''+native+'''
Checkpoint/query buffers explain only part of the peak. Local 12 MiB checkpoint and 64 MiB query-geometry caps do not bound all decoded topology, neutral exchange, output strings and runtime memory.

![Controls](evidence/memory-2026-11-11/memory-controls.svg)
![Disk and buffering](evidence/memory-2026-11-11/memory-storage.svg)

## Complexity and next-version scope

Colocad consumes intermediate bodies at last use but retains final definitions/exchange/output state; distinct geometry remains O(sum G). Instance metadata is O(N). Its JSON ledger has O(H) head-read/append work and O(H²) cumulative append IO. CadQuery's parent chain can retain growing versions (worst case O(sum G_revision), mitigated by topology sharing); build123d without user-retained versions does not automatically imply that same history cost. Both have size-dependent active geometry and verifier/exporter temporaries. These are source-based complexity arguments, not universal measured exponents. [Colocad executor](../crates/agent-cad/src/executor.rs), [session](../src/session.mjs), [CadQuery Workplane](https://cadquery.readthedocs.io/en/latest/workplane.html), [build123d assemblies](https://build123d.readthedocs.io/en/latest/assemblies.html), [OCCT analyzer](https://github.com/Open-Cascade-SAS/OCCT/blob/V7_9_3/src/BRepCheck/BRepCheck_Analyzer.hxx).

Target: runtime + agent/harness + active geometry frontier + operation temporaries + bounded index/IO caches + charged file cache. Disk remains proportional to total geometry, instances and history. O(log N/H) index lookup with a bounded cache is achievable; O(log N) total memory for arbitrary fully resident exact geometry/global operations is not. A procedural pattern can compress its description without removing the expanded topology/output lower bound.

[SDD 0006](../specs/0006-bounded-memory-persistence.md) and [ROADMAP](../ROADMAP.md) scope alpha.9: Rust catalog/SDK migration; byte-budgeted geometry and artifact sinks; streamed single-STEP export; lazy viewer; native multi-architecture and full-harness acceptance. CAD 512 MiB and entire sandbox 2 GiB are separately proposed gates. The existing published alpha.8 runtime is unchanged and alpha.9 is not released. Real appends, kill/recovery, stable refs, cross-platform catalogs, real products and held-out geometry remain required. Postprocessing diagnostics (native output path and host read-only WAL handling) are retained; immutable offline catalog verification does not substitute for future live-reader platform tests.

This is a directed benchmark/diagnosis/prototype iteration. It does not accept the whole G4/RSI automation program, and preserves failed controls rather than hiding them or changing output/verification requirements.
'''
(a.docs/'memory-persistence-report.md').write_text(en)
print('English and Chinese reports written; independent delivery count:',len(verified))
