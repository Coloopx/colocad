"""Independent OCCT STEP readback. Python is verifier-only, never a Colocad runtime dependency."""
from pathlib import Path
import sys,json,hashlib,math
from OCP.STEPControl import STEPControl_Reader
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_SOLID,TopAbs_EDGE
from OCP.TopoDS import TopoDS
from OCP.BRepAdaptor import BRepAdaptor_Curve
from OCP.GeomAbs import GeomAbs_Circle
from OCP.BRepExtrema import BRepExtrema_DistShapeShape
from OCP.BRepAlgoAPI import BRepAlgoAPI_Common
path=Path(sys.argv[1] if len(sys.argv)>1 else 'out/engineering-interfaces/runs.json');data=json.loads(path.read_text());tol=data['manifest']['tolerance'];results=[]
def solids(shape):
 out=[];e=TopExp_Explorer(shape,TopAbs_SOLID)
 while e.More():out.append(TopoDS.Solid_s(e.Current()));e.Next()
 return out
def volume(shape):
 p=GProp_GProps();BRepGProp.VolumeProperties_s(shape,p,Eps=1e-10);return p.Mass()
for run in data['runs']:
 if not run['built']['ok']:continue
 step=path.parent/run['artifactDirectory']/'model.step';assert hashlib.sha256(step.read_bytes()).hexdigest()==run['stepSha256']
 reader=STEPControl_Reader();assert int(reader.ReadFile(str(step)))==1;reader.TransferRoots();shape=reader.OneShape();ss=solids(shape);assert len(ss)==2;assert BRepCheck_Analyzer(shape).IsValid()
 actual=sorted(volume(s) for s in ss);expected=sorted(run['expectedVolumesMm3'])
 for got,want in zip(actual,expected):assert abs(got-want)<=max(1e-6,abs(want)*tol['relative_volume']),(run['id'],got,want)
 distance=BRepExtrema_DistShapeShape(ss[0],ss[1]);distance.Perform();assert distance.IsDone();assert distance.Value()<=tol['linear_mm']
 common=BRepAlgoAPI_Common(ss[0],ss[1]);common.Build();assert common.IsDone();overlap=sum(volume(s) for s in solids(common.Shape()));assert overlap<=1e-7,(run['id'],overlap)
 centers={2:[],2.2:[]};e=TopExp_Explorer(shape,TopAbs_EDGE)
 while e.More():
  c=BRepAdaptor_Curve(TopoDS.Edge_s(e.Current()))
  if c.GetType()==GeomAbs_Circle:
   circle=c.Circle();p=circle.Location();radius=circle.Radius()
   for r in centers:
    if abs(radius-r)<=tol['linear_mm'] and abs(p.Z()-10)<=tol['linear_mm'] and not any(math.dist([p.X(),p.Y(),p.Z()],q)<=tol['linear_mm'] for q in centers[r]):centers[r].append([p.X(),p.Y(),p.Z()])
  e.Next()
 for radius,rows in centers.items():
  assert len(rows)==4,(run['id'],radius,rows)
  for x in [-12,12]:
   for y in [-10,10]:assert any(math.dist([x,y,10],p)<=tol['linear_mm'] for p in rows),(run['id'],radius,rows)
 results.append({'id':run['id'],'valid':True,'solids':2,'volumes_mm3':actual,'boundary_distance_mm':distance.Value(),'overlap_mm3':overlap,'measured_mount_centers_mm':centers,'step_sha256':run['stepSha256']})
assert len(results)==3
receipt={'verifier':'independent OCCT adaptive integration and boundary/boolean checks','tolerance':tol,'verified':results,'expected_rejections':[{ 'id':r['id'],'error':r['built']['error']['code']}for r in data['runs']if not r['built']['ok']]}
(path.parent/'occt-readback.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps({'passed':True,'positive':len(results),'negative':len(receipt['expected_rejections'])}))
