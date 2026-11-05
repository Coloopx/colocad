"""Independent validation of the STEP exported by the Rust build (requires cadquery-ocp)."""
import argparse,json,math
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.TopAbs import TopAbs_FACE,TopAbs_SOLID,TopAbs_IN,TopAbs_OUT
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS
from OCP.gp import gp_Pnt
from OCP.BRepClass3d import BRepClass3d_SolidClassifier
p=argparse.ArgumentParser()
p.add_argument('artifact',type=Path)
p.add_argument('--case',choices=['manifold','sensor-rail'],required=True)
a=p.parse_args()
r=STEPControl_Reader();assert int(r.ReadFile(str(a.artifact/'model.step')))==1
assert r.TransferRoots()==1
s=r.OneShape();assert BRepCheck_Analyzer(s).IsValid()
ex=TopExp_Explorer(s,TopAbs_SOLID);assert ex.More();solid=TopoDS.Solid_s(ex.Current());ex.Next();assert not ex.More()
props=GProp_GProps();BRepGProp.VolumeProperties_s(s,props)
receipt=json.loads((a.artifact/'report.json').read_text());v=props.Mass();native=receipt['geometry']['volume_mm3']
assert abs(v-native)<max(0.01,v*1e-5),(v,native)
tree=json.loads((a.artifact/'feature-tree.json').read_text());probes=[]
def check(x,y,z,expected):
    state=BRepClass3d_SolidClassifier(solid,gp_Pnt(x,y,z),1e-6).State()
    assert state==expected,(x,y,z,state,expected)
    probes.append([x,y,z,'inside' if expected==TopAbs_IN else 'outside'])
if a.case=='manifold':
    for i in range(tree['parameters']['instances']):
        x=14+i*tree['parameters']['pitch'];check(x,12,6,TopAbs_OUT);check(x,12,10,TopAbs_IN)
    check(43,32,2,TopAbs_OUT);check(43,32,6,TopAbs_IN)
    check(.05,24,11.95,TopAbs_OUT);check(2,24,11.95,TopAbs_IN)
else:
    for i in range(tree['parameters']['copies']):check(0,3+i*tree['parameters']['step'],4,TopAbs_OUT)
    check(9,3,4,TopAbs_IN)
print(json.dumps(dict(case=a.case,valid=True,solids=1,volumeMM3=v,rustVolumeMM3=native,materialProbes=probes)))
