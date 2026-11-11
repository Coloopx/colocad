"""Write the README performance sections and linked tables from the verified candidate."""
import argparse,json,statistics
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument("directory",type=Path);args=p.parse_args()
repo=Path(__file__).resolve().parents[2];dest=args.directory.resolve();rel=dest.relative_to(repo).as_posix()
read=lambda name:json.loads((dest/name).read_text())
e=read("observations.json");it=read("iterations.json");controls=read("controls.json")
backends=("colocad","cadquery","build123d")
by={(r['suite'],r['case']):r['backends'] for r in e['summary']}
def med(s,c,b,k):
    assert by[s,c][b]['verified']==3
    return by[s,c][b][k]['median']
def wins(s,k):
    return sum(v['colocad'][k]['median']<min(v[b][k]['median'] for b in backends[1:]) for (suite,_),v in by.items() if suite==s)
assert len(e['observations'])==207 and all(r['verified'] for r in e['observations'])
w=e['windows'];assert all(w[b]['status']=='ok' for b in backends)
def values(s,c,k,digits=2):
    return ' / '.join(f'{med(s,c,b,k):,.{digits}f}' for b in backends)
def pair(v,k):
    return statistics.median(r[k] for r in it['unique64']['trials'] if r['n']==64 and r['variant']==v)
before,after=pair('before','seconds'),pair('after','seconds')
rev=e['engine_commit'][:7]
counts=' / '.join(str(w[b]['count']) for b in backends)
mem=' / '.join(f'{w[b]["peak_sample_mib"]:.1f}' for b in backends)
placed=[r['timing_ms'] for r in it['unique64']['trials'] if r['n']==64 and r['variant']=='after']
phase=lambda k:statistics.median(t[k] for t in placed)/1000
hist_before=it['alpha9_pair']['sessions']['before'];hist_after=it['final_session']
legacy=it['unique64']['initial_linux_phase_medians_s']
for zh in (False,True):
    lang='.zh-CN' if zh else ''
    doc=repo/'docs'/('benchmark-bounded-candidate'+lang+'.md')
    units='Colocad / CadQuery / build123d'
    links=f'[Single-file HTML](benchmarks/bounded-candidate/scorecard.html) · [Trials](benchmarks/bounded-candidate/observations.json) · [Paired experiments](benchmarks/bounded-candidate/iterations.json) · [Reproduction](../benchmarks/release/README.md)'
    title='三后端复测与 64 件装配耗时分析' if zh else 'Three-backend measurements and assembly validation costs'
    body=[f'# {title}',links]
    body+=[(f'源码 `{rev}` 与 CadQuery 2.8.0、build123d 0.13.0 在同一 Linux arm64 镜像中运行。主矩阵 2 GiB，产品用例与装配队列 512 MiB，每次限制 1 vCPU，无交换空间。每格为三个独立进程的中位数，顺序为 {units}。' if zh else f'Source `{rev}`, CadQuery 2.8.0 and build123d 0.13.0 ran in the same Linux arm64 image. Scaling tests allowed 2 GiB and product and queue tests allowed 512 MiB, each with 1 vCPU and no swap. Cells give medians across three fresh processes in the order {units}. Separate CPU sets share the development VM and filesystem.')]
    body+=['| '+('输入 | STEP 耗时，秒 | cgroup 峰值，MiB' if zh else 'Input | Time to STEP, s | Peak cgroup memory, MiB')+' |\n|---|---:|---:|\n'+'\n'.join(f'| {case} | {values(suite,case,"wall_s")} | {values(suite,case,"peak_mib",1)} |' for suite,case in by)]
    body+=['![Memory scaling across 12 inputs](benchmarks/bounded-candidate/memory-scaling.svg)']
    detail=(f'''64 块独立孔板的板宽逐件变化，每块各有 16 个孔。在原复测中 Colocad 的放置与校验中位数为 {legacy["colocad"]["validation"]:.2f} 秒。CadQuery 用含内环的轮廓一次拉伸，组装后核对有效性、体积和实体数，构造与校验分别为 {legacy["cadquery"]["geometry"]:.2f} 与 {legacy["cadquery"]["validated"]:.2f} 秒。build123d 对应为 {legacy["build123d"]["geometry"]:.2f} 与 {legacy["build123d"]["validated"]:.2f} 秒。三者的原生校验算法不同，最终 STEP 均按同一 OCCT 标准回读。

每件有 32 条孔口 UV 曲线。新版在全部有理二次 Bézier 跨段上建立速度和曲率半径下界，临界输入仍调用通用求解器。最终装配的高精度体积积分也核对了 64 次原解析体积公式。与前一候选版 `9f53a4e` 在 512 MiB 下交替配对三次，整次建模从 **{before:.2f} 秒降到 {after:.2f} 秒**。新版 Level‑1 与流形检查 {phase("level1_and_manifold"):.2f} 秒，体积 {phase("placed_volume"):.2f} 秒，包围盒及实体计数 {phase("bounds_and_census"):.2f} 秒。每次交付的 STEP 摘要相同。''' if zh else f'''The stock width varies across 64 distinct plates, each with 16 bores. At revision `9f53a4e`, Colocad spent {legacy["colocad"]["validation"]:.2f} s in assembly placement and validation. CadQuery extruded a profile with inner wires and checked validity, volume and solid count on the compound. Its construction and validation took {legacy["cadquery"]["geometry"]:.2f} s and {legacy["cadquery"]["validated"]:.2f} s. For build123d these took {legacy["build123d"]["geometry"]:.2f} s and {legacy["build123d"]["validated"]:.2f} s. The kernels use different native validation algorithms. The independent STEP checks used identical criteria.

The 32 UV trims on each perforated plate were expensive general curve searches. The change bounds speed and radius on each eligible rational quadratic Bézier span. Ambiguous inputs go through the solver. Each part's precise placed mass integral also checks the original analytic stock-volume oracle. Three alternating 512 MiB pairs reduced wall time from **{before:.2f} s to {after:.2f} s**. The new Level‑1 and manifold checks took {phase("level1_and_manifold"):.2f} s, volume {phase("placed_volume"):.2f} s, and bounds and census {phase("bounds_and_census"):.2f} s. All paired STEP digests matched.''')
    body += [detail,'![Phase breakdown for 64 independently dimensioned plates](benchmarks/bounded-candidate/unique-phases.svg)']
    body += [(f'100 秒灵巧手队列交付 {counts} 个经回读的 43 实体装配，25 ms 采样峰值为 {mem} MiB。每个后端一次窗口。' if zh else f'The 100 s edit queue delivered {counts} verified 43-solid hands. Peaks from 25 ms samples were {mem} MiB. One window per backend.')]
    body+=['![Recorded deliveries at 10×](benchmarks/bounded-candidate/hand-window.gif)','![Logical writes and retained files](benchmarks/bounded-candidate/build-io.svg)']
    io= ' / '.join(f'{med("fixtures","hand",b,"wchar")/1024:.0f}' for b in backends)
    body += [(f'完整手的逻辑写入为 {io} KiB，包含检查点、收据和输出。1 万次操作的会话重测中，alpha.9 与当前版的沙箱峰值分别为 {hist_before["peak_mib"]:.2f}、{hist_after["peak_mib"]:.2f} MiB，均逐条回读了 10,001 个记录。' if zh else f'The full-hand fixture issued {io} KiB of logical writes, including checkpoints, receipts and output. Across 10,000 operations, the session write rerun peaked at {hist_before["peak_mib"]:.2f} MiB for alpha.9 and {hist_after["peak_mib"]:.2f} MiB for this candidate. All 10,001 receipts were audited after reopening both projects.')]
    body += [('扩展压力测试使用同次复测中的第一版候选内核 `9f53a4e`，包含边界拒绝、OOM、超时及按规则停止的用例。单文件报告还包含六组 macOS STEP 渲染对照及各自的内存口径。' if zh else 'Extended stress results belong to the first candidate in this rerun, `9f53a4e`. Their records include resource rejections, OOMs, timeouts and planned boundary stops. The offline report also contains six separately measured macOS STEP shape comparisons.')]
    doc.write_text('\n\n'.join(body)+'\n')
    repo_doc=repo/('README.zh-CN.md' if zh else 'README.md')
    old=repo_doc.read_text()
    lo=old.index('\n',old.index('## 性能' if zh else '## Performance'))+1
    hi=old.index('\n## ',lo)
    text=(f'''Colocad `{rev}`、CadQuery 2.8.0、build123d 0.13.0 共交付 **207 份通过独立 STEP 回读的模型**。12 个规模点中，Colocad 有 **{wins("main","peak_mib")}/12 内存峰值最低、{wins("main","wall_s")}/12 耗时最短**。11 个产品及改参用例中分别为 **{wins("fixtures","peak_mib")}/11、{wins("fixtures","wall_s")}/11**。''' if zh else f'''Colocad `{rev}`, CadQuery 2.8.0 and build123d 0.13.0 delivered **207 independently verified STEP files**. Across the 12 scaling points Colocad used the **least peak memory in {wins("main","peak_mib")}/12** and produced STEP **fastest in {wins("main","wall_s")}/12**. Across the 11 product and edit fixtures, the corresponding counts were **{wins("fixtures","peak_mib")}/11 and {wins("fixtures","wall_s")}/11**.''')
    text+=f'\n\n![Time to verified STEP and peak memory]({rel}/scorecard.svg)\n\n'
    text+=(f'在 Colocad / CadQuery / build123d 的顺序下，128 个共享实例用时 **{values("main","repeated/128","wall_s")} 秒**。64 个独立零件的峰值为 **{values("main","unique/64","peak_mib",1)} MiB**，耗时 **{values("main","unique/64","wall_s")} 秒**。1,024 孔单板仍由 Python 后端在速度和内存上领先。' if zh else f'For 128 occurrences of one part, time to STEP was **{values("main","repeated/128","wall_s")} s** in Colocad / CadQuery / build123d order. For 64 independent plates, peak memory was **{values("main","unique/64","peak_mib",1)} MiB** and elapsed time was **{values("main","unique/64","wall_s")} s**. The Python backends also used less memory and time on the single 1,024-hole plate.')
    text+=f'\n\n![Memory across the full size series]({rel}/memory-scaling.svg)\n\n'
    text+=(f'100 秒内交付的完整 43 实体手为 **{counts}**，采样峰值 **{mem} MiB**。' if zh else f'In a 100 s edit queue, the backends delivered **{counts}** complete 43-solid hands with sampled peaks of **{mem} MiB**.')
    text+=f'\n\n![Verified hand deliveries and running memory peaks at 10×]({rel}/hand-window.gif)\n\n'
    text+=(f'[单文件 HTML，含全部结果和六组实体渲染]({rel}/scorecard.html) · [分段耗时、I/O 与实验范围](docs/{doc.name}) · [复现步骤](benchmarks/release/README.md)。' if zh else f'[Offline HTML with all results and six geometry comparisons]({rel}/scorecard.html) · [Phase timings, I/O and scope](docs/{doc.name}) · [Reproduce](benchmarks/release/README.md).')
    repo_doc.write_text(old[:lo]+'\n'+text+'\n'+old[hi:])
