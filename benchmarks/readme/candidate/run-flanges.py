"""Run both mounting implementations and their common independent verifier."""
import argparse
from pathlib import Path
import subprocess,sys
p=argparse.ArgumentParser();p.add_argument('output',type=Path);p.add_argument('--cpu',type=int,default=5)
a=p.parse_args();repo=Path(__file__).resolve().parents[3];dest=a.output.resolve();dest.mkdir(exist_ok=False,parents=True)
for backend in ('colocad','cadquery','build123d'):
    work=dest/backend;work.mkdir()
    cmd=['docker','run','--rm','--network','none','--cpus','1','--cpuset-cpus',str(a.cpu),'--memory','512m','--memory-swap','512m',
         '-v',f'{repo}:/pkg:ro','-v',f'{work}:/out','colocad-memory:20261111']
    if backend=='colocad':cmd+=['node','/pkg/examples/engineering-interfaces/run.mjs','/out']
    else:cmd += ['/opt/'+('bd' if backend=='build123d' else 'cq')+'/bin/python','/pkg/benchmarks/field-task/python-flange.py',
                 backend,'/pkg/specs/experiments/0004/fixtures/engineering','/out']
    with (work/'run.log').open('w') as log:subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,check=True)
    subprocess.run([sys.executable,repo/'examples/engineering-interfaces/verify.py',work/'runs.json'],check=True)
