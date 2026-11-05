"""Package the same figures, observations and complete STEP pairs as one offline HTML."""
import argparse
import base64
import json
import statistics
from html import escape
from pathlib import Path

p=argparse.ArgumentParser()
p.add_argument('trials',type=Path)
p.add_argument('docs',type=Path,help='directory with figures, runs.json, step-checks.json and preview-runs.json')
a=p.parse_args()
def uri(path,kind):
    return f'data:{kind};base64,'+base64.b64encode(path.read_bytes()).decode()
def download(path,kind,label):
    return f'<a download="{escape(path.name)}" href="{uri(path,kind)}">{escape(label)}</a>'
runs=json.loads((a.docs/'runs.json').read_text())['runs']
checks=json.loads((a.docs/'step-checks.json').read_text())
views=json.loads((a.docs/'preview-runs.json').read_text())
rows=[]; geometry=[]; previews=[]; files=[]
for case,label in [('sensor-rail','传感器导轨 · Sensor rail'),('manifold','四盲孔歧管 · Manifold')]:
    for b in ['colocad','build123d']:
        rec=[r for r in runs if r['rep']>0 and r['backend']==b and r['case']==case]
        rows.append(f'<tr><th>{label}</th><td>{b}</td><td>{statistics.median(r["wall_ms"] for r in rec):,.1f} ms</td><td>{max(r["peak_bytes"] for r in rec)/2**20:.1f} MiB</td><td>{statistics.median(r["geometry_ms"] for r in rec):.1f} ms</td></tr>')
        g=checks[case][b]
        geometry.append(f'<tr><th>{label}</th><td>{b}</td><td>{g["solids"]}</td><td>{g["faces"]} / {g["edges"]}</td><td>{g["volume_mm3"]:,.6f}</td></tr>')
        files.append(download(a.trials/f'{case}-{b}.step','application/octet-stream',f'{label} / {b} STEP'))
    md={m:statistics.median(r['wall_ms'] for r in views if r['case']==case and r['rep'] and r['mode']==m) for m in ('cache','rebuild')}
    previews.append(f'<tr><th>{label}</th><td>{md["rebuild"]:.1f} ms</td><td>{md["cache"]:.1f} ms</td><td>{md["rebuild"]/md["cache"]:.1f}×</td></tr>')
def table(cols,values):
    return '<div class="scroll"><table><thead><tr>'+''.join('<th>'+s+'</th>' for s in cols)+'</tr></thead><tbody>'+''.join(values)+'</tbody></table></div>'
fig=uri(a.docs/'step-comparison.png','image/png')
chart=uri(a.docs/'sandbox-performance.svg','image/svg+xml')
receipts=[download(a.docs/f,'application/json',l) for f,l in [('runs.json','全部构建读数'),('step-checks.json','几何核对'),('preview-runs.json','预览读数与哈希')]]
page='''<!doctype html><html lang="zh-CN"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>colocad / build123d · Linux sandbox benchmark</title>
<style>*{box-sizing:border-box}body{margin:0 auto;padding:40px 24px 60px;max-width:1000px;color:#18383b;font:16px/1.65 system-ui,sans-serif;background:#fafbf8}header{border-bottom:2px solid #397473;padding-bottom:20px}h1{font-size:clamp(30px,6vw,60px);letter-spacing:-.04em;line-height:1.05;margin:12px 0}h2{font-size:24px;margin-top:36px}small{font-weight:bold;letter-spacing:.06em;color:#477476}img{width:100%;height:auto;display:block;margin:12px 0;border-radius:8px}table{width:100%;border-collapse:collapse;font-variant-numeric:tabular-nums;white-space:nowrap}td,th{padding:9px;text-align:right;border-bottom:1px solid #cddbd7}th:first-child,td:first-child{text-align:left}.scroll{overflow-x:auto}a{color:#00685f}li{margin:6px 0}.caption{color:#516666}section{margin-top:24px}</style>
<header><small>COLOCAD / BENCHMARK / 0.1.0-alpha.2</small><h1>小沙箱里的 CAD 对照</h1><p>Linux arm64 · 1 vCPU · 512 MiB · 两款零件 · 20 次正式构建</p></header>
<h2>同参数，两份独立的 STEP</h2><img src="FIGURE" alt="左列 colocad，右列 build123d；导轨与歧管的同相机对比。"><p>两端从相同的建模指令独立生成实体。四份导出文件在 OCCT 中都有效，两件的双向差集体积各为 0 mm³（布尔容差 0.000001 mm）。图像来自同一套 OCCT 三角化设置和正交相机。</p>GEOMETRY
<h2>启动到交付 STEP</h2><img src="CHART" alt="两端的时间和内存柱状图，精确数值在下表。">TIMINGS
<p>测量 npm 发布物中的 Node CLI 和 Rust 二进制，对照 build123d 0.11.1。每件每端单独预热一次，再交替执行五轮；用时取中位数，内存取五次 cgroup 峰值最大值。新进程含导入、验证和 STEP 导出，不含安装、容器启动、渲染或 agent 推理。常驻 Python 能摊薄导入成本；歧管的纯几何阶段 build123d 更快。</p>
<h2>换视角时，复用网格能省多少？</h2>PREVIEWS<p>这是 colocad 内部两条路径的对照。SDK 已加载，同一零件从等轴视图改为俯视图。每件预热一对，再测五对；每对输出的 PNG、GLB、STL 文件哈希一致。</p>
<h2>原始文件</h2><p class="caption">图、STEP、JSON 均内嵌在本页，断网也可下载。</p><ul>DOWNLOADS</ul></html>'''
for k,v in [('FIGURE',fig),('CHART',chart),('GEOMETRY',table(['零件','后端','实体数','面 / 边','体积 mm³'],geometry)),('TIMINGS',table(['零件','后端','总时间','峰值内存','仅几何'],rows)),('PREVIEWS',table(['零件','重建并出图','复用网格','加速比'],previews)),('DOWNLOADS',''.join('<li>'+f+'</li>' for f in files+receipts))]:
    page=page.replace(k,v,1)
(a.docs/'comparison.html').write_text(page)
