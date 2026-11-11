"""Produce one manifest for every successful common-range delivery (all repetitions)."""
import json,sys
from pathlib import Path
out=Path(sys.argv[1]).resolve();data=json.loads((out/'main/runs.json').read_text());rows=[]
for r in data['runs']:
    if r['status']!='ok':continue
    work=out/'main'/r['id']
    if r['backend']=='colocad':step,=work.glob('job-*/revision/model.step')
    else:step=work/'model.step'
    rows.append(dict(id=r['id'],family=r['family'],n=r['n'],path='/data/'+str(step.relative_to(out))))
(out/'main-manifest.json').write_text(json.dumps(rows))
