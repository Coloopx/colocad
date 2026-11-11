"""Independent CadQuery construction from the same rail, manifold or hand instructions."""
import json, sys, time
from pathlib import Path
start = time.perf_counter()
import cadquery as cq
loaded = time.perf_counter()
doc = json.loads(Path(sys.argv[1]).read_text())
def val(v):
    return doc['parameters'][v['param']] if isinstance(v,dict) else v
def profile(p, wp):
    k=p['kind']
    if k=='circle':return wp.circle(val(p['radius']))
    if k=='rectangle':return wp.rect(*map(val,p['size']))
    if k=='polygon':return wp.polyline(p['points']).close()
    if k=='outline':
        wp=wp.moveTo(*p['start'])
        for s in p['segments']:
            wp=wp.lineTo(*s['to']) if s['kind']=='line' else wp.threePointArc(s['via'],s['to'])
        return wp.close()
    raise ValueError(k)
def tree():
    body=None;seeds={}
    for f in doc['features']:
        op=f['op']
        if op in ('cut','extrude'):
            p=f['profile'];x,y=map(val,p.get('center',[0,0]))
            tool=profile(p,cq.Workplane('XY')).extrude(val(f['depth'])).val().translate((x,y,val(f.get('startZ',0))))
            seeds[f['id']]=(op,tool)
            body=tool if body is None else body.cut(tool) if op=='cut' else body.fuse(tool)
        elif op=='fillet':
            z=val(f['edges']['z']);edges=[e for e in body.Edges() if max(abs(v-z) for v in (e.BoundingBox().zmin,e.BoundingBox().zmax))<.001]
            assert len(edges)==val(f['edges']['count'])
            body=body.fillet(val(f['radius']),edges)
        elif op=='linearPattern':
            seed,tool=seeds[f['seed']];pitch=val(f['pitch'])
            copies=[tool.translate((i*pitch,0,0) if f['axis']=='x' else (0,i*pitch,0)) for i in range(1,val(f['count']))]
            body=body.cut(*copies) if seed=='cut' else body.fuse(*copies)
        else:raise ValueError(op)
    return body,None
def hand():
    bodies={};asm=cq.Assembly(name='Hand')
    for f in doc['features']:
        op=f['op']
        if op=='box':body=cq.Solid.makeBox(*f['size'],pnt=cq.Vector(*f['origin']))
        elif op=='cut':body=bodies[f['left']].cut(bodies[f['right']])
        elif op=='extrude':
            fr=f['frame'];x,y=(cq.Vector(*fr[k]) for k in ('x_axis','y_axis'));plane=cq.Plane(fr['origin'],x,x.cross(y))
            body=profile(f['profile'],cq.Workplane(plane)).extrude(val(f['height'])).val()
        elif op=='pipe':
            p=f['path'];e=cq.Edge.makeThreePointArc(*(cq.Vector(*p[k]) for k in ('start','via','end')))
            body=cq.Workplane(cq.Plane(origin=e.startPoint(),normal=e.tangentAt(0))).circle(val(f['radius'])).sweep(e).val()
        else:raise ValueError(op)
        bodies[f['id']]=body
    for i in doc['assembly']:
        loc=cq.Location(*i['translation'],*i['rotation_deg'])
        asm.add(bodies[i['input']],name=i['id'],loc=loc)
    return asm.toCompound(),asm
shape,assembly=hand() if 'assembly' in doc else tree()
built=time.perf_counter()
parts=shape.Solids();assert len(parts)==len(doc.get('assembly',[1])) and all(s.isValid() and s.Volume()>0 for s in parts)
checked=time.perf_counter()
if assembly:assembly.export(sys.argv[2])
else:cq.exporters.export(shape,sys.argv[2])
print(json.dumps({'solids':len(parts),'import_s':loaded-start,'model_s':built-loaded,'verify_s':checked-built,'write_s':time.perf_counter()-checked}))
