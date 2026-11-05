"""Cross-check the first measured exports in OCCT, after all timed runs."""
import argparse
import hashlib
import json
from pathlib import Path

import build123d as bd
from OCP.BRepGProp import BRepGProp
from OCP.BRepAlgoAPI import BRepAlgoAPI_Cut
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.GProp import GProp_GProps

def volume(shape):
    props = GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, props, 1e-9, True, False)
    return props.Mass()

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('trial_directory', type=Path)
    directory = parser.parse_args().trial_directory
    runs = json.loads((directory/'runs.json').read_text())['runs']
    results = {}
    for case in ('sensor-rail','manifold'):
        pair, shapes, boxes, volumes = {}, [], [], []
        for name in ('build123d','colocad'):
            path = directory/f'{case}-{name}.step'
            sha = hashlib.sha256(path.read_bytes()).hexdigest()
            assert sha == next(r['step_sha256'] for r in runs if r['rep']==1 and r['case']==case and r['backend']==name)
            shape = bd.import_step(path)
            box = shape.bounding_box()
            assert shape.is_valid and len(shape.solids()) == 1
            shapes.append(shape)
            boxes.append([*box.min,*box.max])
            volumes.append(volume(shape.wrapped))
            pair[name] = {'valid': True,'solids': 1,'volume_mm3': volumes[-1], 'bounds_mm': boxes[-1],
                          'faces': len(shape.faces()), 'edges':len(shape.edges()),'step_sha256': sha}
        diffs = []
        for i in (0,1):
            cut = BRepAlgoAPI_Cut(shapes[i].wrapped,shapes[1-i].wrapped)
            cut.SetFuzzyValue(1e-6)
            cut.Build()
            assert cut.IsDone() and BRepCheck_Analyzer(cut.Shape()).IsValid()
            diffs.append(abs(volume(cut.Shape())))
        bbox_error = max(abs(a-b) for a,b in zip(*boxes))
        volume_error = abs(volumes[0]-volumes[1])
        assert bbox_error < 1e-4 and max(volume_error,sum(diffs))/max(volumes)<1e-6
        results[case] = {**pair,'bounds_max_delta_mm':bbox_error,'volume_delta_mm3':volume_error,
                         'difference_volumes_mm3':diffs,'boolean_fuzzy_mm':1e-6}
    print(json.dumps(results,indent=2))

if __name__ == '__main__':
    main()
