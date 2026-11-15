"""[INPUT]: 明确指定的新构建原生内核与公开UV缩放STEP回归输入。
[OUTPUT]: 独立OCCT读验、边界/体积/材料点及实际内核SHA回执。
[POS]: 只读几何审计；交付件始终由Colocad构建，不调整任何几何容差。
[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import platform
import subprocess

from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepClass3d import BRepClass3d_SolidClassifier
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.STEPControl import STEPControl_Reader
from OCP.TopAbs import TopAbs_SOLID, TopAbs_IN, TopAbs_OUT
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS
from OCP.gp import gp_Pnt

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / 'crates/agent-cad/tests/fixtures/uv-rescaled-plate.step'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def measure(path, translation):
    reader = STEPControl_Reader()
    assert int(reader.ReadFile(str(path))) == 1
    assert reader.TransferRoots() == 1
    shape = reader.OneShape()
    assert BRepCheck_Analyzer(shape).IsValid()
    explorer = TopExp_Explorer(shape, TopAbs_SOLID)
    assert explorer.More()
    solid = TopoDS.Solid_s(explorer.Current())
    explorer.Next()
    assert not explorer.More()
    properties = GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, properties)
    probes = []
    for point, expected in [((10, 10, 3), TopAbs_OUT), ((12, 10, 3), TopAbs_IN),
                            ((1, 1, 3), TopAbs_IN), ((25, 10, 3), TopAbs_OUT)]:
        world = [x + shift for x, shift in zip(point, translation)]
        state = BRepClass3d_SolidClassifier(solid, gp_Pnt(*world), 1e-6).State()
        assert state == expected, (world, state, expected)
        probes.append({'point_mm': world, 'material': 'inside' if state == TopAbs_IN else 'outside'})
    return {'sha256': digest(path), 'valid': True, 'solids': 1,
            'volume_mm3': properties.Mass(), 'material_probes': probes}


def verify(binary, output):
    output.mkdir(parents=True, exist_ok=False)
    raw = FIXTURE.read_text()
    original = measure(FIXTURE, [0, 0, 0])
    assert abs(original['volume_mm3'] - (2400 - 6 * math.pi)) < 0.001
    cases = []
    for assembly in [False, True]:
        translation = [5, -3, 2] if assembly else [0, 0, 0]
        model = {'version': 1, 'units': 'mm', 'parameters': {},
                 'assets': {'fixture': {'step': raw, 'sha256': digest(FIXTURE)}},
                 'features': [{'id': 'plate', 'op': 'import_step', 'asset': 'fixture', 'assembly': True}]}
        if assembly:
            model['assembly'] = [{'id': 'mounted', 'input': 'plate', 'translation': translation}]
        else:
            model['output'] = 'plate'
        directory = output / ('assembly' if assembly else 'part')
        call = subprocess.run([str(binary), 'build', '--out', str(directory)],
                              input=json.dumps(model), text=True, capture_output=True, timeout=120)
        receipt = json.loads(call.stdout)
        assert call.returncode == 0 and receipt['ok'], receipt
        current = measure(directory / 'model.step', translation)
        assert abs(current['volume_mm3'] - original['volume_mm3']) < 1e-6
        cases.append({'assembly': assembly, 'kernel_policy': receipt['data']['kernel'], **current})
    report = {'status': 'passed', 'platform': platform.platform(), 'native_sha256': digest(binary),
              'independent_engine': 'OCCT via OCP', 'original': original, 'cases': cases,
              'scope': 'independent STEP validity, solid count, equal source/output volume and material probes; no manufacturing certification'}
    (output / 'independent.json').write_text(json.dumps(report, indent=2) + '\n')
    return report


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--native', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = verify(args.native.resolve(), args.output.resolve())
    print(json.dumps({'status': result['status'], 'cases': len(result['cases']),
                      'native_sha256': result['native_sha256']}))
