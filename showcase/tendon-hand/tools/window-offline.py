# -*- coding: utf-8 -*-
"""Self-contained 10x player. Embed the movie, raw time series and sample STEP deliveries."""
import argparse
import csv
import io
import json
from pathlib import Path
import sys
import tempfile
import zipfile

HAND=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(HAND.parents[1]/'benchmarks/readme/replay'))
from offline import inline

ARMS=('optimized','build123d')


def csv_text(header,rows):
    text=io.StringIO()
    writer=csv.writer(text)
    writer.writerow(header)
    writer.writerows(rows)
    return text.getvalue()


def audit_zip(destination,run,evidence,model_path):
    data=json.loads(evidence.read_text());common=data['common_revisions']
    with zipfile.ZipFile(destination,'w',zipfile.ZIP_DEFLATED,compresslevel=6) as archive:
        archive.write(evidence,'window.json')
        archive.write(HAND/'model.mjs','model.mjs')
        for name in ['LICENSE','NOTICE.md','four-views.png']:
            archive.write(HAND/'reference'/name,'reference/'+name)
        for arm in ARMS:
            events=data['events'][arm]
            archive.writestr(arm+'/deliveries.csv',csv_text(
                ['revision_zero_based','start_s','end_s','job_s','in_window','step_sha256'],
                ([row[k] for k in ('revision','start_s','end_s','job_s','in_window','step_sha256')] for row in events)))
            archive.writestr(arm+'/memory.csv',csv_text(['elapsed_s','cgroup_bytes'],data['memory_samples_s_bytes'][arm]))
            last=data['summary'][arm]['count']-1
            for i in sorted({0,common-1,last}):
                row=events[i]
                model=json.loads(model_path.read_text())
                model['parameters'].update(guideRadius=row['guide_diameter_mm']/2,cableRadius=row['cable_radius_mm'])
                prefix=f'{arm}/revision-{i+1:02d}/'
                archive.writestr(prefix+'model.json',json.dumps(model,separators=(',',':')))
                archive.write(run/arm/f'{i:03d}.step',prefix+'model.step')


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('run',type=Path);parser.add_argument('model',type=Path)
    args=parser.parse_args();folder=HAND/'window';ev=HAND/'evidence/window-optimized.json'
    data=json.loads(ev.read_text());summary=data['summary'];c,b=map(summary.get,ARMS)
    with tempfile.TemporaryDirectory() as tmp:
        zip_path=Path(tmp)/'audit.zip';audit_zip(zip_path,args.run,ev,args.model)
        archive=inline(zip_path,'application/zip')
    rows=[('截止前交付',f"{c['count']} 份",f"{b['count']} 份"),
          ('首份完整 STEP',f"{c['first_s']:.3f} s",f"{b['first_s']:.3f} s"),
          (f"共同前 {data['common_revisions']} 份耗时中位数",f"{c['matched_median_s']:.3f} s",f"{b['matched_median_s']:.3f} s"),
          ('100 秒内采样最高内存',f"{c['sampled_peak_mib']:.1f} MiB",f"{b['sampled_peak_mib']:.1f} MiB")]
    table=''.join(f'<tr><th>{k}</th><td>{x}</td><td>{y}</td></tr>' for k,x,y in rows)
    stops=[('四视图',0),('布尔切除',1.6),('孔径改参',3.4),('三角网格',4.9),('五指装配',6.4),('100 秒读数',9.98)]
    controls=''.join(f'<button data-frame="{t}" type="button">{name}</button>' for name,t in stops)
    css="""*{box-sizing:border-box}body{margin:30px auto;padding:0 20px 50px;max-width:1100px;color:#173538;background:#f6f7f3;font:16px/1.65 system-ui,sans-serif}h1{font-size:clamp(28px,4vw,44px);line-height:1.2}video{width:100%;height:auto;border:1px solid #c7d8d4;border-radius:10px;display:block}nav{display:flex;flex-wrap:wrap;gap:9px;margin:13px 0 22px}button{color:#075f57;border:1px solid #8aa9a3;background:white;padding:7px 12px;border-radius:5px;font:inherit;cursor:pointer}table{width:100%;border-collapse:collapse;font-variant-numeric:tabular-nums}th,td{text-align:right;padding:8px;border-bottom:1px solid #c5d4cb}th:first-child{text-align:left}section{overflow-x:auto}a{color:#076d63}details{margin:16px 0}summary{cursor:pointer}"""
    doc=f'''<!doctype html><html lang="zh-CN"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>绳驱灵巧手 · 100 秒建模与装配对照</title><style>{css}</style>
<h1>100 秒能交付多少版完整灵巧手？</h1><p>优化后的 colocad 交付了 {c['count']} 版，build123d 交付了 {b['count']} 版。每版都包含 43 个实体。视频长 10 秒，曲线和计数按实际时间戳以 10× 回放。</p>
<video controls playsinline preload="auto" poster="{inline(folder/'finish.png','image/png')}" src="{inline(folder/'hand-window-zh-CN.mp4','video/mp4')}" aria-label="两端整手交付计数、逐时内存和建模镜头"></video><nav><button id="restart">从头播放</button>{controls}</nav>
<section><table><thead><tr><th>指标</th><th>colocad 优化内核</th><th>build123d 0.11.1</th></tr></thead><tbody>{table}</tbody></table></section>
<details><summary>如何计时、记录内存和检查 STEP</summary><p>相同的预装 Linux arm64 沙箱镜像，各 1 vCPU / 512 MiB，无网络。每端先预热一次，再独立顺序运行一个 100 秒窗口。每版从新 CAD 进程起步，从同一个参数队列修改孔径或绳索半径，建模、装配、执行各自验证、把 STEP 写入同一持久输出目录。只有截止前完成交付的版本计入产量。colocad 使用 alpha.2 SDK 加本次源码优化的 Rust 可执行文件。</p>
<p>约每 25 ms 采集一次整个容器的 memory.current，图上共用 0–512 MiB 标尺。“当前”是该时刻最近一次样本，“峰值”是该窗口至此采到的最高值。内存含工具子进程、采样器、临时文件及归属于本容器的缓存页。每个视频帧展示 0.4 秒区间结束时的读数。</p>
<p>计时后独立用 OCCT 回读所有计入的整手，核对 43 个实体各自的包围盒与体积、食指绳道实际孔径与绳索半径。慢的一端在窗口外补做用于几何比较的版本。四视图、布尔、改参、网格和装配镜头为独立的几何过程回放。窗口内两端成果采用同一输入，各自建模。</p></details>
<a href="{archive}" download="tendon-hand-100s-evidence.zip">下载原始时间戳、内存 CSV、样例 STEP 和对应的输入模型</a>
<script>const v=document.querySelector('video');document.querySelector('#restart').onclick=()=>{{v.currentTime=0;v.play();}};for(const b of document.querySelectorAll('[data-frame]'))b.onclick=()=>{{v.pause();v.currentTime=Number(b.dataset.frame)}};</script></html>'''
    (folder/'hand-window.html').write_text(doc)


if __name__=='__main__':main()
