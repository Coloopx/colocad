"""[INPUT]: step_units生成的合成STEP及指定的OCP环境。
[OUTPUT]: 独立源/输出属性和对手算常量的交付断言，不假设实体顺序对应。
[POS]: B4同长度上下文验收；混合长度/原始mapped输入仅作诊断。
[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
"""
import argparse
import hashlib
import json
from pathlib import Path

import OCP
from OCP.Bnd import Bnd_Box
from OCP.BRepBndLib import BRepBndLib
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.IFSelect import IFSelect_RetDone
from OCP.STEPControl import STEPControl_Reader
from OCP.TopAbs import TopAbs_SOLID
from OCP.TopExp import TopExp_Explorer

# ---- 物理尺寸/体积手算，不读取Colocad报告作为期望 ----
EXPECTED = {
    'same-length-angle': (1, 24., [[0., 0., 0.], [2., 3., 4.]]),
    'same-length-uncertainty': (2, 48., [[0., 0., 0.], [2., 3., 4.]]),
    'uniform-cm': (1, 24000., [[0., 0., 0.], [20., 30., 40.]]),
    'nested-same-length': (2, 48., [[-31., 44., 0.], [-29., 97., 4.]]),
}


def properties(path):
    raw = path.read_bytes()
    reader = STEPControl_Reader()
    assert reader.ReadFile(str(path)) == IFSelect_RetDone
    reader.SetSystemLengthUnit(1.)
    reader.TransferRoots()
    shape = reader.OneShape()
    assert not shape.IsNull()
    props = GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, props, False, False, False)
    box = Bnd_Box()
    BRepBndLib.AddOptimal_s(shape, box, False, False)
    bounds = list(box.Get())
    explorer = TopExp_Explorer(shape, TopAbs_SOLID)
    count = 0
    while explorer.More():
        count += 1
        explorer.Next()
    return {'path': str(path), 'sha256': hashlib.sha256(raw).hexdigest(),
            'valid': bool(BRepCheck_Analyzer(shape, True).IsValid()),
            'solids': count, 'volume_mm3': props.Mass(), 'bbox_mm': [bounds[:3], bounds[3:]]}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--directory', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    assert not args.output.exists(), 'NO_OVERWRITE'
    report = {'OCP_version': OCP.__version__, 'cases': [],
              'scope': 'Independent read-only OCCT transfer; exported geometry checked against literal expected bounds/volumes; raw mapped source diagnostic only',
              'no_solid_index_correspondence_assumed': True}
    for name, (count, volume, bounds) in EXPECTED.items():
        source = properties(args.directory / (name + '.step'))
        output = properties(args.directory / (name + '-output.step'))
        good = output['valid'] and output['solids'] == count
        good = good and abs(output['volume_mm3'] - volume) < 1e-4
        good = good and all(abs(output['bbox_mm'][i][j] - bounds[i][j]) < 1e-5 for i in range(2) for j in range(3))
        source_expected = name != 'nested-same-length'
        if source_expected:
            good = good and source['valid'] and source['solids'] == count
            good = good and abs(source['volume_mm3'] - volume) < 1e-4
            good = good and all(abs(source['bbox_mm'][i][j] - bounds[i][j]) < 1e-5 for i in range(2) for j in range(3))
        report['cases'].append({'name': name, 'source': source, 'output': output,
                                'expected': {'solids': count, 'volume_mm3': volume, 'bbox_mm': bounds},
                                'raw_source_expected_asserted': source_expected, 'passed': good})
    report['passed'] = all(c['passed'] for c in report['cases'])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    assert report['passed'], 'INDEPENDENT_OCCT_COMPARISON_FAILED'


if __name__ == '__main__':
    main()
