"""Matched SDK query runs against private copies of one closed, queryable session."""
import argparse
import json
import subprocess
import uuid
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument('baseline', type=Path)
p.add_argument('candidate', type=Path)
p.add_argument('seed', type=Path)
p.add_argument('out', type=Path)
p.add_argument('--queries', type=int, default=24)
p.add_argument('--repeats', type=int, default=3)
a = p.parse_args()
repo = Path(__file__).resolve().parents[2]
image = 'colocad-memory:20261111'
for rep in range(a.repeats):
    order = [('before',a.baseline),('after',a.candidate)]
    for label, pkg in (order if rep%2==0 else reversed(order)):
        out = (a.out / f'query-{label}-{rep+1}').resolve()
        out.mkdir(parents=True, exist_ok=False)
        volume = 'colocad-query-' + uuid.uuid4().hex
        subprocess.run(['docker','volume','create',volume], stdout=subprocess.DEVNULL, check=True)
        docker = ['docker','run','--rm','--network','none','--cpus','1','--memory','512m',
                  '--memory-swap','512m','-v',volume+':/project']
        try:
            subprocess.run([*docker,'-v',f'{a.seed.resolve()}:/seed:ro',image,
                            'cp','-a','/seed/.','/project/'],check=True)
            # Hold the exact query executable fixed; here the only variable is the SDK.
            subprocess.run([*docker,'-v',f'{pkg.resolve()}:/pkg:ro',
                    '-v',f'{a.baseline.resolve()}/kernel/native:/pkg/kernel/native:ro',
                    '-v',f'{repo}/benchmarks:/bench:ro','-v',f'{out}:/out',image,'python',
                    '/bench/memory/monitor.py','node','/bench/query-io/workload.mjs',
                    'measure','/project',str(a.queries)],check=True)
            if json.loads((out/'measurement.json').read_text())['status']!='ok':
                raise RuntimeError(f'failed query run: {out}')
        finally:
            subprocess.run(['docker','volume','rm',volume], stdout=subprocess.DEVNULL, check=True)
