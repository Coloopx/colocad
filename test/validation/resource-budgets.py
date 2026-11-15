"""[INPUT]: resource-budgets.mjs回执、公开原夹具和实际Colocad输出。
[OUTPUT]: 独立OCCT有效性、单实体/体积/包络与材料采样一致性。
[POS]: 只读几何审计；不将填充规模解释为复杂CAD/制造接受。
[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
"""
from pathlib import Path
import json
import sys
from itertools import product
from OCP.STEPControl import STEPControl_Reader
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.BRepBndLib import BRepBndLib
from OCP.Bnd import Bnd_Box
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_SOLID, TopAbs_IN, TopAbs_OUT
from OCP.TopoDS import TopoDS
from OCP.BRepClass3d import BRepClass3d_SolidClassifier
from OCP.gp import gp_Pnt


def read(path):
    reader = STEPControl_Reader()
    assert int(reader.ReadFile(str(path))) == 1
    reader.TransferRoots()
    shape = reader.OneShape()
    assert BRepCheck_Analyzer(shape).IsValid()
    solids = []
    it = TopExp_Explorer(shape, TopAbs_SOLID)
    while it.More():
        solids.append(TopoDS.Solid_s(it.Current()))
        it.Next()
    assert len(solids) == 1
    props = GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, props)
    box = Bnd_Box()
    BRepBndLib.AddOptimal_s(shape, box)
    return solids[0], {'solids': 1, 'valid': True, 'volume_mm3': props.Mass(), 'bbox_mm': list(box.Get())}


directory = Path(sys.argv[1]).resolve()
receipt = json.loads((directory / 'receipt.json').read_text())
assert receipt['status'] == 'passed'
base = Path(__file__).resolve().parents[2] / 'crates/agent-cad/tests/fixtures/occt-drilled-block.step'
a, original = read(base)
b, exported = read(Path(receipt['output_step']))
assert abs(original['volume_mm3'] - exported['volume_mm3']) <= max(1e-5, abs(original['volume_mm3']) * 1e-8)
assert all(abs(x-y) < 1e-5 for x,y in zip(original['bbox_mm'], exported['bbox_mm']))
box = original['bbox_mm']
probes = []
for fractions in product([0.1, 0.5, 0.9], repeat=3):
    point = [box[i] + fraction * (box[i+3] - box[i]) for i,fraction in enumerate(fractions)]
    left = BRepClass3d_SolidClassifier(a, gp_Pnt(*point), 1e-6).State()
    if left not in (TopAbs_IN, TopAbs_OUT):
        continue
    right = BRepClass3d_SolidClassifier(b, gp_Pnt(*point), 1e-6).State()
    assert left == right, point
    probes.append({'point_mm': point, 'material': 'inside' if left == TopAbs_IN else 'outside'})
assert probes
result = {'status': 'passed', 'original': original, 'exported': exported, 'material_probes': probes,
          'scope': 'OCCT read-only validity/properties/probes; not global equivalence or manufacturing proof'}
(directory / 'independent.json').write_text(json.dumps(result, indent=2)+'\n')
print(json.dumps({'status': result['status'], 'material_probes': len(probes)}))
