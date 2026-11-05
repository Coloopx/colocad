"""Build the single-file player with the recorded STEP pairs and inputs embedded."""
import argparse
import json
from pathlib import Path
import sys
import tempfile
import zipfile

HAND = Path(__file__).resolve().parents[1]
REPO = HAND.parents[1]
sys.path.insert(0,str(REPO/'benchmarks/readme/replay'))
from offline import inline

p=argparse.ArgumentParser()
p.add_argument('finger_trial',type=Path)
p.add_argument('hand_trial',type=Path)
p.add_argument('models',type=Path)
args=p.parse_args()
media=HAND/'replay'
timeline=json.loads((media/'timeline.json').read_text())['chapters_s']
chapters=[('geometry','看指骨剖切'),('finger','指骨实录'),('assembly','看整手装配'),('cause','零件数量实验'),('hand','整手实录'),('result','五轮结果')]
buttons=''.join(f'<button type="button" data-frame="{timeline[key]+0.01:.3f}">{title}</button>' for key,title in chapters)
with tempfile.TemporaryDirectory() as tmp:
    bundle=Path(tmp)/'hand-recording.zip'
    with zipfile.ZipFile(bundle,'w',zipfile.ZIP_DEFLATED) as z:
        for k,d in [('finger',args.finger_trial),('hand',args.hand_trial)]:
            z.write(args.models/f'{k}.json',f'{k}/model.json')
            z.write(media/f'{k}-trace.json',f'{k}/trace.json')
            z.write(media/f'{k}-readback.json',f'{k}/readback.json')
            for arm in ('colocad','build123d'):
                z.write(d/f'{k}-{arm}.step',f'{k}/{arm}.step')
        for source in ('model.mjs','reference/four-views.png','reference/LICENSE','reference/NOTICE.md',
                       'evidence/reuse-summary.json','evidence/reuse-trials.json','evidence/profile.json','evidence/readback-reuse.json'):
            z.write(HAND/source,source)
    artifact=inline(bundle,'application/zip')

style="""*{box-sizing:border-box}body{max-width:1040px;margin:30px auto;padding:0 20px 40px;background:#f6f7f3;color:#173538;font:16px/1.65 system-ui,sans-serif}h1{font-size:clamp(25px,4vw,44px);line-height:1.2}video{width:100%;height:auto;display:block;border:1px solid #c7d8d4;border-radius:12px}nav{display:flex;flex-wrap:wrap;gap:9px;margin:14px 0 22px}button{color:#075f57;border:1px solid #8aa9a3;background:#fff;padding:7px 12px;border-radius:5px;font:inherit;cursor:pointer}a{color:#076d63}summary{cursor:pointer}"""
doc=f'''<!doctype html><html lang="zh-CN"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>绳驱灵巧手建模与性能实录</title><style>{style}</style><h1>从一根指骨到一只绳驱灵巧手</h1>
<p>相同输入、1 vCPU / 512 MiB，观察指骨与 43 实体装配的交付时间和内存。新内核对同一零件、同一朝向的平移实例复用几何检查结果。</p>
<video controls playsinline preload="auto" poster="{inline(media/'poster.png','image/png')}" src="{inline(media/'tendon-hand-zh-CN.mp4','video/mp4')}" aria-label="灵巧手从四视图建模到整手交付的实测回放"></video>
<nav><button type="button" id="restart">从头播放</button>{buttons}</nav>
<p>整手五轮中位数为旧版 3.929 秒、优化内核 2.239 秒、build123d 3.335 秒。优化内核最高峰值为 37.9 MiB，build123d 为 271.3 MiB。</p>
<details><summary>测量口径与建模边界</summary><p>实录使用 @coloopx/colocad 0.1.0-alpha.2 SDK，并指定本次源码构建的 Linux arm64 内核。对照使用 build123d 0.11.1。每一对单次实录独立预热后顺序执行，画面将起点对齐，按 1× 原速播放。内存约每 20 ms 采样一次，最后的峰值来自 memory.peak。阶段装配和 STEP 回读图在计时外生成。片尾统计来自单独五轮试验。绳路属于根据四视图补全的中立姿态设计。</p></details>
<p><a download="hand-recording.zip" href="{artifact}">下载两组配对 STEP、可编辑模型、原始读数与参考图授权</a></p>
<script>const v=document.querySelector('video');document.querySelector('#restart').onclick=()=>{{v.currentTime=0;v.play()}};for(const b of document.querySelectorAll('[data-frame]'))b.onclick=()=>{{v.pause();v.currentTime=Number(b.dataset.frame)}};</script></html>'''
(media/'hand-replay.html').write_text(doc)
