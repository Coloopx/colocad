"""Independent OCCT STEP readback. Run using Python with cadquery-ocp installed.
Checks every saved successful revision, source hashes, solid validity, integrated
volume, catalog interfaces, material at countersinks/bores/helical grooves and
selected rail/carriage clearances. This is not manufacturer certification.
"""
import json,sys,hashlib,math
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from session_steps import session_steps
from OCP.STEPControl import STEPControl_Reader
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.TopAbs import TopAbs_SOLID,TopAbs_IN,TopAbs_OUT
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS
from OCP.BRepClass3d import BRepClass3d_SolidClassifier
from OCP.BRepAlgoAPI import BRepAlgoAPI_Common
from OCP.Bnd import Bnd_Box
from OCP.BRepBndLib import BRepBndLib
from OCP.gp import gp_Pnt,gp_Trsf,gp_Vec
from OCP.BRepBuilderAPI import BRepBuilderAPI_Transform

def volume(shape):
 p=GProp_GProps();BRepGProp.VolumeProperties_s(shape,p);return p.Mass()
def bounds(shape):
 b=Bnd_Box();BRepBndLib.AddOptimal_s(shape,b,False,False);return list(b.Get())
def near(a,b,tol=1e-5):assert abs(a-b)<tol,(a,b,tol)
def probe(s,p,inside):
 state=BRepClass3d_SolidClassifier(s,gp_Pnt(*p),1e-6).State();assert state==(TopAbs_IN if inside else TopAbs_OUT),(p,state,inside)
 return {'point':p,'material':inside}
root=Path(sys.argv[1]);results=[];shapes={}
for entry in session_steps(root):
 if entry['status']!='succeeded' or entry['operation'] in ('observe','query'):continue
 path=root/entry['artifactDirectory']/'model.step';receipt=json.loads((root/entry['receiptFile']).read_text())['data']
 digest=hashlib.sha256(path.read_bytes()).hexdigest();assert digest==receipt['artifact_sha256']['model.step']
 reader=STEPControl_Reader();assert int(reader.ReadFile(str(path)))==1;reader.TransferRoots();shape=reader.OneShape();solids=[];it=TopExp_Explorer(shape,TopAbs_SOLID)
 while it.More():solids.append(TopoDS.Solid_s(it.Current()));it.Next()
 assert len(solids)==receipt['geometry']['solids'],(entry['label'],len(solids));assert all(BRepCheck_Analyzer(s).IsValid() for s in solids),entry['label']
 v=sum(volume(s) for s in solids);expected=receipt['geometry']['volume_mm3'];relative=abs(v-expected)/max(expected,1);assert relative<1e-4,(entry['label'],relative)
 label=entry['label'];shapes[label]=shape;probes=[];bb=bounds(shape)
 if label=='Drill and counterbore the six mounting holes':
  near(bb[3]-bb[0],60);near(bb[5]-bb[2],300)
  for x in [-15,15]:
   for z in [57,157,257]:
    probes.extend([probe(solids[0],(x,.1,z),False),probe(solids[0],(x+4,.1,z),True),probe(solids[0],(x+4,2,z),False),probe(solids[0],(x+5,2,z),True)])
 if label=='Drill the M5 mounting pattern and datum slots':
  near(bb[4],33)
  for x in [-15,15]:
   for z in [-15,15]:probes.extend([probe(solids[0],(x,28,z),False),probe(solids[0],(x+3,28,z),True)])
 if label=='Cut 56 helical turns at 5 mm lead':
  for turn in [0,12,27,55]:
   for theta in [math.pi/4,3*math.pi/4,5*math.pi/4,7*math.pi/4]:
    z=5+turn*5+theta/(2*math.pi)*5
    probes.extend([probe(solids[0],(5.7*math.cos(theta),5.7*math.sin(theta),z),False),probe(solids[0],(5.7*math.cos(theta),5.7*math.sin(theta),(z+2.5 if z+2.5<286 else z-2.5)),True)])
 if label=='Complete the actuator and mounting hardware':near(bb[5]-bb[2],370);near(bb[3]-bb[0],60)
 results.append({'id':entry['id'],'label':label,'solids':len(solids),'valid':True,'volume_mm3':v,'native_volume_mm3':expected,'relative_volume_error':relative,'bbox_mm':bb,'step_sha256':digest,'materialProbes':probes})
 print(json.dumps({'checked':entry['id'],'solids':len(solids),'relativeVolumeError':relative}),flush=True)
# Check carriage body against rail at both travel endpoints (independent Boolean common).
rail=shapes['Drill and counterbore the six mounting holes'];carriage=shapes['Drill the M5 mounting pattern and datum slots'];clearances=[]
for travel in [-105,0,105]:
 t=gp_Trsf();t.SetTranslation(gp_Vec(0,0,157+travel));placed=BRepBuilderAPI_Transform(carriage,t,True).Shape();common=BRepAlgoAPI_Common(rail,placed);common.Build();assert common.IsDone();overlap=abs(volume(common.Shape()));assert overlap<1e-5,overlap
 clearances.append({'travelMm':travel,'railCarriageOverlapMm3':overlap,'scope':'machined carriage body vs rail only'})
result={'validator':'independent OCCT via cadquery-ocp','passed':len(results),'results':results,'clearances':clearances,'limitations':'Catalog-interface reconstruction; proprietary race profile, bearing load/preload, all-pairs interference and manufacturing tolerances are not certified.'}
Path(sys.argv[2]).write_text(json.dumps(result,indent=2)+'\n')
