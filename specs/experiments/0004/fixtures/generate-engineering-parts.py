"""MIT: independent OCCT engineering fixtures with frozen analytic dimensions.
These are generic interface test parts, not reverse engineered commercial products.
"""
from pathlib import Path
import hashlib, importlib.metadata, json, math, re
from OCP.BRepPrimAPI import BRepPrimAPI_MakeBox,BRepPrimAPI_MakeCylinder,BRepPrimAPI_MakeCone
from OCP.BRepAlgoAPI import BRepAlgoAPI_Cut,BRepAlgoAPI_Fuse
from OCP.BRepBuilderAPI import BRepBuilderAPI_Transform,BRepBuilderAPI_NurbsConvert
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.gp import gp_Pnt,gp_Ax2,gp_Dir,gp_Ax1,gp_Trsf,gp_Vec
from OCP.STEPControl import STEPControl_Writer,STEPControl_AsIs
from OCP.BRepAdaptor import BRepAdaptor_Surface
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_FACE
from OCP.TopoDS import TopoDS
root=Path(__file__).resolve().parent/'engineering';root.mkdir(exist_ok=True)

def cyl(x,y,z,r,h):return BRepPrimAPI_MakeCylinder(gp_Ax2(gp_Pnt(x,y,z),gp_Dir(0,0,1)),r,h).Shape()
def boolean(cls,a,b):
 op=cls(a,b);op.Build();assert op.IsDone();return op.Shape()
def cut(a,b):return boolean(BRepAlgoAPI_Cut,a,b)
def fuse(a,b):return boolean(BRepAlgoAPI_Fuse,a,b)
def flange(pitch=24,missing=False,housing=False,thickness=8,diameter=4):
 body=BRepPrimAPI_MakeBox(gp_Pnt(-20,-18,0),40,36,thickness).Shape()
 if housing:body=fuse(body,BRepPrimAPI_MakeBox(gp_Pnt(-17,-15,-22),34,30,22).Shape())
 positions=[(-pitch/2,-10),(pitch/2,-10),(-pitch/2,10),(pitch/2,10)]
 if missing:positions.pop()
 for x,y in positions:body=cut(body,cyl(x,y,-23,diameter/2,thickness+24))
 body=cut(body,cyl(0,0,-23,5,thickness+24))
 # Unique blind key hole determines the installation side and in-plane direction.
 body=cut(body,cyl(16,0,thickness-2,1.2,3))
 return body
cases=[]
def save(id,body,expected):
 assert BRepCheck_Analyzer(body).IsValid(),id
 coarse=GProp_GProps();BRepGProp.VolumeProperties_s(body,coarse)
 props=GProp_GProps();integration_error=BRepGProp.VolumeProperties_s(body,props,Eps=1e-10)
 analytic=4800-108*math.pi if id in ['spline-bore','split-axial-bore','split-angular-bore'] else None
 if analytic is not None:assert abs(props.Mass()-analytic)<max(1e-6,analytic*1e-6)
 writer=STEPControl_Writer();assert int(writer.Transfer(body,STEPControl_AsIs))==1
 path=root/(id+'.step');assert int(writer.Write(str(path)))==1
 text=re.sub(r"'\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}'","'2026-11-09T00:00:00'",path.read_text(),count=1);path.write_text(text)
 types={};ex=TopExp_Explorer(body,TopAbs_FACE)
 while ex.More():
  typ=str(BRepAdaptor_Surface(TopoDS.Face_s(ex.Current())).GetType());types[typ]=types.get(typ,0)+1;ex.Next()
 cases.append({'id':id,'file':path.name,'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'volume_mm3':analytic if analytic is not None else props.Mass(),'analytic_volume_mm3':analytic,'adaptive_volume_mm3':props.Mass(),'default_integration_volume_mm3':coarse.Mass(),'adaptive_relative_error':integration_error,'surface_counts':types,'expected':expected})
base=flange();save('motor',base,{'pitch_mm':[24,20],'bore_diameter_mm':4,'key':[16,0,8],'mount_z_mm':8,'positive':True})
save('support',flange(thickness=10,diameter=4.4),{'pitch_mm':[24,20],'bore_diameter_mm':4.4,'mount_z_mm':10,'positive':True})
save('motor-housing',flange(housing=True),{'pitch_mm':[24,20],'bore_diameter_mm':4,'mount_z_mm':8,'positive':True})
rotation=gp_Trsf();rotation.SetRotation(gp_Ax1(gp_Pnt(0,0,0),gp_Dir(0.3,0.7,0.2)),1.1);rotation.SetTranslationPart(gp_Vec(50,-30,20))
save('motor-rotated',BRepBuilderAPI_Transform(base,rotation,True).Shape(),{'pitch_mm':[24,20],'bore_diameter_mm':4,'positive':True,'source_transform':[[rotation.Value(i,j) for j in range(1,5)] for i in range(1,4)]})
save('motor-wrong-pitch',flange(pitch=25),{'pitch_mm':[25,20],'bore_diameter_mm':4,'positive':False,'reject':'INTERFACE_INCOMPATIBLE'})
save('motor-missing-hole',flange(missing=True),{'positive':False,'reject':'INTERFACE_NOT_FOUND'})
block=BRepPrimAPI_MakeBox(gp_Pnt(-10,-10,0),20,20,12).Shape()
axial=cut(cut(block,cyl(0,0,-1,3,7)),cyl(0,0,6,3,7));save('split-axial-bore',axial,{'holes':1,'depth_mm':12,'diameter_mm':6,'positive':True})
angular=block
for angle in [0,math.pi]:
 direction=gp_Dir(math.cos(angle),math.sin(angle),0);tool=BRepPrimAPI_MakeCylinder(gp_Ax2(gp_Pnt(0,0,-1),gp_Dir(0,0,1),direction),3,14,math.pi).Shape();angular=cut(angular,tool)
save('split-angular-bore',angular,{'holes':1,'depth_mm':12,'diameter_mm':6,'positive':True})
plain=cut(block,cyl(0,0,-1,3,14));save('spline-bore',BRepBuilderAPI_NurbsConvert(plain,True).Shape(),{'query_status':'partial','reason':'unsupported_spline_equivalent','positive':False})
cone=BRepPrimAPI_MakeCone(gp_Ax2(gp_Pnt(0,0,3),gp_Dir(0,0,1)),0,3,3).Shape();save('conical-tip',cut(cut(block,cyl(0,0,6,3,7)),cone),{'query_status':'partial','positive':False})
cross=BRepPrimAPI_MakeCylinder(gp_Ax2(gp_Pnt(-11,0,6),gp_Dir(1,0,0)),2,22).Shape();save('intersecting-bores',cut(plain,cross),{'query_status':'partial','positive':False})
# Independent accessory bracket matching the sourced KK60 30 x 30 mm mount.
bracket=fuse(BRepPrimAPI_MakeBox(gp_Pnt(-25,-25,0),50,50,8).Shape(),BRepPrimAPI_MakeBox(gp_Pnt(-25,20,8),50,5,25).Shape())
for x in [-15,15]:
 for y in [-15,15]:bracket=cut(bracket,cyl(x,y,-1,2.75,10))
side=BRepPrimAPI_MakeCylinder(gp_Ax2(gp_Pnt(0,19,22),gp_Dir(0,1,0)),4,7).Shape();bracket=cut(bracket,side)
save('kk60-accessory-bracket',bracket,{'pitch_mm':[30,30],'bore_diameter_mm':5.5,'termination':'through','mount_z_mm':0,'positive':True,'purpose':'independent accessory example, not a manufacturer bracket SKU'})
manifest={'schema':'colocad.engineering-fixtures/1','license':'MIT','source':'independently authored OCCT geometry, generic mechanical interface cases','generator':'../generate-engineering-parts.py','ocp_version':importlib.metadata.version('cadquery-ocp-novtk'),'tolerance':{'linear_mm':1e-5,'angular_rad':1e-7,'relative_volume':1e-6},'cases':cases}
(root/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n');print(json.dumps({'cases':len(cases),'surface_counts':{c['id']:c['surface_counts'] for c in cases}}))
