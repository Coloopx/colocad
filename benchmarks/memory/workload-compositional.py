"""Control: validate each definition before rigid placement; retain full assembly STEP export."""
import argparse, json, math, os, time
from pathlib import Path

p=argparse.ArgumentParser();p.add_argument('backend');p.add_argument('family');p.add_argument('n',type=int)
p.add_argument('--start',type=int,default=0);p.add_argument('--out',default='/out');p.add_argument('--queryable',action='store_true')
a=p.parse_args();out=Path(a.out);out.mkdir(parents=True,exist_ok=True)
if a.backend=='colocad':
    os.execvp('node',['node','/bench/workload.mjs',a.family,str(a.n),str(a.start),str(out),str(a.queryable).lower()])
t=time.perf_counter()
def event(phase,**kw):
    with (out/'events.jsonl').open('a') as f:f.write(json.dumps({'phase':phase,'elapsed_s':time.perf_counter()-t,**kw})+'\n')
event('start')
if a.backend=='cadquery':import cadquery as cad
else:import build123d as cad
event('imported')
def dimensions(holes,i=0,assembly=False):
    nx=math.isqrt(holes);assert nx*nx==holes
    return (nx*4+4+(i*.0001 if assembly else 0),nx*4+4,[(4+x*4,4+y*4) for x in range(nx) for y in range(nx)])
def part(holes,i=0,assembly=False):
    x,y,points=dimensions(holes,i,assembly)
    if a.backend=='cadquery':
        # A perforated planar profile extruded once, no N serial Booleans.
        outer=cad.Wire.makePolygon([cad.Vector(*v) for v in [(0,0,0),(x,0,0),(x,y,0),(0,y,0)]],close=True)
        inner=[cad.Wire.makeCircle(1,cad.Vector(px,py,0),cad.Vector(0,0,1)) for px,py in points]
        return cad.Solid.extrudeLinear(outer,inner,cad.Vector(0,0,6))
    outer=cad.Wire.make_polygon([(0,0,0),(x,0,0),(x,y,0),(0,y,0)],close=True)
    inner=[cad.Wire.make_circle(1,cad.Plane(origin=(px,py,0))) for px,py in points]
    return cad.Solid.extrude(cad.Face(outer,inner),(0,0,6))
if a.family=='chain':
    nx=math.ceil(math.sqrt(a.n));x=y=nx*4+4
    if a.backend=='cadquery':
        body=cad.Workplane('XY').box(x,y,6,centered=False)
        for i in range(a.n):body=body.cut(cad.Solid.makeCylinder(1,8,cad.Vector(4+4*(i%nx),4+4*(i//nx),-1)))
        shape=body.val()
    else:
        body=cad.Box(x,y,6,align=(cad.Align.MIN,)*3)
        for i in range(a.n):body=body.cut(cad.Solid.make_cylinder(1,8,cad.Plane(origin=(4+4*(i%nx),4+4*(i//nx),-1))))
        shape=body
    expected=(x*y-a.n*math.pi)*6;solids=1
elif a.family=='part':shape=part(a.n);x,y,_=dimensions(a.n);expected=(x*y-a.n*math.pi)*6;solids=1
else:
    shared=part(16) if a.family=='repeated' else None
    shapes=[];expected=0;all_parts_valid=True;shared_valid=None;assembly=cad.Assembly(name='assembly') if a.backend=='cadquery' else None
    for i in range(a.start,a.start+a.n):
        b=shared if shared is not None else part(16,i,True)
        xyz=((i%64)*25,(i//64)*25,0)
        if shared_valid is None:
            valid_part=b.isValid() if a.backend=='cadquery' else b.is_valid
            if shared is not None:shared_valid=valid_part
        else:valid_part=shared_valid
        assert valid_part;all_parts_valid=all_parts_valid and valid_part
        if assembly is not None:assembly.add(b,name='i'+str(i),loc=cad.Location(xyz))
        else:
            b=b.moved(cad.Location(xyz));b.label='i'+str(i);shapes.append(b)
        x,y,_=dimensions(16,i,shared is None);expected+=(x*y-16*math.pi)*6
    shape=assembly.toCompound() if assembly is not None else cad.Compound(children=shapes,label='assembly')
    solids=a.n
event('geometry')
if a.backend=='cadquery':valid=all_parts_valid if a.family in ('unique','repeated') else shape.isValid();event('validity_checked');volume=shape.Volume();event('volume_measured');count=len(shape.Solids());event('solids_counted')
else:valid=all_parts_valid if a.family in ('unique','repeated') else shape.is_valid;event('validity_checked');volume=shape.volume;event('volume_measured');count=len(shape.solids());event('solids_counted')
assert valid and count==solids and math.isclose(volume,expected,rel_tol=1e-7,abs_tol=1e-5),(valid,count,volume,expected)
event('validated',solids=count,volume_mm3=volume)
if a.backend=='cadquery':
    if a.family in ('unique','repeated'):assembly.export(str(out/'model.step'))
    else:cad.exporters.export(shape,str(out/'model.step'))
else:assert cad.export_step(shape,str(out/'model.step'))
event('exported',step_bytes=(out/'model.step').stat().st_size)
(out/'result.json').write_text(json.dumps({'ok':True,'solids':count,'volume_mm3':volume,'expected_volume_mm3':expected,'step_bytes':(out/'model.step').stat().st_size,'step_path':'model.step'})+'\n')
