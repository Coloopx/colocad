"""Bundle the playable recording, the measured files and the raw evidence into one HTML."""
import argparse
import base64
import html
import json
import math
from pathlib import Path


def inline(path, mime):
    return 'data:'+mime+';base64,'+base64.b64encode(path.read_bytes()).decode('ascii')


def bundle(trial, media):
    trace=json.loads((media/'trace.json').read_text())
    clock=json.loads((media/'playback.json').read_text())
    runs=trace['recorded']
    fps=clock['fps']
    def chapter(row):
        frame=math.ceil(row['wall_ms']*fps/1000)
        return clock['intro_s']+(frame+0.5)/fps
    downloads=[(trial/f"manifold-{row['backend']}.step",'application/octet-stream',row['backend']+' STEP') for row in runs]
    downloads += [(media/'trace.json','application/json','时间戳与内存采样'),
                  (media/'readback.json','application/json','几何回读结果')]
    links=''.join(f'<a href="{inline(path,mime)}" download="{path.name}">{html.escape(label)}</a>' for path,mime,label in downloads)
    buttons=''.join(f'<button type="button" data-frame="{chapter(row):.4f}">定位 {row["backend"]} 交付</button>' for row in runs)
    c,b=runs
    ratio=b['wall_ms']/c['wall_ms']
    share=c['peak_bytes']/b['peak_bytes']
    css="""*{box-sizing:border-box}body{max-width:1040px;margin:30px auto;padding:0 20px 34px;background:#f6f7f3;color:#173538;font:16px/1.65 system-ui,sans-serif}h1{font-size:clamp(25px,4vw,42px);line-height:1.18;margin:12px 0}video{width:100%;height:auto;display:block;border:1px solid #c7d8d4;border-radius:13px;background:#eef4f1}nav{display:flex;flex-wrap:wrap;gap:10px;margin:15px 0 20px}button{color:#075f57;border:1px solid #8aa9a3;background:#fff;padding:8px 14px;border-radius:5px;font:inherit;cursor:pointer}button:hover{background:#e7f1ed}h2{font-size:21px;margin-top:25px}a{color:#076d63;margin-right:18px;display:inline-block}p{max-width:940px}summary{cursor:pointer}"""
    doc=f'''<!doctype html><html lang="zh-CN"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>colocad 与 build123d · 交付同一块歧管</title><style>{css}</style>
<h1>交付同一块歧管，要等多久？</h1>
<p>本次实录里，colocad 在 {c['wall_ms']:.0f} ms 交付 STEP，build123d 用了 {b['wall_ms']:,.0f} ms。交付速度比为 {ratio:.1f}×，峰值内存占对方的 {share:.1%}。</p>
<video controls playsinline preload="auto" poster="{inline(media/'replay-poster.png','image/png')}" src="{inline(media/'manifold-zh-CN.mp4','video/mp4')}" aria-label="歧管交付与内存实测动画"></video>
<nav aria-label="回放控制"><button type="button" id="restart">从头播放</button>{buttons}</nav>
<details><summary>这段动画测了什么？</summary><p>Linux arm64，1 vCPU，512 MiB，使用 npm 包 colocad 0.1.0-alpha.2 与 build123d 0.11.1。两端按同一组参数独立建模，预热后各取第一个新进程。两次调用依次运行，画面将启动时刻对齐，计时包含进程启动、导入、几何构建、验证及写出 STEP。曲线使用约 20 ms 间隔采集的 cgroup memory.current，峰值来自 memory.peak。内存计入了容器中的采样器与被测进程。成品图在计时结束后用同一相机从各自的 STEP 渲染。</p><p>片尾另列已有五轮基准，交付时间取五次中位数，内存取五次峰值的最高值。这里展示的是单件歧管。</p></details>
<h2>保存实测文件</h2><p>{links}</p>
<script>
const video=document.querySelector('video');
document.querySelector('#restart').onclick=()=>{{video.currentTime=0;video.play();}};
for(const button of document.querySelectorAll('[data-frame]')) {{
 button.onclick=()=>{{video.pause();video.currentTime=Number(button.dataset.frame);}};
}}
</script></html>'''
    (media/'manifold-replay.html').write_text(doc)


if __name__=='__main__':
    parser=argparse.ArgumentParser()
    parser.add_argument('trial',type=Path)
    parser.add_argument('media',type=Path)
    args=parser.parse_args()
    bundle(args.trial,args.media)
