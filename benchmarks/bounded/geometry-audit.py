"""Prove matched receipts, then independently read each distinct artifact in OCCT."""
import argparse,json,subprocess
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('runs',type=Path);p.add_argument('--image',default='colocad-memory:20261111');a=p.parse_args()
repo=Path(__file__).resolve().parents[2];runs=a.runs.resolve();unique={}; groups={}
for file in sorted(runs.glob('*/comparison.json')):
    row=json.loads(file.read_text());groups.setdefault((row['family'],row['n']),[]).append(row)
assert len(groups)==12,'expected the complete alpha.9 geometry matrix'
for case,rows in groups.items():
    assert len(rows)==6,case
    assert {(r['rep'],r['variant']) for r in rows}=={(i,v) for i in range(3) for v in ['before','after']},case
    keys=['input_sha256','sha256','geometry','validation']
    for row in rows:
        assert all(row[k]==rows[0][k] for k in keys),row['id']
        # The archive is in the same directory even if the entire run has moved.
        sha=row['sha256'];unique[case,sha]={'family':row['family'],'n':row['n'],'id':str(row['family'])+'-'+str(row['n']),'path':'/audit/'+sha+'.step'}
(runs/'occt-manifest.json').write_text(json.dumps(list(unique.values()),indent=2)+'\n')
subprocess.run(['docker','run','--rm','--network','none','--cpus','1','-v',f'{repo}/benchmarks/memory:/bench:ro','-v',f'{runs}:/audit',a.image,'/opt/cq/bin/python','/bench/verify.py','/audit/occt-manifest.json','/audit/occt.json'],check=True)
verified=json.loads((runs/'occt.json').read_text());assert len(verified)==len(unique) and all(r['passed'] for r in verified)
