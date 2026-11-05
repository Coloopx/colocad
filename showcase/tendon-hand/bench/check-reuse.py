"""Compare placed geometry for the recorded before/after/independent STEP triples."""
import argparse
import hashlib
import json
from pathlib import Path

import build123d as bd
from verify import difference, vol


def metrics(part):
    box = part.bounding_box()
    return [*box.min, *box.max], vol(part.wrapped)


def compare(parts, others):
    box_error = max(abs(x-y) for (a,_),(b,_) in zip(parts,others,strict=True) for x,y in zip(a,b))
    mass_error = max(abs(a-b)/b for (_,a),(_,b) in zip(parts,others,strict=True))
    assert box_error < 1e-4 and mass_error < 1e-6, (box_error, mass_error)
    return {'max_bounds_delta_mm':box_error, 'max_relative_volume_delta':mass_error}


def verify(run, hand, out):
    observations = json.loads((run/'scope.json').read_text())['runs']
    result = {}
    for case in sorted({r['case'] for r in observations}):
        saved = [r for r in observations if r['rep']==1 and r['case']==case]
        properties, placed = {}, {}
        for row in saved:
            name = row['backend']
            path = run/f'{case}-{name}.step'
            assert hashlib.sha256(path.read_bytes()).hexdigest()==row['step_sha256']
            shape = bd.import_step(path)
            assert shape.is_valid and len(shape.solids())==row['solids']
            placed[name]=shape.children or [shape]
            assert len(placed[name])==row['solids']
            properties[name]=[metrics(part) for part in placed[name]]
        candidate = properties['candidate']
        record = {name:compare(candidate, properties[name]) for name in ['colocad','build123d']}
        record['valid_solids']=len(candidate)
        if case=='hand':
            input_instances=json.loads(hand.read_text())['assembly']
            tested=set();diffs=[]
            for spec, new, prev in zip(input_instances,placed['candidate'],placed['colocad'],strict=True):
                if spec['input'] in tested: continue
                tested.add(spec['input'])
                measured=[difference(new.wrapped,prev.wrapped),difference(prev.wrapped,new.wrapped)]
                assert sum(measured)/vol(prev.wrapped)<1e-6
                diffs.append({'definition':spec['input'],'difference_mm3':measured})
            record['checked_definitions']=diffs
        result[case]=record
    out.write_text(json.dumps(result,indent=2)+'\n')


if __name__=='__main__':
    p=argparse.ArgumentParser()
    p.add_argument('run',type=Path)
    p.add_argument('hand_model',type=Path)
    p.add_argument('out',type=Path)
    args=p.parse_args()
    verify(args.run,args.hand_model,args.out)
