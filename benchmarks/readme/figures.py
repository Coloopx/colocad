"""Produce the README figures from runs.json and the four STEP exports, outside timing."""
import argparse
import json
import statistics
from pathlib import Path

import build123d as bd
import matplotlib
import numpy as np
from PIL import Image, ImageDraw, ImageFont, ImageFilter

def unit(v):
    return v / np.linalg.norm(v)

def render(path, camera, bounds):
    # Orthographic, fixed world-space bounds from the design for both backends.
    w,h = 936,422
    front = unit(np.array(camera, dtype=float))
    right = unit(np.cross([0,0,np.sign(front[2])],front))
    up = np.cross(front,right)
    axes = np.column_stack([right,-up,front])
    bbox = np.array([[x,y,z] for x in bounds[0] for y in bounds[1] for z in bounds[2]]) @ axes
    center = (bbox.min(0) + bbox.max(0))/2
    scale = min((w-95)/np.ptp(bbox[:,0]), (h-54)/np.ptp(bbox[:,1]))
    shape = bd.import_step(path)
    vertices, indices = shape.tessellate(0.06,0.08)
    vertices = np.array([tuple(v) for v in vertices])
    tris = np.asarray(indices)
    xyz = ((vertices @ axes)-center)*scale
    xyz[:,:2] += [w/2,h/2]
    vs = vertices[tris]
    n = np.cross(vs[:,1]-vs[:,0],vs[:,2]-vs[:,0])
    n /= np.maximum(np.linalg.norm(n,axis=1,keepdims=True),1e-12)
    light = unit(-0.4*right+0.75*up+0.65*front)
    n[n@front < 0] *= -1
    shade = (0.41 + 0.46*np.maximum(n@light,0) + 0.13*np.maximum(n@front,0))
    colors = np.clip(np.array([45,92,106]) + shade[:,None]*[145,145,138],0,255)
    zbuf = np.full((h,w),-np.inf)
    pixels = np.zeros((h,w,3),dtype=np.uint8)
    for tri,c in zip(xyz[tris],colors):
        xmin,ymin = np.maximum(np.floor(tri[:,:2].min(0)).astype(int),[0,0])
        xmax,ymax = np.minimum(np.ceil(tri[:,:2].max(0)).astype(int),[w-1,h-1])
        if xmin>xmax or ymin>ymax: continue
        (ax,ay,az),(bx,by,bz),(cx,cy,cz)=tri
        det=(by-cy)*(ax-cx)+(cx-bx)*(ay-cy)
        if abs(det)<1e-10: continue
        yy,xx=np.ogrid[ymin:ymax+1,xmin:xmax+1]
        u=((by-cy)*(xx+0.5-cx)+(cx-bx)*(yy+0.5-cy))/det
        v=((cy-ay)*(xx+0.5-cx)+(ax-cx)*(yy+0.5-cy))/det
        z=u*az+v*bz+(1-u-v)*cz
        area=np.s_[ymin:ymax+1,xmin:xmax+1]; keep=(u>=0)&(v>=0)&(u+v<=1)&(z>zbuf[area])
        zbuf[area][keep]=z[keep]; pixels[area][keep]=c
    # Contact shading: darken a wall or floor only when neighbouring geometry
    # is closer to the camera. Skip empty pixels outside the silhouette.
    valid = np.isfinite(zbuf)
    shade_factor = np.ones((h,w))
    for dx,dy in ((0,9),(0,-9),(-9,0),(9,0),(8,8),(-8,8),(-8,-8),(8,-8)):
        near = np.roll(zbuf,(dy,dx),(0,1))
        occupied = valid & np.isfinite(near)
        shade_factor[occupied] -= 0.053*np.clip((near[occupied]-zbuf[occupied]-9)/18,0,1)
    pixels = (pixels * shade_factor[:,:,None]).astype(np.uint8)
    mask = Image.fromarray((np.isfinite(zbuf)*255).astype('uint8'))
    base=Image.new('RGB',(w,h),'#eaf1f0')
    shadow=Image.new('L',(w,h)); shadow.paste(mask,(0,9))
    shadow=shadow.filter(ImageFilter.GaussianBlur(10)).point(lambda x:x*0.17)
    base.paste('#18363c',(0,0,w,h),shadow)
    base.paste(Image.fromarray(pixels),(0,0),mask)
    return base.resize((468,211),Image.Resampling.LANCZOS)

def comparison(directory):
    img=Image.new('RGB',(1240,636),'#f8faf8')
    d=ImageDraw.Draw(img)
    fonts=Path(matplotlib.get_data_path())/'fonts/ttf'
    def label(x,y,s,size,bold=False,color='#18373b'):
        name='DejaVuSans-Bold.ttf' if bold else 'DejaVuSans.ttf'
        d.text((x,y),s,font=ImageFont.truetype(str(fonts/name),size), fill=color,anchor='lt')
    for x,txt in [(232,'colocad'),(733,'build123d')]:
        label(x,37,txt,30,True)
    label(32,47,'STEP READBACK',16,True,'#477274')
    for i,(case,title,features,cam,bounds) in enumerate([
        ('sensor-rail','Sensor rail',['Three through-slots','30 × 66 × 9 mm'],(1,0.72,1.12),[(-15,15),(-11,55),(0,9)]),
        ('manifold','Manifold',['Four blind ports','90 × 45 × 12 mm'],(-0.46,-1,-1.3),[(0,90),(0,45),(0,12)])]):
        y=96+247*i
        label(32,y+51,f'0{i+1}',17,True,'#477274')
        label(32,y+85,title,24,True)
        for n,s in enumerate(features):label(32,y+130+n*25,s,15,color='#496263')
        for j,name in enumerate(['colocad','build123d']):
            img.paste(render(directory/f'{case}-{name}.step',cam,bounds),(232+501*j,y))
    d.line((32,580,1200,580),fill='#c7d8d6',width=1)
    label(32,601,'Independent STEP exports   /   Same OCCT mesher, camera, scale and lighting within each row',17,color='#456364')
    return img

def chart(rows):
    lower = [r for r in rows if r['rep']>0]
    keys=['sensor-rail','manifold']
    panels=[]
    def text(x,y,s,sz,bold=False,fill='#15363a'):
        return f'<text x="{x}" y="{y}" font-family="DejaVu Sans,Arial,sans-serif" font-size="{sz}" font-weight="{700 if bold else 400}" fill="{fill}">{s}</text>'
    for p,(title,sub,field,cap) in enumerate([('Time to STEP','Median wall time · milliseconds','wall_ms',3200),('Sandbox memory','Highest measured cgroup peak · MiB','peak_bytes',300)]):
        x=30+p*501
        panels += [text(x,92,title,28,True),text(x,119,sub,15,fill='#506568')]
        for i,k in enumerate(keys):
            y=160+i*108
            panels.append(text(x,y,'Sensor rail' if not i else 'Manifold',18,True))
            for j,name in enumerate(['colocad','build123d']):
                data=[r[field] for r in lower if r['case']==k and r['backend']==name]
                v=statistics.median(data) if p==0 else max(data)/2**20
                unit='ms' if p==0 else 'MiB'
                lab=f'{v:,.0f} {unit}' if p==0 else f'{v:,.1f} {unit}'
                yy=y+15+j*28
                color='#087d75' if not j else '#566f82'
                panels.extend([text(x,yy+11,name,14,fill=color),f'<rect x="{x+88}" y="{yy}" width="{v/cap*248:.2f}" height="13" rx="3" fill="{color}"/>',text(x+98+v/cap*248,yy+12,lab,15,True)])
    return ('<svg xmlns="http://www.w3.org/2000/svg" role="img" aria-labelledby="title desc" viewBox="0 0 1000 369">'
            '<title id="title">Time to STEP and peak sandbox memory</title><desc id="desc">Five fresh runs per case and backend; shorter bars are better. Exact numbers are also in the README.</desc>'
            '<rect width="1000" height="369" rx="12" fill="#f4f8f7"/>'
            +text(30,37,'LINUX ARM64     /     1 vCPU     /     512 MiB     /     5 RUNS PER PAIR',15,True,'#397073')
            +'<line x1="502" x2="502" y1="72" y2="335" stroke="#c1d3d2"/>' + ''.join(panels)+'</svg>\n')

if __name__ == '__main__':
    p=argparse.ArgumentParser()
    p.add_argument('trials',type=Path)
    p.add_argument('destination',type=Path)
    a=p.parse_args()
    a.destination.mkdir(exist_ok=True,parents=True)
    comparison(a.trials).save(a.destination/'step-comparison.png',optimize=True)
    (a.destination/'sandbox-performance.svg').write_text(chart(json.loads((a.trials/'runs.json').read_text())['runs']))
