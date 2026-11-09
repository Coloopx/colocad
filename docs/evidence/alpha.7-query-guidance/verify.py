from pathlib import Path
import json,math,hashlib
import OCP
from importlib.metadata import version
from OCP.STEPControl import STEPControl_Reader
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.BRepAlgoAPI import BRepAlgoAPI_Cut
from OCP.BRepBndLib import BRepBndLib
from OCP.Bnd import Bnd_Box
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_SOLID
w=Path.cwd();probes=json.loads((w/'probe.json').read_text());ledger=json.loads((w/'query-session/session.json').read_text())
plain=next(p for p in probes if p['id']=='plain')['result']['data']
paths={'grid':Path(plain['artifact_directory'])/'model.step'}
for label,i in [('holes',0),('edited',2)]:paths[label]=w/'query-session'/ledger['steps'][i]['artifactDirectory']/'model.step'
def load(path):
 r=STEPControl_Reader();assert int(r.ReadFile(str(path)))==1;r.TransferRoots();s=r.OneShape();assert BRepCheck_Analyzer(s).IsValid();return s
def volume(s):
 p=GProp_GProps();BRepGProp.VolumeProperties_s(s,p,Eps=1e-10);return p.Mass()
shapes={k:load(p) for k,p in paths.items()};rows=[]
for k,s in shapes.items():
 e=TopExp_Explorer(s,TopAbs_SOLID);count=0
 while e.More():count+=1;e.Next()
 assert count==1
 radius=4 if k=='edited' else 3;v=volume(s);assert abs(v-(80*50-4*math.pi*radius**2)*6)<1e-5
 bb=Bnd_Box();BRepBndLib.AddOptimal_s(s,bb);bounds=list(bb.Get());assert max(abs(a-b) for a,b in zip(bounds,[0,0,0,80,50,6]))<1e-5
 rows.append({'id':k,'valid':True,'solids':count,'volume_mm3':v,'bounds_mm':bounds,'step_sha256':hashlib.sha256(paths[k].read_bytes()).hexdigest()})
differences=[]
for a,b in [('grid','holes'),('holes','grid')]:
 cut=BRepAlgoAPI_Cut(shapes[a],shapes[b]);cut.Build();assert cut.IsDone();v=abs(volume(cut.Shape()));assert v<1e-6;differences.append({'a':a,'b':b,'material_difference_mm3':v})
result={'passed':True,'verifier':{'engine':'OCCT via OCP','binding_version':OCP.__version__,'distribution':'cadquery-ocp-novtk','distribution_version':version('cadquery-ocp-novtk')},'linear_tolerance_mm':1e-5,'volume_tolerance_mm3':1e-5,'symmetric_difference_tolerance_mm3':1e-6,'rows':rows,'differences':differences}
(w/'occt-guidance.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
