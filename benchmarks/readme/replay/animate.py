"""Animate aligned wall clocks and sampled cgroup memory. CAD readback is outside timing."""
import argparse
from bisect import bisect_right
from functools import lru_cache
import json
import math
from pathlib import Path
import subprocess
import tempfile

import matplotlib
from PIL import Image, ImageDraw, ImageFont

WIDTH, HEIGHT, FPS = 1024, 640, 25
INTRO, HOLD = 1.20, 3
BG, INK, MUTED = '#f6f7f3', '#173538', '#496568'
RULE, PAPER = '#ccd9d5', '#ffffff'
COLORS = ['#05776d', '#4a627e']
TINTS = ['#e7f3ee', '#eaf0f6']


@lru_cache(maxsize=80)
def face(size, bold, chinese):
    if chinese:
        candidates = [('/System/Library/Fonts/Hiragino Sans GB.ttc', int(bold)),
                      ('/usr/share/fonts/opentype/noto/NotoSansCJK-Bold.ttc' if bold else
                       '/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc', 2)]
        for name, index in candidates:
            if Path(name).exists():
                return ImageFont.truetype(name,size,index=index)
        raise RuntimeError('Chinese output needs Hiragino Sans GB or Noto Sans CJK SC.')
    name = 'DejaVuSans-Bold.ttf' if bold else 'DejaVuSans.ttf'
    return ImageFont.truetype(str(Path(matplotlib.get_data_path())/'fonts/ttf'/name),size)


def label(draw,x,y,text,size=17,bold=False,color=INK,anchor='lt'):
    chinese=any('\u3000' <= c <= '\uffff' for c in text)
    draw.text((x,y),text,font=face(size,bold,chinese),fill=color,anchor=anchor)


def input_sketch(draw,center,scale,model,color):
    cx,cy=center
    def xy(u,v):
        return (cx+(u-45)*scale, cy-(v-22.5)*scale)
    features={f['id']:f for f in model['features']}
    outline=[xy(*point) for point in features['Mount']['profile']['points']]
    draw.polygon(outline,fill='#eef4f2',outline=color,width=2)
    radius=model['parameters']['pocketRadius']*scale
    port=features['Port']['profile']
    for instance in range(model['parameters']['instances']):
        x,y=xy(port['center'][0]+instance*model['parameters']['pitch'], port['center'][1])
        draw.ellipse((x-radius,y-radius,x+radius,y+radius),outline=color,width=2)
    profile=features['Channel']['profile']
    x,y=profile['center']
    w,h=profile['size']
    draw.rectangle((*xy(x-w/2,y+h/2),*xy(x+w/2,y-h/2)),outline=color,width=2)


class Replay:
    def __init__(self, trial, benchmark, model, chinese):
        self.zh=chinese
        self.runs=json.loads((trial/'trace.json').read_text())['recorded']
        self.historical=json.loads(benchmark.read_text())['manifold']
        self.model=json.loads(model.read_text())
        self.images={row['backend']:Image.open(trial/(row['backend']+'.png')).convert('RGB') for row in self.runs}
        self.last=max(row['wall_ms'] for row in self.runs)
        self.extent=math.ceil(self.last/500)*500

    def l(self,en,cn):
        return cn if self.zh else en

    def canvas(self,title,sub):
        image=Image.new('RGB',(WIDTH,HEIGHT),BG)
        draw=ImageDraw.Draw(image)
        label(draw,32,23,'colocad',21,True)
        label(draw,134,28,self.l('CAD PERFORMANCE','CAD 性能实验'),14,False,MUTED)
        label(draw,992,26,'Linux arm64    /    1 vCPU    /    512 MiB',15,False,MUTED,'rt')
        draw.line((32,55,992,55),fill=RULE)
        label(draw,31,73,title,34,True)
        label(draw,33,126,sub,17,False,MUTED)
        return image,draw

    def header(self):
        image,draw=self.canvas(self.l('One manifold, two fresh tool calls','同一块歧管，两次独立建模'),
            self.l('Time to a written STEP, with the same CPU and memory budget.','比较从进程启动到写完 STEP 的时间，以及整次调用的内存峰值。'))
        input_sketch(draw,(263,335),4.20,self.model,'#457a76')
        label(draw,96,453,'90 × 45 × 12 mm',20,True)
        label(draw,96,489,self.l('Bottom view of the input design','输入设计的底视图'),17,False,MUTED)
        draw.line((525,182,525,548),fill=RULE)
        label(draw,558,218,self.l('The same input','相同的建模输入'),26,True)
        label(draw,558,272,self.l('4 blind ports  •  milled channel','4 个盲孔、1 条浅槽'),20)
        label(draw,558,323,self.l('Each tool builds its own solid.','两端各自生成实体并导出 STEP。'),19)
        label(draw,558,384,self.l('Playback runs at actual speed.','接下来按 1× 原速回放。'),21,True,COLORS[0])
        label(draw,558,434,self.l('Two recorded launches aligned at t = 0.','两次实录按各自的启动时刻对齐。'),16,False,MUTED)
        return image

    def race(self,ms):
        image,d=self.canvas(self.l('Time to first STEP','第一份 STEP 什么时候交付？'),
            self.l('Same inputs. Independent runs aligned at launch. The clock runs at 1×.','同一输入，两个独立运行的启动时刻对齐，回放速度 1×。'))
        for i,row in enumerate(self.runs):
            x=32+490*i; name=row['backend']; color=COLORS[i]
            d.rounded_rectangle((x,163,x+470,372),radius=13,fill=PAPER,outline=RULE)
            label(d,x+18,181,name,22,True,color)
            done=ms>=row['wall_ms']
            elapsed=min(ms,row['wall_ms'])/1000
            label(d,x+18,233,f'{elapsed:.3f} s',28,True)
            label(d,x+18,281,self.l('STEP saved','STEP 已写出') if done else self.l('Building…','正在建模…'),19,True,color if done else MUTED)
            if done:
                thumb=self.images[name].resize((232,105),Image.Resampling.LANCZOS)
                image.paste(thumb,(x+224,218))
                caption=self.l('STEP readback','STEP 回读图')
            else:
                input_sketch(d,(x+338,265),1.67,self.model,'#91a6a5')
                caption=self.l('Input sketch','输入草图')
            label(d,x+343,333,caption,14,False,MUTED,'mt')
        label(d,34,398,self.l('Sandbox memory','沙箱内存'),22,True)
        label(d,299,404,self.l('Whole cgroup  ·  target sample interval 20 ms','整个 cgroup，每约 20 ms 采样一次'),15,False,MUTED)
        x0,x1,y0,y1=88,682,570,448
        def px(t):return x0+(x1-x0)*t/self.extent
        def py(b):return y0-(y0-y1)*b/(512*2**20)
        for mib in [0,256,512]:
            y=py(mib*2**20);d.line((x0,y,x1,y),fill=RULE)
            label(d,x0-9,y,str(mib),13,False,MUTED,'rm')
        for t in range(0,self.extent+1,500):
            label(d,px(t),579,f'{t/1000:g}s',13,False,MUTED,'mt')
        d.line((px(ms),y1,px(ms),y0),fill='#738682',width=1)
        for i,row in enumerate(self.runs):
            points=row['samples_ms_bytes'];tmax=min(ms,row['wall_ms'])
            limit=bisect_right([p[0] for p in points],tmax)
            last_t,last_bytes=points[0];coords=[(px(last_t),py(last_bytes))]
            for t,b in points[1:limit]:
                coords.extend([(px(t),py(last_bytes)),(px(t),py(b))]);last_bytes=b;last_t=t
            end=min(tmax,points[-1][0]) if ms>=row['wall_ms'] else tmax
            coords.append((px(end),py(last_bytes)));d.line(coords,fill=COLORS[i],width=3)
            endx,endy=coords[-1];d.ellipse((endx-3,endy-3,endx+3,endy+3),fill=COLORS[i])
            y=446+63*i
            d.rounded_rectangle((712,y,990,y+53),radius=6,fill=TINTS[i])
            label(d,723,y+7,row['backend'],13,True,COLORS[i])
            done=ms>=row['wall_ms']
            value=row['peak_bytes'] if done else last_bytes
            label(d,723,y+29,(self.l('peak','峰值') if done else self.l('sample','采样'))+f'  {value/2**20:.1f} MiB',16,True)
        label(d,33,615,self.l('Memory axis: 0–512 MiB. Peak comes from Linux memory.peak. Output images are rendered after measurement.',
                              '内存纵轴为 0–512 MiB，峰值取自 Linux memory.peak。模型图片在计时结束后生成。'),14,False,MUTED)
        return image

    def summary(self):
        image,d=self.canvas(self.l('Both STEP exports passed the geometry check','两份 STEP 都通过了几何核对'),
            self.l('Each STEP reopened as one valid solid with 27 faces and 54 edges.','两端回读均为 1 个有效实体、27 个面、54 条边。'))
        c,b=self.runs
        ratio=b['wall_ms']/c['wall_ms'];share=c['peak_bytes']/b['peak_bytes']
        for x,title,num,desc in [
          (34,self.l('Delivery speedup','交付速度'),f'{ratio:.1f}×',
           f"{c['wall_ms']:.0f} ms  /  {b['wall_ms']:,.0f} ms"),
          (528,self.l('Share of build123d peak','峰值内存占 build123d'),f'{share:.1%}',
           f"{c['peak_bytes']/2**20:.1f} MiB  /  {b['peak_bytes']/2**20:.1f} MiB")]:
            d.rounded_rectangle((x,184,x+460,376),radius=12,fill=PAPER,outline=RULE)
            label(d,x+20,206,title,20,True)
            label(d,x+20,247,num,56,True,COLORS[0])
            label(d,x+22,334,desc,22)
        label(d,35,411,self.l('Recorded pair above   •   colocad / build123d','上方为本次实录，数值顺序为 colocad / build123d'),17,False,MUTED)
        d.line((34,460,990,460),fill=RULE)
        hist=self.historical
        label(d,35,485,self.l('Published five-run benchmark','另一次五轮基准'),23,True)
        label(d,35,534,self.l(f"{hist['speedup']:.1f}× faster to STEP   ·   {hist['memory_fraction']:.1%} of peak memory",
                                f"交付快 {hist['speedup']:.1f} 倍   ·   峰值内存占 {hist['memory_fraction']:.1%}"),25,True,COLORS[0])
        label(d,35,602,self.l('Linux arm64, 1 vCPU, 512 MiB. Time = median, memory = highest peak across five new processes.',
                                'Linux arm64，1 vCPU / 512 MiB。五个新进程的耗时取中位数，内存取最高峰值。'),15,False,MUTED)
        return image


def encode(replay,stem):
    race_frames=math.ceil(replay.last*FPS/1000)+1
    intro=replay.header();summary=replay.summary()
    with subprocess.Popen(['ffmpeg','-loglevel','error','-y','-f','rawvideo','-pix_fmt','rgb24',
                      '-s',f'{WIDTH}x{HEIGHT}','-r',str(FPS),'-i','-','-an','-c:v','libx264',
                      '-threads','2','-crf','19','-pix_fmt','yuv420p','-movflags','+faststart',str(stem.with_suffix('.mp4'))],stdin=subprocess.PIPE) as proc:
        def send(image,count=1):
            data=image.tobytes()
            for _ in range(count):proc.stdin.write(data)
        send(intro,round(INTRO*FPS))
        for frame in range(race_frames):
            send(replay.race(min(frame*1000/FPS,replay.last)))
        send(summary,round(HOLD*FPS))
        proc.stdin.close()
        if proc.wait(): raise RuntimeError('MP4 encoding failed')
    # One global palette keeps labels and the memory curves stable between frames.
    with tempfile.TemporaryDirectory() as temp:
        palette=str(Path(temp)/'palette.png')
        mp4=str(stem.with_suffix('.mp4'))
        subprocess.run(['ffmpeg','-loglevel','error','-y','-i',mp4,'-vf','palettegen=stats_mode=full',palette],check=True)
        subprocess.run(['ffmpeg','-loglevel','error','-y','-i',mp4,'-i',palette,'-filter_complex',
                        'paletteuse=dither=bayer:bayer_scale=4:diff_mode=rectangle','-loop','0',str(stem.with_suffix('.gif'))],check=True)
    return {'width':WIDTH,'height':HEIGHT,'fps':FPS,'intro_s':INTRO,'race_frames':race_frames,'hold_s':HOLD}


if __name__=='__main__':
    p=argparse.ArgumentParser()
    p.add_argument('trial',type=Path);p.add_argument('out',type=Path)
    a=p.parse_args();a.out.mkdir(exist_ok=True,parents=True)
    root=Path(__file__).resolve().parents[3]
    for zh,lang in [(True,'zh-CN'),(False,'en')]:
        replay=Replay(a.trial,root/'docs/benchmarks/scorecard.json',root/'examples/manifold.json',zh)
        meta=encode(replay,a.out/f'manifold-{lang}')
        if zh:
            replay.race(replay.last).save(a.out/'replay-poster.png')
            (a.out/'playback.json').write_text(json.dumps(meta,indent=2)+'\n')
