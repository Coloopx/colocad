"""Independent readback of actual live-agent session outputs, not model claims."""
from pathlib import Path
import sys,json,hashlib,math
sys.path.insert(0, str(Path(__file__).resolve().parents[3]/'examples'))
from session_steps import session_steps
from OCP.STEPControl import STEPControl_Reader
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.Bnd import Bnd_Box
from OCP.BRepBndLib import BRepBndLib
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_SOLID,TopAbs_EDGE
from OCP.TopoDS import TopoDS
from OCP.BRepAdaptor import BRepAdaptor_Curve
from OCP.GeomAbs import GeomAbs_Circle
from OCP.BRepExtrema import BRepExtrema_DistShapeShape
from OCP.BRepAlgoAPI import BRepAlgoAPI_Common
root=Path(sys.argv[1]);summary=json.loads((root/'summary.json').read_text());assert summary['passed'];session=root/'session';steps=[s for s in session_steps(session) if s['status']=='succeeded' and s['operation'] not in ['query','observe']];assert len(steps)==2
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def items(shape,kind):
 result=[];e=TopExp_Explorer(shape,kind)
 while e.More():result.append(e.Current());e.Next()
 return result
def volume(s):
 p=GProp_GProps();BRepGProp.VolumeProperties_s(s,p,Eps=1e-10);return p.Mass()
def bounds(s):
 b=Bnd_Box();BRepBndLib.AddOptimal_s(s,b,False,False);return list(b.Get())
def close(a,b,tol):assert max(abs(x-y)for x,y in zip(a,b))<=tol,(a,b,tol)
rows=[]
for index,step in enumerate(steps):
 receiptPath=session/step['receiptFile'];assert sha(receiptPath)==step['receiptSha256'];receipt=json.loads(receiptPath.read_text());modelPath=session/step['inputFile'];assert sha(modelPath)==step['inputSha256'];model=json.loads(modelPath.read_text());file=session/step['artifactDirectory']/'model.step';assert sha(file)==receipt['data']['artifact_sha256']['model.step']
 reader=STEPControl_Reader();assert int(reader.ReadFile(str(file)))==1;reader.TransferRoots();shape=reader.OneShape();assert BRepCheck_Analyzer(shape).IsValid();solids=items(shape,TopAbs_SOLID);v=sum(volume(s)for s in solids);row={'step_id':step['id'],'solids':len(solids),'volume_mm3':v,'step_sha256':sha(file),'valid':True}
 if summary['mode']=='plate':
  radius=3 if index==0 else 3.5;assert len(solids)==1;assert model['parameters']['holeRadius']==radius;close(bounds(shape),[0,0,0,80,50,6],1e-5);expected=24000-4*math.pi*radius*radius*6;assert abs(v-expected)<expected*1e-6
  centers=[]
  for edge in items(shape,TopAbs_EDGE):
   c=BRepAdaptor_Curve(TopoDS.Edge_s(edge))
   if c.GetType()==GeomAbs_Circle and abs(c.Circle().Radius()-radius)<1e-5:
    p=c.Circle().Location();point=[p.X(),p.Y(),p.Z()]
    if abs(p.Z()-6)<1e-5 and not any(math.dist(point,q)<1e-5 for q in centers):centers.append(point)
  assert len(centers)==4,centers
  for x,y in [(10,10),(70,10),(10,40),(70,40)]:assert any(math.dist([x,y,6],p)<1e-5 for p in centers)
  row.update(radius_mm=radius,measured_hole_centers_mm=centers)
 else:
  travel=0 if index==0 else 20;assert model['parameters']['travel']==travel;assert len(solids)==82
  expected={'bracket':[-25,33,132+travel,25,66,182+travel],'carriage':[-21,8,131.5+travel,21,33,182.5+travel],'motor':[-29.8,.5,311,29.8,45,370]};selected={}
  for name,want in expected.items():
   found=[s for s in solids if max(abs(x-y)for x,y in zip(bounds(s),want))<.01];assert len(found)==1,(name,len(found));selected[name]=found[0]
  measurements=[]
  for name,want in [('carriage',0),('motor',129-travel)]:
   a=selected['bracket'];b=selected[name];d=BRepExtrema_DistShapeShape(a,b);d.Perform();assert d.IsDone();assert abs(d.Value()-want)<.01
   op=BRepAlgoAPI_Common(a,b);op.Build();assert op.IsDone();overlap=sum(abs(volume(s))for s in items(op.Shape(),TopAbs_SOLID));assert overlap<=1e-6;measurements.append({'pair':['bracket',name],'distance_mm':d.Value(),'overlap_mm3':overlap})
  row.update(travel_mm=travel,pairs=measurements)
 rows.append(row)
if summary['mode']=='product':assert abs(rows[1]['volume_mm3']-rows[0]['volume_mm3'])<rows[0]['volume_mm3']*1e-6
result={'passed':True,'mode':summary['mode'],'verifier':'OCCT 7.9.3.1.1 independent STEP readback','rows':rows,'scope':'Live text-to-parametric plate and live edit/query of a seeded sourced product; not image-to-CAD inference or OEM/continuous-motion certification.'};(root/'occt-readback.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'passed':True,'mode':summary['mode'],'verified_steps':len(rows)}))
