"""Independent OCCT readback of every native STEP in a recorded engine build.
Usage: python verify.py /path/to/session /path/to/verification.json
Requires cadquery-ocp; it is a development verifier, not a runtime dependency.
"""
import json,math,sys,hashlib
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
from OCP.gp import gp_Pnt
root=Path(sys.argv[1]);results=[]
for entry in session_steps(root):
    if entry['status']!='succeeded' or entry['operation']=='observe':continue
    directory=root/entry['artifactDirectory'];path=directory/'model.step';receipt=json.loads((root/entry['receiptFile']).read_text())['data']
    digest=hashlib.sha256(path.read_bytes()).hexdigest();assert digest==receipt['artifact_sha256']['model.step']
    reader=STEPControl_Reader();assert int(reader.ReadFile(str(path)))==1;reader.TransferRoots();shape=reader.OneShape()
    solids=[];it=TopExp_Explorer(shape,TopAbs_SOLID)
    while it.More():solids.append(TopoDS.Solid_s(it.Current()));it.Next()
    assert len(solids)==receipt['geometry']['solids'],(entry['label'],len(solids))
    assert all(BRepCheck_Analyzer(s).IsValid() for s in solids),entry['label']
    volume=0
    for solid in solids:
        props=GProp_GProps();BRepGProp.VolumeProperties_s(solid,props);volume+=props.Mass()
    expected=receipt['geometry']['volume_mm3'];relative=abs(volume-expected)/max(expected,1)
    assert relative<1e-4,(entry['label'],volume,expected,relative)
    probes=[]
    if entry['label']=='Bore the fan hub':
        for p,want in [((0,0,10),TopAbs_OUT),((20,0,10),TopAbs_IN)]:
            actual=BRepClass3d_SolidClassifier(solids[0],gp_Pnt(*p),1e-6).State();assert actual==want;probes.append({'point':p,'state':'inside' if want==TopAbs_IN else 'outside'})
    if entry['label']=='Cut the radial dilution ports':
        for i in range(16):
            theta=i*math.pi/8;p=(67*math.cos(theta),67*math.sin(theta),267);want=TopAbs_OUT if i%2==0 else TopAbs_IN
            actual=BRepClass3d_SolidClassifier(solids[0],gp_Pnt(*p),1e-6).State();assert actual==want,(i,actual,want);probes.append({'point':p,'state':'inside' if want==TopAbs_IN else 'outside'})
    results.append({'id':entry['id'],'label':entry['label'],'solids':len(solids),'valid':True,'volume_mm3':volume,'native_volume_mm3':expected,'relative_volume_error':relative,'step_sha256':digest,'probes':probes})
result={'validator':'OCCT via cadquery-ocp','scope':'independent STEP topology, solid count, volume and critical bore/liner material probes; no aerodynamic or clearance certification','passed':len(results),'results':results}
Path(sys.argv[2]).write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'passed':len(results),'finalSolids':results[-1]['solids'],'maxRelativeVolumeError':max(x['relative_volume_error'] for x in results)}))
