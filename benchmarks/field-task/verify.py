"""Independent OCCT readback. No face IDs or producer receipts are used as geometric truth."""
import hashlib,json,math,sqlite3,sys
from pathlib import Path
from OCP.BRepAdaptor import BRepAdaptor_Curve
from OCP.BRepAlgoAPI import BRepAlgoAPI_Common
from OCP.BRepBndLib import BRepBndLib
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepClass3d import BRepClass3d_SolidClassifier
from OCP.BRepExtrema import BRepExtrema_DistShapeShape
from OCP.BRepGProp import BRepGProp
from OCP.Bnd import Bnd_Box
from OCP.GeomAbs import GeomAbs_Circle
from OCP.GProp import GProp_GProps
from OCP.STEPControl import STEPControl_Reader
from OCP.TopAbs import TopAbs_SOLID,TopAbs_EDGE,TopAbs_IN,TopAbs_OUT
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS
from OCP.gp import gp_Pnt
def walk(s,what):
    e=TopExp_Explorer(s,what)
    while e.More():
        yield e.Current();e.Next()
def load(path):
    r=STEPControl_Reader();assert int(r.ReadFile(str(path)))==1 and r.TransferRoots()>0
    shape=r.OneShape();assert BRepCheck_Analyzer(shape).IsValid(),path
    return [TopoDS.Solid_s(s) for s in walk(shape,TopAbs_SOLID)]
def vol(s):
    p=GProp_GProps();BRepGProp.VolumeProperties_s(s,p,Eps=1e-10);return p.Mass()
def bounds(s):
    b=Bnd_Box();BRepBndLib.AddOptimal_s(s,b,False,False);return list(b.Get())
def ring(s,x,y,z,r):
    for e in walk(s,TopAbs_EDGE):
        curve=BRepAdaptor_Curve(TopoDS.Edge_s(e))
        if curve.GetType()!=GeomAbs_Circle:continue
        c=curve.Circle()
        if abs(c.Radius()-r)<.01 and abs(c.Axis().Direction().Y())>.9999 and math.dist(c.Location().Coord(),(x,y,z))<.01:return True
    return False
def state(s,p,expected):
    assert BRepClass3d_SolidClassifier(s,gp_Pnt(*p),1e-6).State()==expected,(p,expected)
def verify(path,source,id,x,z,l,g,p,window):
    solids=load(path);assert len(solids)==2
    saddle,plate=sorted(solids,key=lambda s:bounds(s)[1])
    assert max(abs(a-b) for a,b in zip(bounds(plate),[x-22,33,z-l/2,x+22,33+g,z+l/2]))<.01
    src=load(source)[0]
    assert abs(vol(saddle)-vol(src))<.01
    expected=g*(44*l-4*math.pi*2.75**2-2*math.pi*1.7**2-(32 if window else 0))
    v=vol(plate);assert abs(v-expected)<.01
    for u in (-15,15):
        for w in (-15,15):
            assert ring(saddle,x+u,33,z+w,2.5) and ring(saddle,x+u,25,z+w,2.5)
            assert ring(plate,x+u,33,z+w,2.75) and ring(plate,x+u,33+g,z+w,2.75)
            state(plate,(x+u,33+g/2,z+w),TopAbs_OUT)
            state(plate,(x+u+3.1,33+g/2,z+w),TopAbs_IN)
            state(saddle,(x+u,27,z+w),TopAbs_OUT);state(saddle,(x+u,24.5,z+w),TopAbs_IN)
    for sign in (-1,1):
        assert ring(plate,x+sign*p/2,33,z,1.7) and ring(plate,x+sign*p/2,33+g,z,1.7)
    state(plate,(x,33+g/2,z+7),TopAbs_OUT if window else TopAbs_IN)
    state(plate,(x+4.5,33+g/2,z+7),TopAbs_IN)
    d=BRepExtrema_DistShapeShape(saddle,plate);d.Perform();assert d.IsDone() and d.Value()<.01
    op=BRepAlgoAPI_Common(saddle,plate);op.Build();assert op.IsDone()
    overlap=sum(vol(s) for s in walk(op.Shape(),TopAbs_SOLID));assert overlap<1e-6
    return dict(id=id,validSolids=2,plateVolumeMm3=v,mounts=4,cameraBores=2,window=window,contactGapMm=d.Value(),overlapMm3=overlap,sha256=hashlib.sha256(path.read_bytes()).hexdigest())
out=Path(sys.argv[1]); result={}
if len(sys.argv)>3 and sys.argv[3]=='edits':
    root=out/'history';data=json.loads((root/'result.json').read_text())
    expected_edits=int(sys.argv[4]) if len(sys.argv)>4 else 64
    assert expected_edits>0 and data['historyEdits']==len(data['editSteps'])==expected_edits
    db=next((root/'session').glob('*.sqlite'))
    con=sqlite3.connect(db.resolve().as_uri()+'?immutable=1',uri=True)
    edits=[]
    for i,r in enumerate(data['editSteps'],1):
        seq=i+4;e=json.loads(con.execute('select entry from steps where seq=?',(seq,)).fetchone()[0])
        assert e['id']==r['id'] and e['operation']=='edit' and e['status']=='succeeded'
        g=5+((i-1)%5)*.5;p=16+((i-1)//5)*.25
        assert (g,p)==(r['gauge'],r['cameraPitch'])
        report=verify(root/'session'/e['artifactDirectory']/'model.step',out/'inputs'/'relocated.step',e['id'],7,11,54,g,p,True)
        assert report['sha256']==r['sha256'];edits.append(report)
    assert len({e['sha256'] for e in edits})==expected_edits
    Path(sys.argv[2]).write_text(json.dumps(edits,indent=2)+'\n');sys.exit()
for backend in ('colocad','cadquery','build123d'):
    root=out/backend
    rejected=[r for r in json.loads((root/'result.json').read_text())['receipts'] if 'reason' in r]
    assert rejected==[dict(id='missing',reason='MOUNT_CARDINALITY',observed=3)] and not (root/'missing.step').exists()
    rows=[]
    for id,x,z,l,g,p,window in [('install',0,0,44,5,16,False),('resize',0,0,54,7,22,False),('service',0,0,54,7,22,True),('replace',7,11,54,7,22,True)]:
        rows.append(verify(root/(id+'.step'),out/'inputs'/('relocated.step' if id=='replace' else 'nominal.step'),id,x,z,l,g,p,window))
    result[backend]=dict(steps=rows,rejected=rejected)
Path(sys.argv[2]).write_text(json.dumps(result,indent=2)+'\n')
