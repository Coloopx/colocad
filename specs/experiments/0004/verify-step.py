"""Independent OCCT acceptance for run.mjs; not a build123d speed benchmark."""
import argparse
import hashlib
import importlib.metadata
import json
import math
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.BRepExtrema import BRepExtrema_DistShapeShape
from OCP.GProp import GProp_GProps
from OCP.TopAbs import TopAbs_SOLID
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS

p = argparse.ArgumentParser()
p.add_argument("run", type=Path, help="run.json produced by run.mjs")
a = p.parse_args()
run = json.loads(a.run.read_text())
step = a.run.parent / run["artifactDirectory"] / "model.step"
assert hashlib.sha256(step.read_bytes()).hexdigest() == run["stepSha256"]
reader = STEPControl_Reader()
assert int(reader.ReadFile(str(step))) == 1
assert reader.TransferRoots() > 0
shape = reader.OneShape()
assert BRepCheck_Analyzer(shape).IsValid()
solids = []
it = TopExp_Explorer(shape, TopAbs_SOLID)
while it.More():
    solids.append(TopoDS.Solid_s(it.Current()))
    it.Next()
assert len(solids) == 2
volumes = []
for solid in solids:
    assert BRepCheck_Analyzer(solid).IsValid()
    props = GProp_GProps()
    BRepGProp.VolumeProperties_s(solid, props)
    volumes.append(props.Mass())
expected = sorted([4800 - 108 * math.pi, 30 * 2.8 ** 2 * math.pi])
assert all(abs(x - y) < 1e-4 for x, y in zip(sorted(volumes), expected))
distance = BRepExtrema_DistShapeShape(solids[0], solids[1])
distance.Perform()
assert distance.IsDone()
assert abs(distance.Value() - 0.2) < 1e-5, distance.Value()
result = {
    "passed": True, "method": "independent OCCT STEP readback",
    "ocpPackage": importlib.metadata.version("cadquery-ocp-novtk"),
    "stepSha256": run["stepSha256"], "valid": True, "solids": 2,
    "volumeMm3": sorted(volumes), "expectedVolumeMm3": expected,
    "minimumDistanceMm": distance.Value(), "expectedMinimumDistanceMm": 0.2,
    "scope": "exported shape/placement correctness; not datum or clearance-query API acceptance",
}
(a.run.parent / "occt-readback.json").write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps(result))
