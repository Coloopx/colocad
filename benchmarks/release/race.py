"""A 10× replay from three recorded queues. Stills show geometry; charts show actual timestamps."""
import bisect,json,sys
from pathlib import Path
import numpy as np,build123d as b
from PIL import Image,ImageDraw
repo=Path(__file__).resolve().parents[2];sys.path.insert(0,str(repo/'showcase/tendon-hand/tools'))
from projection import CAMERAS,render,font
out=Path(sys.argv[1]);dest=Path(sys.argv[2]);data=json.loads((dest/'observations.json').read_text());w=data['windows'];bs=['colocad','cadquery','build123d'];names=['Colocad','CadQuery','build123d'];colors=['#006a66','#98601a','#564d98']
parts=b.import_step(out/'windows/colocad/000.step').solids();meshes=[]
for i,p in enumerate(parts):
 v,idx=p.tessellate(.20,.14);meshes.append((np.array([tuple(pt) for pt in v]),np.array(idx),[64,150,146] if i%4 else [173,153,103]))
bg='#f5f6f2';base=Image.new('RGB',(1000,530),bg);dr=ImageDraw.Draw(base)
def txt(draw,x,y,msg,size=16,bold=False,col='#1a343b'):draw.text((x,y),msg,font=font(size,bold),fill=col)
txt(dr,22,16,'Colocad '+data['version']+'   /   1 vCPU, 512 MiB',14,True,'#006a66');txt(dr,22,40,'How many complete hand revisions fit in 100 seconds?',29,True)
txt(dr,22,85,'43-solid tendon hand  /  first delivered STEP',16,True)
for j,(cam,vec) in enumerate(CAMERAS.items()):
 x=19+j%2*193;y=113+j//2*168
 base.paste(render(meshes,vec,scale=.72,size=(190,145)),(x,y));txt(dr,x+4,y+141,cam.upper(),12,True)
txt(dr,23,483,'Four fixed readback views; geometry shown at revision 1.',13)
axis0,axis1=452,967
def chart_title(y,label,mx):
 txt(dr,axis0-26,y-32,label,16,True);dr.line((axis0,y,axis0,y+95,axis1,y+95),fill='#9aa9a5',width=2)
 txt(dr,axis0-42,y-5,str(mx),12);txt(dr,axis0-17,y+83,'0',12)
maxn=max(w[b]['count'] for b in bs);top=((maxn+9)//10)*10
chart_title(148,'Verified STEP handoffs',top);chart_title(330,'Sampled memory peak so far  /  MiB',512)
for x,t in [(axis0,'0 s'),((axis0+axis1)//2,'50 s'),(axis1-28,'100 s')]:txt(dr,x,428,t,12)
def xy(t,y,val,mx):return (axis0+(axis1-axis0)*t/100,y+95-95*val/mx)
frames=[]
for frame in range(100):
 t=frame+1;im=base.copy();draw=ImageDraw.Draw(im);txt(draw,805,91,f'{t:3d} s  /  10× replay',17,True)
 for i,bk in enumerate(bs):
  arm=w[bk];data=arm['deliveries'];count=bisect.bisect_right(data,t);peak=0;points=[xy(0,148,0,top)]
  for k,sec in enumerate(data[:count]):points.extend([xy(sec,148,k,top),xy(sec,148,k+1,top)])
  points.append(xy(t,148,count,top));draw.line(points,fill=colors[i],width=3)
  points=[]
  for sec,v in arm['memory']:
   if sec>t:break
   value=v/2**20
   if not points or value>peak:
    if points:points.append(xy(sec,330,peak,512))
    peak=max(peak,value);points.append(xy(sec,330,peak,512))
  points.append(xy(t,330,peak,512));draw.line(points,fill=colors[i],width=3)
  x=428+i*184;txt(draw,x,459,names[i],17,True,colors[i]);txt(draw,x,482,f'{count} STEP  ·  {peak:.0f} MiB',14,False,colors[i])
 frames.append(im.convert('P',palette=Image.Palette.ADAPTIVE,colors=160))
frames[0].save(dest/'hand-window.gif',save_all=True,append_images=frames[1:],duration=100,loop=0,optimize=True)
