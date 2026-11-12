"""Each backend imports the neutral STEP and measures its own mounting circles."""
import json, sys
from pathlib import Path
backend, source_dir, out = sys.argv[1:]
out = Path(out)
if backend == 'cadquery':
    import cadquery as cq
    def load(path): return cq.importers.importStep(str(path)).val()
    def circles(shape):
        return [(e.arcCenter().toTuple(), e.normal().toTuple()) for e in shape.Edges()
                if e.geomType() == 'CIRCLE' and abs(e.radius()-2.5)<.001 and e.IsClosed()]
    def box(x,y,z,dx,dy,dz): return cq.Solid.makeBox(dx,dy,dz,(x,y,z))
    def bore(x,y,z,r,h): return cq.Solid.makeCylinder(r,h,(x,y,z),(0,1,0))
    def export(parts,path): cq.exporters.export(cq.Compound.makeCompound(parts),str(path))
elif backend == 'build123d':
    import build123d as bd
    def load(path): return bd.import_step(str(path))
    def circles(shape):
        return [(tuple(e.arc_center),tuple(e.normal())) for e in shape.edges()
                if e.geom_type==bd.GeomType.CIRCLE and abs(e.radius-2.5)<.001 and e.is_closed]
    def box(x,y,z,dx,dy,dz): return bd.Solid.make_box(dx,dy,dz).translate((x,y,z))
    def bore(x,y,z,r,h): return bd.Solid.make_cylinder(r,h,bd.Plane(origin=(x,y,z),z_dir=(0,1,0)))
    def export(parts,path): assert bd.export_step(bd.Compound(children=parts),str(path))
else: raise ValueError(backend)

receipts=[]; prior=None
for o in json.loads(Path(__file__).with_name('orders.json').read_text()):
    if o['source'] != prior:
        saddle=load(Path(source_dir)/(o['source']+'.step'))
        cc=[p for p,axis in circles(saddle) if abs(axis[1])>.9999]
        y=max(p[1] for p in cc); rings=[(a,c) for a,b,c in cc if abs(b-y)<.01]
        if len(rings)!=4:
            receipts.append(dict(id=o['id'],reason='MOUNT_CARDINALITY',observed=len(rings)));continue
        xs,zs=zip(*rings);assert abs(max(xs)-min(xs)-30)<.01 and abs(max(zs)-min(zs)-30)<.01
        x=sum(xs)/4;z=sum(zs)/4;prior=o['source']
    g,l,p=o['gauge'],o['length'],o['cameraPitch']
    plate=box(x-22,y,z-l/2,44,g,l)
    cutters=[bore(u,y-1,v,2.75,g+2) for u,v in rings]+[bore(x+s*p/2,y-1,z,1.7,g+2) for s in (-1,1)]
    if o['window']:cutters.append(box(x-4,y-1,z+5,8,g+2,4))
    plate=plate.cut(*cutters)
    name=o['id']+'.step';export([saddle,plate],out/name)
    receipts.append(dict(id=o['id'],placed=[x,y,z],step=name))
assert [r.get('reason','STEP') for r in receipts]==['STEP','STEP','STEP','MOUNT_CARDINALITY','STEP']
(out/'result.json').write_text(json.dumps(dict(ok=True,receipts=receipts)))
