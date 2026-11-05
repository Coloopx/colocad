"""Check both recorded STEPs and produce their same-camera readback images."""
import argparse
import hashlib
import json
from pathlib import Path
import sys

import build123d as bd
from OCP.BRepAlgoAPI import BRepAlgoAPI_Cut
from OCP.BRepCheck import BRepCheck_Analyzer
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from check_step import volume
from figures import render


def prepare(directory):
    trace = json.loads((directory/'trace.json').read_text())
    shapes = []
    checks = {}
    for row in trace['recorded']:
        backend = row['backend']
        path = directory/f'manifold-{backend}.step'
        assert hashlib.sha256(path.read_bytes()).hexdigest() == row['step_sha256']
        shape = bd.import_step(path)
        assert shape.is_valid and len(shape.solids()) == 1
        shapes.append(shape)
        box = shape.bounding_box()
        checks[backend] = {'solids':1, 'faces':len(shape.faces()), 'edges':len(shape.edges()),
                           'volume_mm3':volume(shape.wrapped), 'bounds_mm':[*box.min,*box.max]}
        render(path, (-0.46,-1,-1.3), [(0,90),(0,45),(0,12)]).save(directory/f'{backend}.png')
    checks['difference_mm3'] = []
    for first, second in [(0,1),(1,0)]:
        cut = BRepAlgoAPI_Cut(shapes[first].wrapped, shapes[second].wrapped)
        cut.SetFuzzyValue(1e-6)
        cut.Build()
        assert cut.IsDone() and BRepCheck_Analyzer(cut.Shape()).IsValid()
        checks['difference_mm3'].append(abs(volume(cut.Shape())))
    assert max(checks['difference_mm3']) < 1e-4
    (directory/'readback.json').write_text(json.dumps(checks,indent=2)+'\n')


if __name__=='__main__':
    parser=argparse.ArgumentParser()
    parser.add_argument('directory',type=Path)
    prepare(parser.parse_args().directory)
