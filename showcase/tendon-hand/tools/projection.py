"""Orthographic camera and renderer shared by input and STEP readback. Units: mm."""
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw, ImageFont
CAMERAS={'front':(0,-1,0),'side':(1,0,0),'end':(0,0,1),'oblique':(.76,-1,.38)}
CENTER=np.array([24.,-4.,78.])
WIDTH,HEIGHT=480,548
SCALE=1.98
def unit(v): return np.asarray(v)/np.linalg.norm(v)
def axes(camera):
    n=unit(camera); right=unit(np.cross([0,0,1] if abs(n[2])<.999 else [0,1,0],n))
    return np.column_stack((right,-np.cross(n,right),n))
def project(points,camera,scale=SCALE,center=CENTER,size=(WIDTH,HEIGHT)):
    v=(points-center)@axes(camera);v*=scale;v[:,:2]+=np.array(size)/2
    return v
def render(meshes,camera,scale=SCALE,center=CENTER,size=(WIDTH,HEIGHT),mask=False):
    # Each entry is (vertices_mm, triangle indices, base RGB). Supersample each silhouette.
    polys=[];colors=[];depth=[]; ax=axes(camera)
    for v,i,color in meshes:
        w=project(v,camera,scale*2,center,(size[0]*2,size[1]*2)); pts=w[i]
        n=np.cross(v[i[:,1]]-v[i[:,0]],v[i[:,2]]-v[i[:,0]]);n/=np.maximum(np.linalg.norm(n,axis=1,keepdims=True),1e-12)
        n[(n@ax[:,2])<0]*=-1
        light=unit(-.42*ax[:,0]-.64*ax[:,1]+.74*ax[:,2])
        s=.50+.44*np.maximum(0,n@light)+.06*np.maximum(0,n@ax[:,2])
        c=np.clip(s[:,None]*np.array(color)+5,0,255)
        polys.append(pts[:,:,:2].astype(int).reshape(-1,6));colors.extend(map(tuple,c.astype('uint8').tolist()));depth.append(pts[:,:,2].mean(1))
    im=Image.new('RGB',(size[0]*2,size[1]*2),'#f5f6f2');dr=ImageDraw.Draw(im)
    polys=np.concatenate(polys).tolist()
    for j in np.argsort(np.concatenate(depth),kind='stable'):
        dr.polygon(polys[j],fill=(0,0,0) if mask else colors[j])
    return im.resize(size,Image.Resampling.LANCZOS)
def font(size,bold=False):
    for path in (f'/System/Library/Fonts/Supplemental/Arial{" Bold" if bold else ""}.ttf',
                 f'/usr/share/fonts/truetype/dejavu/DejaVuSans{"-Bold" if bold else ""}.ttf'):
        if Path(path).exists():return ImageFont.truetype(path,size)
    return ImageFont.load_default(size=size)
def contact(views,title):
    page=Image.new('RGB',(1020,1260),'#f5f6f2'); d=ImageDraw.Draw(page)
    d.text((24,15),title,font=font(28,True),fill='#16373e')
    d.text((25,52),'Palm datum O  /  orthographic  /  scale in millimetres',font=font(18),fill='#43666c')
    for ix,k in enumerate(CAMERAS):
        x=21+(ix%2)*500;y=91+(ix//2)*578
        page.paste(views[k],(x,y))
        d.text((x+12,y+8),k.upper(),font=font(17,True),fill='#31585b')
        u,v=project(np.array([[0,0,0]]),CAMERAS[k])[0,:2]
        if 0<u<WIDTH and 0<v<HEIGHT:
            d.line((x+u-4,y+v,x+u+4,y+v),fill='#c6643e',width=2);d.line((x+u,y+v-4,x+u,y+v+4),fill='#c6643e',width=2)
        d.line((x+13,y+HEIGHT-19,x+13+round(20*SCALE),y+HEIGHT-19),fill='#285257',width=3)
        d.text((x+17+round(20*SCALE),y+HEIGHT-29),'20 mm',font=font(15),fill='#31585b')
    return page
