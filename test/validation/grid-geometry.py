"""Independent OCCT check of grid-query.mjs outputs. Run with an OCP environment."""
from pathlib import Path
import sys, json, math, hashlib
import OCP
from OCP.STEPControl import STEPControl_Reader
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.BRepAlgoAPI import BRepAlgoAPI_Cut
from OCP.BRepBndLib import BRepBndLib
from OCP.Bnd import Bnd_Box
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_SOLID, TopAbs_FACE, TopAbs_IN, TopAbs_OUT
from OCP.TopoDS import TopoDS
from OCP.BRepAdaptor import BRepAdaptor_Surface
from OCP.GeomAbs import GeomAbs_Cylinder
from OCP.BRepClass3d import BRepClass3d_SolidClassifier
from OCP.gp import gp_Pnt

root = Path(sys.argv[1]).resolve()
inputs = json.loads((root / 'geometry-inputs.json').read_text())
def volume(shape):
    p = GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, p, Eps=1e-10)
    return p.Mass()

rows, shapes = [], {}
for case in inputs:
    path = root / case['artifact_directory'] / 'model.step'
    reader = STEPControl_Reader()
    assert int(reader.ReadFile(str(path))) == 1
    reader.TransferRoots()
    shape = reader.OneShape()
    assert BRepCheck_Analyzer(shape).IsValid()
    solids = TopExp_Explorer(shape, TopAbs_SOLID)
    count = 0
    while solids.More():
        solid = TopoDS.Solid_s(solids.Current())
        count += 1
        solids.Next()
    assert count == 1
    radius, centers, length = case['radius'], case['centers'], case['length']
    expected = (length * 50 - len(centers) * math.pi * radius**2) * 6
    measured = volume(shape)
    assert abs(measured - expected) < 1e-5
    bbox = Bnd_Box()
    BRepBndLib.AddOptimal_s(shape, bbox)
    bounds = list(bbox.Get())
    assert max(abs(a-b) for a,b in zip(bounds,[0,0,0,length,50,6])) < 1e-5
    faces = TopExp_Explorer(shape, TopAbs_FACE)
    cylinders = []
    while faces.More():
        surface = BRepAdaptor_Surface(TopoDS.Face_s(faces.Current()))
        if surface.GetType() == GeomAbs_Cylinder:
            c = surface.Cylinder()
            assert abs(c.Radius()-radius) < 1e-5
            assert abs(abs(c.Axis().Direction().Z())-1) < 1e-7
            cylinders.append([c.Location().X(),c.Location().Y()])
        faces.Next()
    # Exact analytic bore axes, plus material/air probes, not just total volume.
    assert len(cylinders) == len(centers)
    for x,y in centers:
        assert any(math.hypot(x-a,y-b) < 1e-5 for a,b in cylinders)
        for z in [0.1,3,5.9]:
            assert BRepClass3d_SolidClassifier(solid,gp_Pnt(x,y,z),1e-7).State() == TopAbs_OUT
            assert BRepClass3d_SolidClassifier(solid,gp_Pnt(x+radius+0.1,y,z),1e-7).State() == TopAbs_IN
    shapes[case['id']] = shape
    rows.append(dict(id=case['id'],valid=True,solids=count,volume_mm3=measured,
        bounds_mm=bounds,cylinder_axes_xy=cylinders,material_air_probes=len(centers)*6,
        step_sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
differences = []
for a,b in [('plain','queryable'),('queryable','plain')]:
    cut = BRepAlgoAPI_Cut(shapes[a],shapes[b])
    cut.Build()
    assert cut.IsDone()
    v = abs(volume(cut.Shape()))
    assert v < 1e-6
    differences.append(dict(a=a,b=b,material_difference_mm3=v))
result = dict(passed=True,verifier='OCCT via OCP',binding_version=OCP.__version__,
    linear_tolerance_mm=1e-5,volume_tolerance_mm3=1e-5,
    symmetric_difference_tolerance_mm3=1e-6,rows=rows,differences=differences)
(root/'occt-readback.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result))
