"""Independent OCCT check of query-packed.mjs plate deliveries; verifier only."""
from pathlib import Path
import hashlib, json, math, sys
from OCP.STEPControl import STEPControl_Reader
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.BRepBndLib import BRepBndLib
from OCP.Bnd import Bnd_Box
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_SOLID, TopAbs_EDGE
from OCP.TopoDS import TopoDS
from OCP.BRepAdaptor import BRepAdaptor_Curve
from OCP.GeomAbs import GeomAbs_Circle

out = Path(sys.argv[1]).resolve()
receipts = json.loads((out / 'receipts.json').read_text())
rows = []
for name, height in [('built', 8), ('initial', 8), ('edited', 10)]:
    receipt = receipts[name]['data']
    # Linux outputs were mounted at /consumer; bind the portable suffix to this copy.
    suffix = receipt['artifact_directory'].split('/' + out.name + '/', 1)[1]
    step = (out / suffix / 'model.step').resolve()
    assert step.is_relative_to(out)
    digest = hashlib.sha256(step.read_bytes()).hexdigest()
    assert digest == receipt['artifact_sha256']['model.step']
    reader = STEPControl_Reader()
    assert int(reader.ReadFile(str(step))) == 1
    reader.TransferRoots()
    shape = reader.OneShape()
    assert BRepCheck_Analyzer(shape).IsValid()
    solids = TopExp_Explorer(shape, TopAbs_SOLID)
    count = 0
    while solids.More():
        count += 1
        solids.Next()
    assert count == 1
    props = GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, props, Eps=1e-10)
    volume = props.Mass()
    expected = (40 * 30 - 4 * math.pi * 2**2) * height
    assert abs(volume - expected) < 1e-5
    box = Bnd_Box()
    BRepBndLib.AddOptimal_s(shape, box)
    bounds = box.Get()
    assert max(abs(a-b) for a,b in zip(bounds, [-20,-15,0,20,15,height])) < 1e-5
    centers = []
    edges = TopExp_Explorer(shape, TopAbs_EDGE)
    while edges.More():
        curve = BRepAdaptor_Curve(TopoDS.Edge_s(edges.Current()))
        if curve.GetType() == GeomAbs_Circle:
            circle = curve.Circle()
            point = circle.Location()
            p = [point.X(), point.Y(), point.Z()]
            if abs(circle.Radius()-2) < 1e-5 and abs(point.Z()-height) < 1e-5:
                if not any(math.dist(p,q) < 1e-5 for q in centers):
                    centers.append(p)
        edges.Next()
    assert len(centers) == 4
    for x in [-10,10]:
        for y in [-6,6]:
            assert any(math.dist([x,y,height],p) < 1e-5 for p in centers)
    rows.append({'id':name,'valid':True,'solids':count,'volume_mm3':volume,
                 'bounds_mm':bounds,'top_hole_centers_mm':centers,'step_sha256':digest})
result = {'passed':True,'verifier':'independent OCCT','linear_tolerance_mm':1e-5,
          'volume_tolerance_mm3':1e-5,'rows':rows}
(out / 'plate-occt-readback.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({'passed':True,'plates':len(rows)}))
