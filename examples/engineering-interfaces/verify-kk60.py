"""Independent whole-product readback and the exact three requested part pairs/poses."""
from pathlib import Path
import sys,json,hashlib,math
from OCP.STEPControl import STEPControl_Reader
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.Bnd import Bnd_Box
from OCP.BRepBndLib import BRepBndLib
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_SOLID,TopAbs_EDGE
from OCP.TopoDS import TopoDS
from OCP.BRepAdaptor import BRepAdaptor_Curve
from OCP.GeomAbs import GeomAbs_Circle
from OCP.BRepExtrema import BRepExtrema_DistShapeShape
from OCP.BRepAlgoAPI import BRepAlgoAPI_Common
from OCP.BRepBuilderAPI import BRepBuilderAPI_Transform
from OCP.gp import gp_Trsf,gp_Vec
path=Path(sys.argv[1] if len(sys.argv)>1 else 'out/kk60-interfaces/run.json');run=json.loads(path.read_text());built=run['built']['data'];step=Path(built['artifact_directory'])/'model.step';digest=hashlib.sha256(step.read_bytes()).hexdigest();assert digest==built['artifact_sha256']['model.step']
reader=STEPControl_Reader();assert int(reader.ReadFile(str(step)))==1;reader.TransferRoots();shape=reader.OneShape();assert BRepCheck_Analyzer(shape).IsValid()
def solids(shape):
 result=[];e=TopExp_Explorer(shape,TopAbs_SOLID)
 while e.More():result.append(TopoDS.Solid_s(e.Current()));e.Next()
 return result
def bounds(s):
 b=Bnd_Box();BRepBndLib.AddOptimal_s(s,b,False,False);return list(b.Get())
def volume(s):
 p=GProp_GProps();BRepGProp.VolumeProperties_s(s,p,Eps=1e-10);return p.Mass()
ss=solids(shape);assert len(ss)==82
expected={'bracket':[-25,33,132,25,66,182],'carriage':[-21,8,131.5,21,33,182.5],'rail':[-30,0,7,30,22,307],'motor_support':[-29.8,.5,311,29.8,45,370]}
selected={}
for id,want in expected.items():
 found=[s for s in ss if max(abs(a-b) for a,b in zip(bounds(s),want))<.01];assert len(found)==1,(id,len(found));selected[id]=found[0]
assert abs(volume(selected['bracket'])-run['source']['volume_mm3'])<=max(1e-6,run['source']['volume_mm3']*1e-6)
for id,radius in [('bracket',2.75),('carriage',2.5)]:
 centers=[];e=TopExp_Explorer(selected[id],TopAbs_EDGE)
 while e.More():
  c=BRepAdaptor_Curve(TopoDS.Edge_s(e.Current()))
  if c.GetType()==GeomAbs_Circle and abs(c.Circle().Radius()-radius)<.01:
   p=c.Circle().Location();q=[p.X(),p.Y(),p.Z()]
   if abs(q[1]-33)<.01 and not any(math.dist(q,v)<.01 for v in centers):centers.append(q)
  e.Next()
 assert len(centers)==4,(id,centers)
 for x in [-15,15]:
  for z in [142,172]:assert any(math.dist([x,33,z],p)<.01 for p in centers),(id,centers)
rows=[]
for travel in [-105,0,105]:
 transform=gp_Trsf();transform.SetTranslation(gp_Vec(0,0,travel));a=BRepBuilderAPI_Transform(selected['bracket'],transform,True).Shape()
 for id in ['carriage','rail','motor_support']:
  b=BRepBuilderAPI_Transform(selected[id],transform,True).Shape() if id=='carriage' else selected[id]
  distance=BRepExtrema_DistShapeShape(a,b);distance.Perform();assert distance.IsDone();d=distance.Value()
  want=0 if id=='carriage' else 11 if id=='rail' else 129-travel
  assert abs(d-want)<.01,(id,travel,d,want)
  common=BRepAlgoAPI_Common(a,b);common.Build();assert common.IsDone();overlap=sum(volume(s) for s in solids(common.Shape()));assert overlap<=1e-6,(id,travel,overlap)
  rows.append({'pair':['bracket',id],'travel_mm':travel,'distance_mm':d,'expected_distance_mm':want,'overlap_mm3':overlap})
result={'passed':True,'verifier':'OCCT STEP independent geometry, no native face indices','source_step_sha256':digest,'solids':82,'bbox_mm':bounds(shape),'mount_hole_centers_verified':True,'checked_pairs':3,'pose_samples_per_pair':3,'samples':rows,'scope':'Only listed pairs and discrete poses; other product pairs, continuous motion, threads, load ratings and OEM accuracy are not certified.'}
(path.parent/'occt-readback.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'passed':True,'solids':82,'pairs':3,'samples':len(rows)}))
