"""Independent OCCT geometric acceptance. Optional development verifier only."""
import argparse, hashlib, importlib.metadata, json
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.BRepExtrema import BRepExtrema_DistShapeShape
from OCP.GProp import GProp_GProps
from OCP.TopAbs import TopAbs_SOLID
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS
p=argparse.ArgumentParser();p.add_argument('run',type=Path);args=p.parse_args()
run=json.loads(args.run.read_text());step=args.run.parent/run['artifactDirectory']/'model.step'
assert hashlib.sha256(step.read_bytes()).hexdigest()==run['stepSha256']
r=STEPControl_Reader();assert int(r.ReadFile(str(step)))==1;assert r.TransferRoots()>0
shape=r.OneShape();assert BRepCheck_Analyzer(shape).IsValid()
solids=[];it=TopExp_Explorer(shape,TopAbs_SOLID)
while it.More():
    solids.append(TopoDS.Solid_s(it.Current()));it.Next()
assert len(solids)==2
volumes=[]
for s in solids:
    assert BRepCheck_Analyzer(s).IsValid();props=GProp_GProps();BRepGProp.VolumeProperties_s(s,props);volumes.append(props.Mass())
assert all(abs(a-b)<1e-4 for a,b in zip(sorted(volumes),sorted(run['expectedVolumesMm3'])))
d=BRepExtrema_DistShapeShape(*solids);d.Perform();assert d.IsDone();assert abs(d.Value()-run['expectedMinimumDistanceMm'])<1e-5
result={'passed':True,'ocp_version':importlib.metadata.version('cadquery-ocp-novtk'),'step_sha256':run['stepSha256'],'valid_solids':2,'volumes_mm3':sorted(volumes),'minimum_distance_mm':d.Value(),'expected_minimum_distance_mm':run['expectedMinimumDistanceMm'],'scope':'foreign analytic STEP, geometry-derived datum mates and exact readback; not general interface/replacement or industrial corpus acceptance'}
(args.run.parent/'occt-readback.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
