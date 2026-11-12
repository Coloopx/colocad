"""Geometry-driven keyed mounting frame in CadQuery or build123d, with the existing neutral fixture oracle."""
from pathlib import Path
import hashlib, json, sys
import numpy as np
backend,fixtures,dest=sys.argv[1:];fixtures=Path(fixtures);dest=Path(dest);dest.mkdir(exist_ok=True,parents=True)
if backend=='cadquery':
    import cadquery as cq
    def load(path):return cq.importers.importStep(str(path)).val()
    def rings(part):return [(e.radius(),np.array(e.arcCenter().toTuple()),np.array(e.normal().toTuple())) for e in part.Edges() if e.geomType()=='CIRCLE' and e.IsClosed()]
    def frame(c,u,n):return cq.Plane(tuple(c),tuple(u),tuple(n)).location
    def moved(part,a,b):return part.moved(a*b.inverse)
    def export(parts,path):cq.exporters.export(cq.Compound.makeCompound(parts),str(path))
elif backend=='build123d':
    import build123d as bd
    def load(path):return bd.import_step(str(path))
    def rings(part):return [(e.radius,np.array(tuple(e.arc_center)),np.array(tuple(e.normal()))) for e in part.edges() if e.geom_type==bd.GeomType.CIRCLE and e.is_closed]
    def frame(c,u,n):return bd.Plane(origin=tuple(c),x_dir=tuple(u),z_dir=tuple(n)).location
    def moved(part,a,b):return part.moved(a*b.inverse())
    def export(parts,path):assert bd.export_step(bd.Compound(children=parts),str(path))
else:raise ValueError(backend)
def unit(v):return v/np.linalg.norm(v)
class Incompatible(ValueError):pass
def interface(part,radius):
    circles=rings(part)
    def with_radius(r):return [(p,n) for size,p,n in circles if abs(size-r)<1e-5]
    keys=with_radius(1.2);registers=with_radius(5)
    pairs=[(k,c) for k,_ in keys for c,n in registers if abs(np.dot(k-c,n))<1e-5]
    assert len(pairs)==1 and len(keys)==2
    key,center=pairs[0]
    floor,=[p for p,_ in keys if np.linalg.norm(p-key)>1e-5]
    n=unit(key-floor);u=unit(key-center);v=np.cross(n,u)
    openings=[p for p,axis in with_radius(radius) if abs(np.dot(p-center,n))<1e-5 and abs(np.dot(axis,n))>.99999]
    if len(openings)!=4:raise Incompatible('INTERFACE_NOT_FOUND')
    projected=sorted((round(float(np.dot(p-center,u)),5),round(float(np.dot(p-center,v)),5)) for p in openings)
    if projected!=[(-12.,-10.),(-12.,10.),(12.,-10.),(12.,10.)]:raise Incompatible('INTERFACE_INCOMPATIBLE')
    return center,u,n
manifest=json.loads((fixtures/'manifest.json').read_text());cases={c['id']:c for c in manifest['cases']}
def asset(name):
    row=cases[name];path=fixtures/row['file']
    assert hashlib.sha256(path.read_bytes()).hexdigest()==row['sha256']
    return load(path)
support=asset('support');c,u,n=interface(support,2.2);destination=frame(c,u,-n)
results=[]
for name in ('motor','motor-housing','motor-rotated','motor-wrong-pitch','motor-missing-hole'):
    motor=asset(name);row=cases[name];result=dict(id=name,expectedVolumesMm3=[row['volume_mm3'],cases['support']['volume_mm3']])
    try:source=frame(*interface(motor,2))
    except Incompatible as error:
        assert not row['expected']['positive'] and str(error)==row['expected']['reject']
        result['built']=dict(ok=False,error=dict(code=str(error)))
    else:
        assert row['expected']['positive'];directory=dest/name;directory.mkdir();path=directory/'model.step'
        export([support,moved(motor,destination,source)],path)
        result.update(built=dict(ok=True),artifactDirectory=name,stepSha256=hashlib.sha256(path.read_bytes()).hexdigest())
    results.append(result)
(dest/'runs.json').write_text(json.dumps(dict(manifest=manifest,runs=results)))
