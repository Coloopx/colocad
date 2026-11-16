"""[INPUT]: Win原生完整STEP控制的reference/seam17/export/readback原字节。
[OUTPUT]: 独立OCP有效性、实体/圆柱、体积/边界、材料点与双向差集回执。
[POS]: G2完整交付核验；不重建输入，不显式heal，不使用native几何结果作真值。
[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
"""
from pathlib import Path
import hashlib,json,math,platform,sys
if platform.system()!='Linux' or 'microsoft' not in platform.release().lower():
    raise SystemExit('WSL2_REQUIRED: no local OCP execution')
import OCP
from OCP.STEPControl import STEPControl_Reader
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.BRepBndLib import BRepBndLib
from OCP.Bnd import Bnd_Box
from OCP.BRepAlgoAPI import BRepAlgoAPI_Cut
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_SOLID,TopAbs_FACE,TopAbs_IN,TopAbs_OUT
from OCP.TopoDS import TopoDS
from OCP.BRepAdaptor import BRepAdaptor_Surface
from OCP.GeomAbs import GeomAbs_Cylinder
from OCP.BRepClass3d import BRepClass3d_SolidClassifier
from OCP.gp import gp_Pnt
root=Path(sys.argv[1]).resolve()
receipt={'schema':'g2.closed-solid.ocp.v1','status':'unknown','verifier':'independent OCP','binding_version':OCP.__version__,'platform':platform.platform(),'tolerances':{'volume_mm3':1e-5,'bounds_mm':1e-6,'material_difference_mm3':1e-6,'point_classification_mm':1e-7},'semantics':'standard STEP transfer may normalize; no explicit healing; equivalence means only the recorded independent numerical limits','rows':[],'differences':[]}
output=root/'ocp-closed-solid.json'
if output.exists(): raise SystemExit('Refuse overwriting an existing OCP receipt')
def volume(shape):
    g=GProp_GProps();BRepGProp.VolumeProperties_s(shape,g,Eps=1e-10);return g.Mass()
def solids(shape):
    e=TopExp_Explorer(shape,TopAbs_SOLID);items=[]
    while e.More():items.append(TopoDS.Solid_s(e.Current()));e.Next()
    return items
def read(name):
    path=root/name;r=STEPControl_Reader();assert int(r.ReadFile(str(path)))==1;assert r.TransferRoots()>0
    s=r.OneShape();assert not s.IsNull();return s,hashlib.sha256(path.read_bytes()).hexdigest()
shapes={}
try:
    native=json.loads((root/'native-reports.json').read_text())
    assert hashlib.sha256((root/'reference.step').read_bytes()).hexdigest()==native['original_fixture_sha256']
    receipt['native_reports_sha256']=hashlib.sha256((root/'native-reports.json').read_bytes()).hexdigest()
    for name in ['reference.step','seam17.step','export.step','readback.step']:
        s,digest=read(name);valid=BRepCheck_Analyzer(s,True).IsValid();assert valid
        items=solids(s);assert len(items)==1
        v=volume(s);assert abs(v-(3200.-40.*math.pi))<1e-5
        box=Bnd_Box();BRepBndLib.AddOptimal_s(s,box,False,False);bounds=list(box.Get())
        assert max(abs(a-b) for a,b in zip(bounds,[0,0,0,20,16,10]))<1e-6
        e=TopExp_Explorer(s,TopAbs_FACE);cylinders=[];face_count=0
        while e.More():
            face_count+=1;surface=BRepAdaptor_Surface(TopoDS.Face_s(e.Current()))
            if surface.GetType()==GeomAbs_Cylinder:
                c=surface.Cylinder();assert abs(c.Radius()-2.)<1e-7
                assert abs(c.Location().X()-10.)<1e-7 and abs(c.Location().Y()-8.)<1e-7
                assert abs(abs(c.Axis().Direction().Z())-1.)<1e-7
                cylinders.append({'radius_mm':c.Radius(),'axis_xy':[c.Location().X(),c.Location().Y()]})
            e.Next()
        assert cylinders
        points=[((10.,8.,5.),TopAbs_OUT),((11.9,8.,5.),TopAbs_OUT),((12.1,8.,5.),TopAbs_IN),((1.,1.,1.),TopAbs_IN),((21.,8.,5.),TopAbs_OUT)]
        for xyz,state in points:assert BRepClass3d_SolidClassifier(items[0],gp_Pnt(*xyz),1e-7).State()==state
        shapes[name]=s;receipt['rows'].append({'file':name,'sha256':digest,'valid':valid,'solids':len(items),'faces':face_count,'volume_mm3':v,'bbox_mm':bounds,'cylinders':cylinders,'material_air_probes':len(points)})
    for other in ['seam17.step','export.step','readback.step']:
        for left,right in [('reference.step',other),(other,'reference.step')]:
            cut=BRepAlgoAPI_Cut(shapes[left],shapes[right]);cut.Build();assert cut.IsDone()
            remainder=cut.Shape();assert not remainder.IsNull()
            difference=abs(volume(remainder));assert difference<1e-6
            receipt['differences'].append({'left':left,'right':right,'material_difference_mm3':difference})
    negative=json.loads((root/'negative-receipt.json').read_text());assert negative['expected_rejection'] is True
    receipt['native_negative_receipt_sha256']=hashlib.sha256((root/'negative-receipt.json').read_bytes()).hexdigest()
    receipt['status']='passed_with_recorded_numerical_limits'
except BaseException as error:
    receipt['status']='failed_or_incomplete';receipt['error']={'type':type(error).__name__,'message':str(error)}
    output.write_text(json.dumps(receipt,indent=2)+'\n');raise
output.write_text(json.dumps(receipt,indent=2)+'\n');print(output)
