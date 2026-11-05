"""Self-contained Chinese scorecard. No network, JavaScript, video or CDN."""
import base64,html,json
from pathlib import Path
root=Path(__file__).resolve().parents[2];d=root/'docs/benchmarks';h=root/'showcase/tendon-hand'
def uri(path,mime):
    return 'data:'+mime+';base64,'+base64.b64encode(path.read_bytes()).decode()
def download(path,mime,name):
    return f'<a href="{uri(path,mime)}" download="{path.name}">{name}</a>'
sc=json.loads((d/'scorecard.json').read_text());rows=[]
for k,name in [('sensor-rail','三槽导轨'),('manifold','四盲孔歧管'),('finger','绳驱手近节指骨'),('hand','43 实体整手')]:
    a,b=[sc[k][x] for x in ('colocad','build123d')]
    rows.append(f'<tr><th>{name}</th><td>{a[0]:,.0f} / {b[0]:,.0f} ms</td><td>{sc[k]["speedup"]:.2f}×</td><td>{a[1]:.1f} / {b[1]:.1f} MiB</td><td>{sc[k]["memory_fraction"]:.1%}</td></tr>')
p='''<!doctype html><html lang="zh-CN"><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>colocad · Time to STEP</title>
<style>body{max-width:1000px;margin:35px auto;padding:0 20px 50px;font:16px/1.7 system-ui,sans-serif;color:#163d41;background:#f6f8f5}h1{font-size:clamp(28px,6vw,48px);line-height:1.15}h2{margin-top:36px}img{width:100%;height:auto}th,td{text-align:right;padding:8px;border-bottom:1px solid #c5d4cb;white-space:nowrap}th:first-child{text-align:left}table{width:100%;border-collapse:collapse}section{overflow-x:auto}a{color:#007366}small{color:#46716c}</style>
<small>COLOCAD / 0.1.0-alpha.2 / 交付性能</small><h1>Agent 等到首份 STEP，需要多久？</h1>
<p>同一台 Linux arm64 机器，各给 1 vCPU / 512 MiB。导轨、歧管的端到端速度比是 <b>22.0×、12.9×</b>，峰值内存分别是 build123d 的 <b>7.7%、8.0%</b>。整手装配则是 build123d 更快。</p>
<img src="CHART" alt="各工作负载的时间与内存条形图"><section><table><thead><tr><th>交付物</th><th>colocad / build123d</th><th>交付速度比</th><th>实测峰值内存</th><th>内存占对方</th></tr></thead><tbody>ROWS</tbody></table></section>
<h2>读数的含义</h2><p>各后端从相同参数独立建模。每组预热后交替跑五次，每次起新的工具进程和 cgroup。用时取中位数，从启动至正常退出（已写出 STEP），包含导入、建模、各自验证和导出；内存取五轮实测 cgroup 峰值最大值，含子进程、计入本组的文件页和临时输出。安装、容器启动、渲染、推理及常驻 agent 不计。常驻 Python 解释器可以摊薄导入成本。</p>
<h2>整手是反例</h2><p>另一组独立的 100 秒连续改参实验里，colocad 得到 24 份整手 STEP，build123d 得到 30 份，每份都包含 43 个有效实体。相同前 24 个输入，交付中位数分别是 3.946 和 3.058 秒。该实验与上表独立，不混算。</p>
<details><summary>查看两端独立生成的导轨和歧管</summary><img src="PAIR" alt="两端 STEP 独立回读对照"></details>
<h2>下载实验依据</h2><p>本页中的图与以下文件都已内嵌，可离线保存。</p><ul>FILES</ul></html>'''
files=[(d/'runs.json','application/json','导轨与歧管：逐轮读数'),(h/'evidence/scope.json','application/json','指骨与整手：逐轮读数'),(h/'evidence/window.json','application/json','100 秒计数、时间戳与几何核对'),(h/'hand-study.zip','application/zip','四视图、可编辑模型、两端 STEP 和实测记录')]
for k,v in [('CHART',uri(d/'time-to-step.zh-CN.svg','image/svg+xml')),('PAIR',uri(d/'step-comparison.png','image/png')),('ROWS',''.join(rows)),('FILES',''.join('<li>'+download(*j)+'</li>' for j in files))]:p=p.replace(k,v,1)
(d/'time-to-step.html').write_text(p)
