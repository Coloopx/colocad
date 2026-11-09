"""MIT-licensed analytic import fixture authored in OCCT, independently of Colocad."""
from pathlib import Path
import hashlib, importlib.metadata, json, math, re
from OCP.BRepPrimAPI import BRepPrimAPI_MakeBox, BRepPrimAPI_MakeCylinder
from OCP.BRepAlgoAPI import BRepAlgoAPI_Cut
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.gp import gp_Pnt, gp_Ax2, gp_Dir
from OCP.STEPControl import STEPControl_Writer, STEPControl_AsIs

root=Path(__file__).resolve().parent
body=BRepPrimAPI_MakeBox(gp_Pnt(-20,-20,0),40,40,8).Shape()
holes=[(-12,-12,2),(12,-12,2),(-12,12,2),(12,12,2),(0,0,5)]
for x,y,r in holes:
    cutter=BRepPrimAPI_MakeCylinder(gp_Ax2(gp_Pnt(x,y,-1),gp_Dir(0,0,1)),r,10).Shape()
    cut=BRepAlgoAPI_Cut(body,cutter);cut.Build();assert cut.IsDone();body=cut.Shape()
assert BRepCheck_Analyzer(body).IsValid()
props=GProp_GProps();BRepGProp.VolumeProperties_s(body,props)
expected=12800-328*math.pi;assert abs(props.Mass()-expected)<1e-6
writer=STEPControl_Writer();assert int(writer.Transfer(body,STEPControl_AsIs))==1
path=root/'occt-flange.step';assert int(writer.Write(str(path)))==1
text=path.read_text();text=re.sub(r"'\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}'","'2026-11-09T00:00:00'",text,count=1);path.write_text(text)
metadata={'id':'occt-analytic-flange-v1','license':'MIT','source':'independently authored analytic fixture; not a commercial product reconstruction','generator':'generate-occt-flange.py','ocp_version':importlib.metadata.version('cadquery-ocp-novtk'),'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'units':'mm','stock':[40,40,8],'holes_xy_radius':holes,'expected_volume_mm3':expected,'verified_volume_mm3':props.Mass(),'expected_top_area_mm2':1600-41*math.pi,'acceptance_tolerance_mm':1e-5}
(root/'occt-flange.json').write_text(json.dumps(metadata,indent=2)+'\n')
print(json.dumps(metadata))
