"""Render the hand story and replay both newly measured SDK calls on their own wall clocks."""
import argparse
from functools import lru_cache
import json
import math
from pathlib import Path
import subprocess
import sys
import tempfile

from PIL import Image, ImageDraw, ImageOps

HAND = Path(__file__).resolve().parents[1]
REPO = HAND.parents[1]
sys.path.insert(0,str(REPO/'benchmarks/readme/replay'))
from animate import Replay, WIDTH, HEIGHT, FPS, RULE, MUTED, INK, COLORS, label

GOLD = '#af6931'
STAGES = ['wrist','palm','proximal','middle','tips','thumb','drive']


@lru_cache(maxsize=60)
def photo(path):
    return Image.open(path).convert('RGB')


def place(image, source, bounds):
    x,y,w,h=bounds
    tile=ImageOps.contain(source,(w,h),Image.Resampling.LANCZOS)
    image.paste(tile,(x+(w-tile.width)//2,y+(h-tile.height)//2))


class HandMovie(Replay):
    def __init__(self, case, assets, pictures, zh):
        self.case=case
        self.zh=zh
        self.assets=assets
        self.pictures=pictures
        self.runs=json.loads((HAND/'replay'/f'{case}-trace.json').read_text())['recorded']
        self.last=max(row['wall_ms'] for row in self.runs)
        self.extent=math.ceil(self.last/500)*500
        self.images={r['backend']:photo(pictures/f"{case}-{r['backend']}.png") for r in self.runs}
        self.stats=json.loads((HAND/'evidence/reuse-summary.json').read_text())
        self.geo=json.loads((HAND/'evidence/geometry.json').read_text())

    def race_heading(self):
        title = self.l('Build one proximal finger link','单独交付食指近节') if self.case=='finger' else self.l('Build and deliver the complete hand','交付完整灵巧手')
        return title,self.l('Optimized source kernel vs build123d   ·   independently recorded launches synchronized at 1×',
                            '优化内核与 build123d 独立运行，两次启动按 1× 原速对齐。')

    def pending(self,image,draw,x):
        reference=photo(HAND/'reference/front.png')
        if self.case=='finger':
            reference=reference.crop((233,133,282,255))
        place(image,reference,(x+224,218,232,105))
        return self.l('Reference view','参考视图')

    def footer(self,draw,text):
        draw.line((32,575,992,575),fill=RULE)
        label(draw,33,601,text,15,color=MUTED)

    def asset(self,image,name,bounds):
        place(image,photo(self.assets/(name+'.png')),bounds)

    def opening(self):
        im,d=self.canvas(self.l('From four views to a tendon-driven hand','从四视图重建绳驱灵巧手'),
                         self.l('Shadow Hand E3M5 right hand  ·  neutral pose  ·  four calibrated reference views',
                                '参考件为 Shadow Hand E3M5 右手，四张视图使用同一毫米刻度。'))
        titles=[self.l('Front','正视'),self.l('Side','侧视'),self.l('End','端视'),self.l('Oblique','斜视')]
        for i,k in enumerate(('front','side','end','oblique')):
            x=33+(i%2)*191;y=165+(i//2)*194
            source=photo(HAND/'reference'/(k+'.png')).copy()
            if k=='front':
                ImageDraw.Draw(source).rectangle((235,133,280,254),outline=GOLD,width=5)
            place(im,source,(x,y,180,166))
            label(d,x+9,y+167,titles[i],15,True,MUTED)
        d.line((432,167,432,548),fill=RULE)
        self.asset(im,'drive',(457,146,516,391))
        label(d,521,540,self.l('Reconstructed   /   43 solids','重建装配体  /  43 个实体'),19,True,COLORS[0])
        self.footer(d,self.l('Reference geometry © Shadow Robot / MuJoCo Menagerie, Apache 2.0. Internal cord routes are assumptions.',
                             '参考模型来自 Shadow Robot / MuJoCo Menagerie，Apache 2.0。内部绳路为示意设计。'))
        return im

    def operation(self,stage):
        keys=['ProximalBlank','ProximalFork','cutter','cutaway','wire']
        titles=[('Extrude the link blank','拉伸指骨毛坯'),('Machine the clevis','切出关节叉口'),('Bore the tendon channel','切出纵向绳道'),
                ('Inspect and edit the guide bore','剖开指骨，验证孔径改动'),('Tessellate the finger link','将指骨曲面三角化')]
        im,d=self.canvas(self.l(*titles[stage]),self.l('Index proximal link   ·   reconstruction detail outside the benchmark clock',
                                                        '食指近节的真实几何回放。以下镜头独立于性能计时。'))
        source=photo(HAND/'reference/front.png').copy()
        ImageDraw.Draw(source).rectangle((235,133,280,254),outline=GOLD,width=6)
        place(im,source,(27,164,305,379))
        self.asset(im,keys[stage],(335,163,340,378))
        if stage == 3:
            # Overlay the two actual circular STEP section contours at one enlarged scale.
            outlines=json.loads((self.assets/'loops.json').read_text())
            for key, color in [('guide-edit',GOLD),('ProximalCableGuide',COLORS[0])]:
                bore=min(outlines[key],key=lambda loop:max(x for x,y in loop)-min(x for x,y in loop))
                points=[(370+19*x,469-19*(y+5)) for x,y in bore]
                d.line(points,fill=color,width=3)
            label(d,306,511,'Ø2.9 / Ø3.3 mm',15,True,MUTED)
        for i,title in enumerate(titles):
            y=202+52*i
            color=COLORS[0] if i==stage else MUTED
            d.ellipse((681,y+3,692,y+14),fill=color if i<=stage else RULE)
            label(d,708,y,self.l(*title),18,i==stage,color)
        info=[self.l('Blank STEP: 6 faces.','毛坯回读为 6 个面。'),self.l('The fork stage has 15 faces.','叉口切除后共 15 个面。'),
              self.l('Gold shows the actual cylindrical subtraction tool.','金色是布尔切除使用的圆柱刀体。'),
              self.l('At z = 23 mm: Ø2.900 → Ø3.300 mm. The wider bore removes 86.727 mm³.','在 z = 23 mm 截面量得 Ø2.900 → Ø3.300 mm，单根指骨减少 86.727 mm³。'),
              self.l('6,066 preview triangles in this link. The exported STEP retains its curved faces.','这根指骨的预览含 6,066 个三角形，STEP 保留精确曲面。')]
        self.footer(d,info[stage])
        return im

    def assembly(self,i):
        im,d=self.canvas(self.l('Assemble the five fingers','从腕座装到五根手指'),
                         self.l('Actual STEP milestones   ·   one camera and one scale','每一步都从该阶段实际导出的 STEP 渲染，相机和比例保持一致。'))
        self.asset(im,STAGES[i],(30,153,536,411))
        names=[('Wrist','腕座'),('Palm and sockets','手掌和指根座'),('Proximal links','近节指骨'),('Middle links','中节指骨'),
               ('Fingertips','指尖'),('Opposed thumb','外展拇指'),('Pins and cords','销轴与绳索')]
        for j,k in enumerate(STAGES):
            y=185+j*47; col=COLORS[0] if j<=i else MUTED
            label(d,592,y,self.l(*names[j]),22,j==i,col)
            label(d,957,y,str(self.geo['scenes'][k]['solids']),22,j==i,col,'rt')
        self.footer(d,self.l('Numbers show cumulative solids. Assembly motion and load capacity have not been tested.',
                             '右列为该阶段累计实体数。展示中采用中立姿态。'))
        return im

    def explanation(self):
        im,d=self.canvas(self.l('Why did more parts take so long?','为什么零件一多，旧版就慢下来？'),
                         self.l('Controlled experiment: one finger definition repeated with identical orientation.',
                                '对照实验只改变同一根指骨的摆放数量，朝向与几何定义保持一致。'))
        label(d,43,183,self.l('Copies','实例数'),19,True)
        label(d,202,183,'colocad alpha.2',19,True)
        label(d,510,183,self.l('Optimized kernel','优化内核'),19,True,COLORS[0])
        label(d,793,183,'build123d',19,True)
        for i,n in enumerate((1,4,16,43)):
            y=230+53*i
            label(d,50,y,str(n),23,True)
            row=self.stats['links-'+str(n)]
            for x,key in [(202,'colocad'),(510,'candidate'),(793,'build123d')]:
                label(d,x,y,f"{row[key]['median_ms']/1000:.2f} s",22,key=='candidate',COLORS[0] if key=='candidate' else MUTED)
        d.line((40,449,977,449),fill=RULE)
        label(d,41,476,self.l('43 placements: one full check, 42 translated results reused.',
                               '同一朝向摆放 43 次，完整检查 1 次，平移复用 42 次。'),23,True,COLORS[0])
        self.footer(d,self.l('Time to STEP: five-run medians. Python imports are paid once per tool call.',
                             '五轮新进程交付 STEP 的中位数。Python 依赖每次工具调用只导入一次。'))
        return im

    def outcome(self):
        im,d=self.canvas(self.l('What changed for the complete hand?','优化让整手交付快了多少？'),
                         self.l('43 solids, including rotated parts  ·  five fresh processes per backend',
                                '43 个实体，包含不同形状和不同朝向的零件，每个后端各测 5 轮。'))
        place(im,photo(self.pictures/'hand-colocad-large.png'),(26,160,382,399))
        row=self.stats['hand']
        for i,(key,name) in enumerate([('colocad','colocad alpha.2'),('candidate',self.l('Optimized source','colocad 优化内核')),('build123d','build123d')]):
            y=222+i*81
            label(d,420,y,name,19,True,COLORS[0] if key=='candidate' else INK)
            label(d,716,y,f"{row[key]['median_ms']/1000:.2f} s",23,True,anchor='rt')
            label(d,975,y,f"{row[key]['max_peak_mib']:.1f} MiB",23,True,anchor='rt')
        label(d,716,172,self.l('Median','耗时中位数'),15,False,MUTED,'rt')
        label(d,975,172,self.l('Max cgroup peak','最高内存峰值'),15,False,MUTED,'rt')
        label(d,420,486,self.l('21 full checks  ·  22 instance reuses','完整检查 21 次，复用 22 个实例'),21,True,COLORS[0])
        self.footer(d,self.l('All 43 solids reopened; both boolean differences were zero for each of the 20 unique definitions.',
                             '43 个实体均通过回读，20 个不同零件定义在优化前后的双向布尔差集均为 0 mm³。'))
        return im


def encode(finger, hand, path):
    chapters={}
    frames=0
    ff=['ffmpeg','-loglevel','error','-y']
    proc=subprocess.Popen(ff+['-f','rawvideo','-pix_fmt','rgb24','-s',f'{WIDTH}x{HEIGHT}','-r',str(FPS),'-i','-',
                          '-an','-c:v','libx264','-threads','2','-crf','19','-pix_fmt','yuv420p','-movflags','+faststart',str(path)],stdin=subprocess.PIPE)
    def hold(im,n):
        nonlocal frames
        buf=im.tobytes()
        for _ in range(n):proc.stdin.write(buf)
        frames+=n
    def race(scene,key):
        chapters[key]=frames/FPS
        for i in range(math.ceil(scene.last*FPS/1000)+1):
            hold(scene.race(min(i*1000/FPS,scene.last)),1)
        for row in scene.runs:
            chapters[key+'-'+row['backend']]=chapters[key]+(math.ceil(row['wall_ms']*FPS/1000)+0.25)/FPS
        hold(scene.race(scene.last),16)
    try:
        hold(hand.opening(),45)
        chapters['geometry']=frames/FPS
        for i in range(5):hold(hand.operation(i),20 if i!=3 else 35)
        race(finger,'finger')
        chapters['assembly']=frames/FPS
        previous=None
        for i in range(7):
            current=hand.assembly(i)
            if previous is not None:
                for progress in [.25,.50,.75]:hold(Image.blend(previous,current,progress),1)
            hold(current,12);previous=current
        chapters['cause']=frames/FPS
        hold(hand.explanation(),70)
        race(hand,'hand')
        chapters['result']=frames/FPS
        hold(hand.outcome(),85)
    finally:
        proc.stdin.close()
        if proc.wait():raise RuntimeError('FFmpeg encoding failed')
    with tempfile.TemporaryDirectory() as t:
        pal=str(Path(t)/'palette.png')
        subprocess.run(ff+['-i',str(path),'-vf','palettegen=stats_mode=full',pal],check=True)
        subprocess.run(ff+['-i',str(path),'-i',pal,'-filter_complex','paletteuse=dither=bayer:bayer_scale=4:diff_mode=rectangle',
                           '-loop','0',str(path.with_suffix('.gif'))],check=True)
    return {'fps':FPS,'duration_s':frames/FPS,'chapters_s':chapters}


if __name__=='__main__':
    p=argparse.ArgumentParser()
    p.add_argument('stages',type=Path);p.add_argument('pictures',type=Path)
    args=p.parse_args();out=HAND/'replay'
    for zh,lang in [(True,'zh-CN'),(False,'en')]:
        finger=HandMovie('finger',args.stages,args.pictures,zh)
        hand=HandMovie('hand',args.stages,args.pictures,zh)
        # Main README and offline player use precisely the same picture sequence.
        timeline=encode(finger,hand,out/f'tendon-hand-{lang}.mp4')
        if zh:
            hand.outcome().save(out/'poster.png')
            (out/'timeline.json').write_text(json.dumps(timeline,indent=2)+'\n')
