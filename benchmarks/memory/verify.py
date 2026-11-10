"""OCCT 7.9.3 readback + independent analytic invariants (outside timed cgroups)."""
import argparse,collections,gc,hashlib,json,math
from pathlib import Path
import cadquery as cq
from OCP.BRepAdaptor import BRepAdaptor_Surface
from OCP.GeomAbs import GeomAbs_Cylinder
p=argparse.ArgumentParser();p.add_argument('manifest',type=Path);p.add_argument('out',type=Path);a=p.parse_args()
rows=json.loads(a.manifest.read_text());results=[]
for row in rows:
    f=Path(row['path']);shape=cq.importers.importStep(str(f)).val();n=row['n'];family=row['family'];start=row.get('start',0)
    expected=[];vol=0
    if family in ('part','chain'):
        nx=math.ceil(math.sqrt(n));x=y=nx*4+4
        expected=[(4+4*(i%nx),4+4*(i//nx)) for i in range(n)];vol=(x*y-math.pi*n)*6;solids=1
    else:
        solids=n
        for i in range(start,start+n):
            dx,dy=(i%64)*25,(i//64)*25;x=20+(i*.0001 if family=='unique' else 0)
            vol+=(x*20-16*math.pi)*6
            expected.extend((dx+4+4*u,dy+4+4*v) for u in range(4) for v in range(4))
    actual=[]
    for face in shape.Faces():
        surf=BRepAdaptor_Surface(face.wrapped)
        if surf.GetType()==GeomAbs_Cylinder:
            cy=surf.Cylinder();assert abs(cy.Radius()-1)<1e-6
            axis=cy.Axis();assert abs(abs(axis.Direction().Z())-1)<1e-7
            actual.append((round(axis.Location().X(),5),round(axis.Location().Y(),5)))
    solid_shapes=shape.Solids()
    boxes=[s.BoundingBox() for s in solid_shapes];boxes.sort(key=lambda b:(round(b.ymin,5),round(b.xmin,5)))
    for j,bb in enumerate(boxes):
        if family in ('part','chain'):bounds=[0,0,0,x,y,6]
        else:
            i=start+j;dx,dy=(i%64)*25,(i//64)*25
            bounds=[dx,dy,0,dx+20+(i*.0001 if family=='unique' else 0),dy+20,6]
        assert all(abs(u-v)<1e-5 for u,v in zip([bb.xmin,bb.ymin,bb.zmin,bb.xmax,bb.ymax,bb.zmax],bounds)),(row['id'],'solid placement/dimensions')
    # Validate each STEP-read solid, avoiding the same whole-compound temporary
    # allocation under investigation. Check all bore axes and total count/volume.
    measured=sum(s.Volume() for s in solid_shapes);valid=all(s.isValid() for s in solid_shapes);count=len(solid_shapes)
    assert valid and count==solids,(row['id'],valid,count,solids)
    assert math.isclose(measured,vol,rel_tol=1e-7,abs_tol=1e-5),(row['id'],measured,vol)
    assert collections.Counter(actual)==collections.Counter((round(x,5),round(y,5)) for x,y in expected),(row['id'],'bore axes differ',len(actual),len(expected))
    h=hashlib.sha256()
    with f.open('rb') as stream:
        for b in iter(lambda:stream.read(1024*1024),b''):h.update(b)
    result={'id':row['id'],'passed':True,'sha256':h.hexdigest(),'step_bytes':f.stat().st_size,'solids':count,'bores':len(actual),'volume_mm3':measured,'expected_volume_mm3':vol,'checks':['every_solid_valid','every_solid_bounds_and_placement','solid_count','analytic_volume','every_bore_radius_axis_center']}
    results.append(result);a.out.write_text(json.dumps(results,indent=2)+'\n');print(row['id'],'PASS',flush=True)
    del shape,solid_shapes;gc.collect()
