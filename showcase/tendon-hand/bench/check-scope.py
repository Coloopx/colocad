"""Verify the first measured STEP pair for each workload after the clocks stop."""
import argparse,hashlib,json
from pathlib import Path
import build123d as b
from verify import vol,difference
p=argparse.ArgumentParser();p.add_argument('run',type=Path);p.add_argument('out',type=Path);a=p.parse_args()
rows=json.loads((a.run/'scope.json').read_text())['runs'];res={}
for case,n in [('finger',1),('hand',43)]:
    ms=[];shapes=[];hashes={}
    for backend in ['colocad','build123d']:
        src=a.run/f'{case}-{backend}.step';h=hashlib.sha256(src.read_bytes()).hexdigest()
        assert h==next(i['step_sha256'] for i in rows if i['rep']==1 and i['case']==case and i['backend']==backend)
        s=b.import_step(src);assert s.is_valid and len(s.solids())==n
        parts=s.children or [s];assert len(parts)==n
        ms.append([([*q.bounding_box().min,*q.bounding_box().max],vol(q.wrapped)) for q in parts]);shapes.append(s);hashes[backend]=h
    bbox=max(abs(x-y) for (xba,v),(xbb,w) in zip(*ms) for x,y in zip(xba,xbb))
    rel=max(abs(v-w)/w for (_,v),(_,w) in zip(*ms))
    assert bbox<1e-4 and rel<1e-6
    res[case]={'solids':n,'valid':True,'bounds_max_delta_mm':bbox,'max_relative_volume_delta_per_solid':rel,'hashes':hashes}
    if n==1:
        sa,sb=shapes;d=[difference(sa.wrapped,sb.wrapped),difference(sb.wrapped,sa.wrapped)]
        assert sum(d)<1e-4;res[case]['difference_mm3']=d
a.out.write_text(json.dumps(res,indent=2)+'\n')
