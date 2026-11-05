"""Ten seconds of synchronous readouts from two separately recorded 100-second windows."""
import argparse
from bisect import bisect_right
from functools import lru_cache
from itertools import accumulate
import json
from pathlib import Path
import subprocess
import sys
import tempfile

from PIL import Image, ImageDraw, ImageOps

HAND = Path(__file__).resolve().parents[1]
sys.path.insert(0,str(HAND.parents[1]/'benchmarks/readme/replay'))
from animate import label, BG, INK, MUTED, RULE, COLORS

SIZE=(1080,720)
FPS=25
ARMS=('optimized','build123d')
STAGES=('wrist','palm','proximal','middle','tips','thumb','drive')

@lru_cache(maxsize=48)
def picture(path):
    return Image.open(path).convert('RGB')


def fit(canvas, source, x, y, width, height):
    fitted=ImageOps.contain(source,(width,height),Image.Resampling.LANCZOS)
    canvas.paste(fitted,(int(x+(width-fitted.width)/2),int(y+(height-fitted.height)/2)))


class Track:
    def __init__(self,data,name):
        self.events=[e for e in data['events'][name] if e['in_window']]
        self.handoffs=[e['end_s'] for e in self.events]
        self.times,self.mib=zip(*[(t,mem/2**20) for t,mem in data['memory_samples_s_bytes'][name]])
        self.high=list(accumulate(self.mib,max))


class Film:
    def __init__(self,data,assets,zh):
        self.assets=assets
        self.zh=zh
        self.tracks=[Track(data,k) for k in ARMS]
        self.checked=data['verified_pairs']
        self.loops=json.loads((assets/'loops.json').read_text())
        self.stage_images={stage:picture(assets/(stage+'.png')) for stage in STAGES}

    def l(self,en,cn):return cn if self.zh else en

    def storyboard(self,im,d,t):
        # The choices below are editorial cuts. The clocks, counts, ticks and memory
        # always use t in both recordings, independently of the shot on the left.
        selftext=self.l('CAD detail replay','几何镜头回放')
        label(d,32,121,selftext,15,True,MUTED)
        if t<15:
            title=self.l('Four views of the reference','灵巧手的四张参考视图')
            for j,k in enumerate(('front','side','end','oblique')):
                ref=picture(HAND/'reference'/(k+'.png'))
                if k=='front':
                    ref=ref.copy()
                    ImageDraw.Draw(ref).rectangle((235,133,280,254),outline='#b36a30',width=5)
                x=38+j%2*225;y=192+j//2*104
                fit(im,ref,x,y,210,105)
            note=self.l('Gold marks the index proximal link.','金框标出食指近节。')
        elif t<33:
            title=self.l('Cut the finger link','在指骨上切出叉口和绳道')
            progressive=['ProximalBlank','ProximalFork','cutter','cutaway']
            idx=min(3,int((t-15)/4.5))
            fit(im,picture(self.assets/(progressive[idx]+'.png')),42,178,417,228)
            note=self.l('Gold is the subtraction tool.','金色为布尔切除使用的刀体。')
        elif t<48:
            title=self.l('Edit the guide bore','改参后回读绳道截面')
            for i,(name,col,dia) in enumerate([('ProximalCableGuide',COLORS[0],'2.900'),('guide-edit','#ab6229','3.300')]):
                x=141+i*224;y=272
                for loop in self.loops[name]:
                    points=[(x+u*8.6,y-v*8.6) for u,v in loop]
                    d.line(points+[points[0]],fill=col,width=3)
                label(d,x,369,'Ø'+dia+' mm',17,True,col,'mt')
            note=self.l('OCCT cross-sections at z = 23 mm.','同一 z = 23 mm 平面的实际 STEP 截面。')
        elif t<63:
            title=self.l('Mesh once, place again','三角化与实例复用')
            fit(im,picture(self.assets/'wire.png'),32,182,210,216)
            fit(im,picture(self.assets/'reuse.png'),243,178,244,222)
            note=self.l('20 mesh definitions, 43 placements.','20 份显示网格，共 43 个实例。')
        else:
            title=self.l('Assemble the five fingers','装配腕座、五指和绳索')
            j=min(6,int((t-63)/4.6))
            fit(im,self.stage_images[STAGES[j]],38,176,286,236)
            names=[self.l('Wrist','腕座'),self.l('Palm','手掌'),self.l('Proximal links','近节'),self.l('Middle links','中节'),self.l('Fingertips','指尖'),self.l('Thumb','拇指'),self.l('Pins / cords','销轴、绳索')]
            for i,n in enumerate(names):
                label(d,318,195+27*i,n,16,i==j,COLORS[0] if i<=j else MUTED)
            note=self.l('Actual STEP milestones. Neutral pose.','每个阶段的 STEP 回读图，中立姿态。')
        label(d,32,150,title,23,True)
        label(d,32,411,note,15,False,MUTED)

    def scoreboard(self,d,i,t):
        track=self.tracks[i]; col=COLORS[i];x=544;y=130+i*144
        n=bisect_right(track.handoffs,t)
        d.rounded_rectangle((x,y,x+506,y+132),radius=11,fill='#ffffff',outline=RULE)
        label(d,x+15,y+13,'colocad' if i==0 else 'build123d',21,True,col)
        label(d,x+19,y+47,str(n).zfill(2),44,True,col)
        label(d,x+104,y+62,self.l('STEPs delivered','份整手 STEP'),18,True)
        if n:
            event=track.events[n-1]
            diameter=self.checked[event['revision']]['step_guide_diameter_mm'][ARMS[i]]
            label(d,x+268,y+49,f"{event['end_s']:.2f} s",22,True)
            label(d,x+268,y+79,self.l(f'Last save  ·  bore {diameter:.2f} mm',f'最近交付  ·  孔径 {diameter:.2f} mm'),14,False,MUTED)
        else:
            label(d,x+268,y+64,self.l('Building first hand…','正在生成第一份…'),17,False,MUTED)
        ax=x+18;end=x+484;line=y+112
        d.line((ax,line,end,line),fill=RULE,width=2)
        for tick in track.handoffs[:n]:
            pos=ax+(end-ax)*tick/100
            d.line((pos,line-8,pos,line+2),fill=col,width=2)
        pos=ax+(end-ax)*t/100
        d.ellipse((pos-2,line-2,pos+2,line+2),fill=INK)

    def frame(self,t):
        im=Image.new('RGB',SIZE,BG);d=ImageDraw.Draw(im)
        label(d,31,22,self.l('Tendon hand  /  100-second STEP delivery','绳驱灵巧手  /  100 秒 STEP 交付'),32,True)
        label(d,1048,24,f'{t:05.1f} s',33,True,INK,'rt')
        label(d,33,76,self.l('Same sequence of 43-solid revisions. A mark appears when a full STEP has been saved.',
                              '两端按同一队列改参。每写完一份含 43 个实体的整手 STEP，就增加一条刻线。'),17,False,MUTED)
        label(d,1048,78,self.l('10× playback','10× 回放'),16,True,COLORS[0],'rt')
        d.line((30,106,1050,106),fill=RULE)
        self.storyboard(im,d,t)
        self.scoreboard(d,0,t);self.scoreboard(d,1,t)
        counts=[bisect_right(track.handoffs,t) for track in self.tracks]
        lead=counts[0]-counts[1]
        if lead:
            faster='colocad' if lead>0 else 'build123d'
            gap=self.l(f'{faster} leads by {abs(lead)} deliveries',f'{faster} 已多交付 {abs(lead)} 份')
        else:
            gap=self.l('Equal deliveries so far','当前交付数量相同')
        label(d,548,418,gap,17,True,COLORS[0] if lead>=0 else COLORS[1])
        d.line((30,446,1050,446),fill=RULE)
        label(d,31,461,self.l('Sandbox memory','沙箱内存'),23,True)
        label(d,206,469,self.l('memory.current sampled ~25 ms','约每 25 ms 采集 memory.current'),15,False,MUTED)
        ax,ex,top,bottom=86,731,512,632
        def u(sec):return ax+(ex-ax)*sec/100
        def v(mib):return bottom-(bottom-top)*mib/512
        for mem in (0,128,256,384,512):
            yy=v(mem);d.line((ax,yy,ex,yy),fill=RULE)
            label(d,ax-9,yy,str(mem),12,False,MUTED,'rm')
        for sec in (0,25,50,75,100):label(d,u(sec),642,f'{sec} s',13,False,MUTED,'mt')
        d.line((u(t),top,u(t),bottom),fill='#909e98')
        for i,track in enumerate(self.tracks):
            j=bisect_right(track.times,t)-1
            # Sample and hold. Never interpolate a future sample or claim a peak timestamp.
            pts=[(ax,v(track.mib[0]))]
            for k in range(1,j+1):
                xx=u(track.times[k]);pts.extend([(xx,v(track.mib[k-1])),(xx,v(track.mib[k]))])
            pts.append((u(t),v(track.mib[j])))
            d.line(pts,fill=COLORS[i],width=2)
            lastx,lasty=pts[-1];d.ellipse((lastx-3,lasty-3,lastx+3,lasty+3),fill=COLORS[i])
            x=765;y=510+65*i
            label(d,x,y,'colocad' if i==0 else 'build123d',15,True,COLORS[i])
            label(d,x,y+25,f'{track.mib[j]:.1f} / {track.high[j]:.1f} MiB',19,True,COLORS[i])
        label(d,764,644,self.l('Current / sampled peak so far','当前 / 截至此刻的采样峰值'),14,False,MUTED)
        label(d,32,685,self.l('Two serial recordings, starts aligned  ·  1 vCPU / 512 MiB each  ·  CAD detail shots are an editorial replay',
                              '两次独立实测按启动时刻对齐，各 1 vCPU / 512 MiB。左侧为几何过程回放，计数和内存均来自时间戳。'),14,False,MUTED)
        return im


def encode(film,path):
    ff=['ffmpeg','-loglevel','error','-y']
    command=ff+['-f','rawvideo','-pix_fmt','rgb24','-s',f'{SIZE[0]}x{SIZE[1]}','-r',str(FPS),'-i','-',
                '-an','-c:v','libx264','-threads','2','-crf','18','-pix_fmt','yuv420p','-movflags','+faststart',str(path)]
    with subprocess.Popen(command,stdin=subprocess.PIPE) as proc:
        for i in range(FPS*10):
            # Each frame closes a 0.4 s measurement bin. The final frame includes all 100 s.
            proc.stdin.write(film.frame((i+1)*10/FPS).tobytes())
        proc.stdin.close()
        if proc.wait():raise RuntimeError('Could not encode the race')
    with tempfile.TemporaryDirectory() as tmp:
        palette=str(Path(tmp)/'palette.png')
        subprocess.run(ff+['-i',str(path),'-vf','palettegen=stats_mode=full',palette],check=True)
        subprocess.run(ff+['-i',str(path),'-i',palette,'-filter_complex','paletteuse=dither=bayer:bayer_scale=4:diff_mode=rectangle',
                           '-loop','0',str(path.with_suffix('.gif'))],check=True)


if __name__=='__main__':
    p=argparse.ArgumentParser()
    p.add_argument('evidence',type=Path);p.add_argument('assets',type=Path);p.add_argument('out',type=Path)
    args=p.parse_args();args.out.mkdir(exist_ok=True,parents=True)
    records=json.loads(args.evidence.read_text())
    for zh,lang in [(True,'zh-CN'),(False,'en')]:
        film=Film(records,args.assets,zh)
        encode(film,args.out/f'hand-window-{lang}.mp4')
        if zh:film.frame(100).save(args.out/'finish.png')
