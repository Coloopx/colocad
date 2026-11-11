"""Independently reopen the packaged hybrid-enclosure example with Python OCP."""
import json
import math
import sys
from pathlib import Path

from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.STEPControl import STEPControl_Reader
from OCP.TopAbs import TopAbs_SOLID
from OCP.TopExp import TopExp_Explorer

root = Path(sys.argv[1])
report = json.loads((root / "report.json").read_text())
mass = {r["id"]: r["volumeMm3"] for r in report["features"]}
proof = {}
for name, expected in [
    ("exact.step", [mass["housing"], 84]),
    ("lens.faceted.step", [48]),
    ("assembly.step", [mass["housing"], 84, 48]),
]:
    stream = STEPControl_Reader()
    assert int(stream.ReadFile(str(root / name))) == 1
    assert stream.TransferRoots() == len(expected)
    result = stream.OneShape()
    assert BRepCheck_Analyzer(result).IsValid()
    items = TopExp_Explorer(result, TopAbs_SOLID)
    volumes = []
    while items.More():
        props = GProp_GProps()
        BRepGProp.VolumeProperties_s(items.Current(), props, Eps=1e-9)
        volumes.append(props.Mass())
        items.Next()
    assert len(volumes) == len(expected)
    for a, b in zip(sorted(volumes), sorted(expected)):
        assert math.isclose(a, b, rel_tol=5e-5, abs_tol=1e-5), (name, a, b)
    proof[name] = {"valid": True, "solids": len(volumes), "volumeMm3": volumes}
print(json.dumps(proof))
