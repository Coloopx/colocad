"""Generate reference comparisons and sectional stills from STEP readbacks/real native triangles. Run after timing."""
import argparse,hashlib,json
from pathlib import Path
import numpy as np
import build123d as b
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from PIL import Image,ImageDraw
from projection import CAMERAS,render,project,axes,font,contact

p=argparse.ArgumentParser();p.add_argument('scenes',type=Path);p.add_argument('baseline_step',type=Path);p.add_argument('out',type=Path);p.add_argument('evidence',type=Path);a=p.parse_args()
a.out.mkdir(parents=True,exist_ok=True)
root=Path(__file__).resolve().parents[1]
bg='#f5f6f2';teal=[76,170,163];dark=[95,115,130];orange=[232,158,67];pale=[160,188,184]
def material(s):
    k=s.label.lower()
    return orange if ('cord' in k or 'tendon' in k) else [210,157,91] if ('pin'==k or '_pin' in k or 'axle' in k) else dark if ('tip' in k or 'endlink' in k) else teal if ('link' in k or 'cableguide' in k or '_middle' in k or '_proximal' in k) else pale
def meshes(shape,override=None):
    res=[]
    for s in shape.children or [shape]:
        verts,idx=s.tessellate(.11,.20)
        res.append((np.array([tuple(v) for v in verts]),np.array(idx),override or material(s)))
    return res
def exact_volume(shape):
    p=GProp_GProps();BRepGProp.VolumeProperties_s(shape.wrapped,p,1e-9,True,False);return p.Mass()
def zoom(ms):return render(ms,(1,-.90,.56),5.35,np.array([0,0,23]),(430,380))
def sread(name): return b.import_step(a.scenes/(name+'.step'))
stages=['wrist','palm','proximal','middle','tips','thumb','drive']
imgs={}
for s in stages+['baseline']:
    shape=b.import_step(a.baseline_step) if s=='baseline' else sread(s)
    m=meshes(shape);render(m,CAMERAS['oblique']).save(a.out/(s+'.png'))
    if s=='drive':
        ious={}
        for k,cam in CAMERAS.items():
            got=render(m,cam);imgs[k]=got
            ref=np.array(Image.open(root/'reference'/(k+'.png')).convert('RGB'))[:,:,0]<215
            mask=np.array(render(m,cam,mask=True))[:,:,0]<215
            ious[k]=round(float((mask&ref).sum()/(mask|ref).sum()),4)
        contact(imgs,'02   /   Reconstructed hand     ·     STEP readback').save(a.out/'output-four.png')
for s in ['ProximalBlank','ProximalTangPos','ProximalPinRoot','ProximalFork','ProximalPinEnd','ProximalCableGuide','guide-edit']:zoom(meshes(sread(s),teal)).save(a.out/(s+'.png'))
# Cutter and a longitudinal material section, derived from actual bodies.
tool=sread('ProximalCableGuideTool');stock=sread('ProximalPinEnd')
# Split a half-body to expose the axis of the real cutting cylinder.
half=stock.split(b.Plane.YZ,keep=b.Keep.BOTTOM);zoom(meshes(half,pale)+meshes(tool,orange)).save(a.out/'cutter.png')
final=sread('ProximalCableGuide');half=final.split(b.Plane.YZ,keep=b.Keep.BOTTOM);zoom(meshes(half,teal)).save(a.out/'cutaway.png')
metrics={}
for f in ['ProximalCableGuide','guide-edit']:
    part=sread(f);sec=b.section(part,section_by=b.Plane.XY.offset(23))
    circles=[e for e in sec.edges() if e.geom_type==b.GeomType.CIRCLE];assert len(circles)==1
    r=circles[0].radius
    # Actual section contour sampled from the STEP curves, not diagrammed from nominal radii.
    loops=[[(float(p.X),float(p.Y)) for t in np.linspace(0,1,321) for p in [w.position_at(float(t))]] for w in sec.wires()]
    metrics[f]={'diameter_mm':2*r,'volume_mm3':exact_volume(part),'loops':loops}
assert abs(metrics['ProximalCableGuide']['diameter_mm']-2.9)<1e-6 and abs(metrics['guide-edit']['diameter_mm']-3.3)<1e-6
(a.out/'loops.json').write_text(json.dumps({k:v.pop('loops') for k,v in metrics.items()}))
# Orthographic wireframe: actual triangles from the Rust preview mesh.
j=json.loads((a.scenes/'ProximalCableGuide.mesh.json').read_text())
m=j['meshes'][0][1];v=np.array(m['positions']);idx=np.array(m['triangles'])
w=project(v,(1,-.90,.56),10.7,np.array([0,0,23]),(860,760));pts=w[idx];im=Image.new('RGB',(860,760),bg);draw=ImageDraw.Draw(im)
for i in np.argsort(pts[:,:,2].mean(1),kind='stable'):
    draw.polygon([tuple(p) for p in pts[i,:,:2]],fill='#b4d6cf',outline='#356c71',width=1)
im.resize((430,380),Image.Resampling.LANCZOS).save(a.out/'wire.png')
evidence={'sections':metrics,'reference_silhouette_iou':ious,'individual_part_triangles':len(idx),'scenes':json.loads((a.scenes/'scenes.json').read_text())}
a.evidence.write_text(json.dumps(evidence,indent=2)+'\n')
