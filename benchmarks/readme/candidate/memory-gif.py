"""Animate measured cgroup current, aligned by accepted edit count (not by elapsed seconds)."""
import bisect
import json
from pathlib import Path
import sys
from PIL import Image, ImageDraw
repo=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(repo/'showcase/tendon-hand/tools'))
from projection import font

dest=Path(sys.argv[1]);data=json.loads((dest/'engineering.json').read_text());runs=data['sessions']
ink='#162e35';muted='#455d63';bg='#f5f6f2';grid='#d3ded8'
names={'colocad':'Colocad RC','cadquery':'CadQuery','build123d':'build123d','alpha10':'Colocad α.10'}
colors={'colocad':'#00776b','cadquery':'#965617','build123d':'#634d96','alpha10':'#687780'}
order=list(names)
def txt(draw,x,y,s,size,fill=ink,b=False):draw.text((x,y),s,font=font(size,b),fill=fill)
width,height=960,464
left,top,right,bottom=75,120,630,365
def xy(n,mib):return (left+(right-left)*n/256,bottom-(bottom-top)*mib/512)
def frame(edit):
    im=Image.new('RGB',(width,height),bg);d=ImageDraw.Draw(im)
    txt(d,25,16,'COLOCAD   /   PERSISTED CAD HISTORY',15,colors['colocad'],True)
    txt(d,25,44,'256 geometry edits in a 512 MiB sandbox',32,ink,True)
    txt(d,25,91,'Current cgroup memory (MiB)',15,muted,True)
    for mem in (0,128,256,384,512):
        _,y=xy(0,mem);d.line((left,y,right,y),fill=grid,width=1);txt(d,left-40,y-9,str(mem),14,muted)
    for n in (0,64,128,192,256):
        x,_=xy(n,0);d.line((x,bottom,x,bottom+4),fill=muted);txt(d,x-9,bottom+7,str(n),14,muted)
    txt(d,218,bottom+30,'Accepted geometry edits',16,muted)
    txt(d,671,100,f'Edit {edit:3d} / 256',20,ink,True)
    txt(d,672,142,'Backend',14,muted);txt(d,806,142,'Current',14,muted);txt(d,890,142,'Peak*',14,muted)
    for i,key in enumerate(order):
        s=runs[key]['checkpoints'];idx=bisect.bisect_right([r['edits'] for r in s],edit)-1
        points=[xy(v['edits'],v['current_mib']) for v in s[:idx+1]]
        if len(points)>1:d.line(points,fill=colors[key],width=4 if key=='colocad' else 3)
        x,y=points[-1];d.ellipse((x-3,y-3,x+3,y+3),fill=colors[key])
        yy=174+i*44
        d.rounded_rectangle((672,yy+7,683,yy+18),3,fill=colors[key]);txt(d,691,yy,names[key],16,ink,True)
        txt(d,810,yy,f"{s[idx]['current_mib']:.1f}",17);txt(d,888,yy,f"{s[idx]['peak_mib']:.1f}",17)
    txt(d,670,358,'* Kernel peak through this edit.',14,muted)
    txt(d,25,429,'Same KK60 edit sequence · Linux arm64 · 1 vCPU · Native volume · One session per implementation',16)
    return im
edits=[256,0,1,*range(4,257,4)];frames=[frame(n) for n in edits]
frames[0].save(dest/'revision-memory.gif',save_all=True,append_images=frames[1:],
               duration=[2000,500,120]+[110]*(len(edits)-4)+[2000],loop=0,optimize=True,disposal=2)
