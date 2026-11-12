"""Render accepted STEP files and rejected source fixtures from the two engineering work orders."""
import argparse
from functools import lru_cache
import json
from pathlib import Path
import sys
import build123d as bd
import numpy as np
from PIL import Image,ImageDraw
repo=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(repo/'showcase/tendon-hand/tools'))
from projection import render,font

p=argparse.ArgumentParser();p.add_argument('trial',type=Path);p.add_argument('destination',type=Path);a=p.parse_args()
field=a.trial/'field';flange=a.trial/'flange'
backends=['colocad','cadquery','build123d'];names=['Colocad','CadQuery','build123d']
flange_inputs=repo/'specs/experiments/0004/fixtures/engineering'
fixtures={c['id']:flange_inputs/c['file'] for c in json.loads((flange_inputs/'manifest.json').read_text())['cases']}
rendering={'install':('Install the camera adapter','Four mounting holes, two camera bores and a 5 mm plate.'),'resize':('Resize the adapter','Plate length 44 to 54 mm, gauge 5 to 7 mm, camera pitch 16 to 22 mm.'),'service':('Cut a cable window','An 8 × 4 mm through-window added to the revised plate.'),'missing':('Reject the three-hole saddle','The mount requires four openings. The replacement has three.'),'replace':('Refit to a relocated saddle','The mounting coordinates moved 7 mm in X and 11 mm in Z.'),'motor':('Locate a keyed motor flange','The register and key bore define the installation frame.'),'motor-housing':('Swap to a larger housing','The housing changes while the locating geometry still fits.'),'motor-rotated':('Recover the mounting frame','The replacement is translated and rotated 1.1 radians about an oblique axis.'),'motor-wrong-pitch':('Reject a wrong bolt pitch','The incoming pattern measures 25 × 20 mm. The support requires 24 × 20 mm.'),'motor-missing-hole':('Reject a missing flange bore','Three of four required bolt axes are present.')}
orders=json.loads((repo/'benchmarks/field-task/orders.json').read_text())
decisions={}
for b in backends:
    check=json.loads((field/(b+'-checked.json')).read_text())
    flange_check=json.loads((flange/b/'occt-readback.json').read_text())
    runs=json.loads((flange/b/'runs.json').read_text())['runs']
    decisions[b]={r['id']:field/b/(r['id']+'.step') for r in check['setup']}
    decisions[b].update({r['id']:None for r in check['rejections']})
    decisions[b].update({r['id']:flange/b/r['artifactDirectory']/'model.step' if r['built']['ok'] else None for r in runs})
    assert set(r['id'] for r in flange_check['verified'])=={r['id'] for r in runs if r['built']['ok']}
    assert len(decisions[b])==10 and sum(v is not None for v in decisions[b].values())==7

@lru_cache(maxsize=64)
def meshes(path,saddle):
    solids=list(bd.import_step(str(path)).solids())
    solids.sort(key=lambda s: s.bounding_box().min.Y if saddle else s.bounding_box().min.Z)
    result=[]
    for i,solid in enumerate(solids):
        vv,ff=solid.tessellate(.12,.12);vertices=np.array([tuple(v) for v in vv])
        if saddle:vertices=np.column_stack([vertices[:,0],-vertices[:,2],vertices[:,1]])
        colors=([100,130,145],[50,186,161]) if saddle else ([100,130,145],[222,155,80])
        result.append((vertices,np.asarray(ff),colors[i%len(colors)]))
    return result

bg='#f5f6f2';ink='#162e35';teal='#00685b';brown='#944b23'
W,H=960,572
def text(draw,xy,s,size,color=ink,bold=False):draw.text(xy,s,font=font(size,bold),fill=color)
def tile(pic,xy,label,path,saddle,source=False):
    x,y=xy;draw=ImageDraw.Draw(pic);draw.rounded_rectangle((x,y,x+446,y+174),8,fill=bg,outline='#bacac2')
    if path is None:
        text(draw,(x+23,y+74),'No assembly exported',22,brown,True)
    else:
        geom=meshes(path,saddle)
        pts=np.concatenate([g[0] for g in geom]);c=(pts.min(axis=0)+pts.max(axis=0))/2
        if not source:c=np.array([4,-6,23]) if saddle else np.array([0,0,15])
        camera=(.69,-1.06,.98) if saddle else (.68,-1.05,.83)
        size=(298,148);scale=2.1 if saddle else 2.75
        if source:scale=min(scale,125/max(24,np.linalg.norm(np.ptp(pts,axis=0))*.66))
        img=render(geom,camera,scale=scale,center=c,size=size);pic.paste(img,(x+116,y+14))
    text(draw,(x+15,y+11),label,19,ink,True)
    text(draw,(x+15,y+140),'INPUT' if source else ('VERIFIED' if path else 'REJECTED'),13,teal if path else brown,True)
frames=[]
for index,(key,(title,subtitle)) in enumerate(rendering.items()):
    saddle=index<5;input_id=orders[index]['source'] if saddle else key
    path=field/'inputs'/(input_id+'.step') if saddle else fixtures[key]
    frame=Image.new('RGB',(W,H),bg);d=ImageDraw.Draw(frame)
    text(d,(25,15),'COLOCAD   /   ENGINEERING CHANGE ORDERS',15,teal,True)
    text(d,(770,15),f'{index+1:02d} / 10',16,ink,True)
    text(d,(25,43),title,34,ink,True);text(d,(25,91),subtitle,17)
    text(d,(25,125),'KK60  /  camera adapter' if saddle else 'MOTOR FLANGE  /  keyed mount',15,teal,True)
    tile(frame,(25,159),'Source STEP',path,saddle,True)
    for b,label,xy in zip(backends,names,[(489,159),(25,348),(489,348)]):tile(frame,xy,label,decisions[b][key],saddle)
    d=ImageDraw.Draw(frame);text(d,(25,538),'Independent STEP readbacks · Input centered for viewing · Playback follows work orders',15)
    frames.append(frame)
a.destination.parent.mkdir(parents=True,exist_ok=True)
frames[0].save(a.destination,save_all=True,append_images=frames[1:],duration=[2050]*len(frames),loop=0,optimize=True,disposal=2)
