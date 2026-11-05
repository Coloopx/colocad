"""Independent build123d builder for every operation in this hand; no input STEP/mesh."""
import copy, json, sys, time
from pathlib import Path
t0=time.perf_counter()
import build123d as b
t1=time.perf_counter()
model=json.loads(Path(sys.argv[1]).read_text())
def dim(x): return model['parameters'][x['param']] if isinstance(x,dict) else x
def profile(p):
    k=p['kind']
    if k=='circle': return b.Circle(dim(p['radius']))
    if k=='polygon': return b.Polygon(*map(tuple,p['points']),align=None)
    if k=='outline':
        a=p['start'];edges=[]
        for s in p['segments']:
            q=s['to']
            edges.append(b.Edge.make_line(a,q) if s['kind']=='line' else b.Edge.make_three_point_arc(a,s['via'],q))
            a=q
        return b.Face(b.Wire(edges))
    raise ValueError(k)
bodies={}
for f in model['features']:
    op=f['op']
    if op=='extrude':
        fr=f['frame']; x=b.Vector(*fr['x_axis']);y=b.Vector(*fr['y_axis'])
        obj=b.extrude(profile(f['profile']),amount=dim(f['height']),dir=(0,0,1)).moved(b.Plane(origin=fr['origin'],x_dir=x,z_dir=x.cross(y)).location)
    elif op=='box':
        obj=b.Box(*f['size'],align=(b.Align.MIN,)*3).translate(f['origin'])
    elif op=='cut': obj=bodies[f['left']].cut(bodies[f['right']])
    elif op=='pipe':
        p=f['path']; rail=b.Edge.make_three_point_arc(p['start'],p['via'],p['end'])
        plane=b.Plane(origin=rail.position_at(0),z_dir=rail.tangent_at(0))
        obj=b.sweep(plane*b.Circle(f['radius']),path=rail)
    else: raise ValueError(op)
    bodies[f['id']]=obj
instances=[]
for i in model['assembly']:
    o=copy.copy(bodies[i['input']]); rx,ry,rz=i['rotation_deg']
    for axis,angle in zip((b.Axis.X,b.Axis.Y,b.Axis.Z),(rx,ry,rz)):
        if angle: o=o.rotate(axis,angle)
    o=o.translate(i['translation']);o.label=i['id'];instances.append(o)
assembly=b.Compound(label='Hand',children=instances)
t2=time.perf_counter()
assert len(assembly.solids())==len(instances) and all(x.is_valid and x.volume>0 for x in instances)
t3=time.perf_counter()
assert b.export_step(assembly,sys.argv[2])
print(json.dumps({'solids':len(instances),'import_s':t1-t0,'model_s':t2-t1,'verify_s':t3-t2,'write_s':time.perf_counter()-t3}))
