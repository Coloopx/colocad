"""Compare a recorded KK60 edit session on fresh Docker volumes, one workflow at a time."""
import argparse
import json
from pathlib import Path
import subprocess
import uuid

parser = argparse.ArgumentParser()
parser.add_argument('out', type=Path)
parser.add_argument('--candidate', required=True, type=Path)
parser.add_argument('--baseline', required=True, type=Path)
parser.add_argument('--cpu', default=4, type=int)
args = parser.parse_args()
repo = Path(__file__).resolve().parents[2]
out = args.out.resolve()
out.mkdir(exist_ok=True, parents=True)
inputs = out/'inputs'
if not inputs.exists():
    subprocess.run(['node', repo/'benchmarks/field-task/prepare.mjs', inputs], check=True)
image = 'colocad-memory:20261111'
for name in ('alpha10', 'cadquery', 'colocad', 'build123d'):
    dest = out/name
    dest.mkdir()
    runtime = (args.baseline if name == 'alpha10' else args.candidate).resolve()
    volume = 'colocad-edit-'+uuid.uuid4().hex
    command = ['docker','run','--rm','--cpuset-cpus',str(args.cpu),'--cpus','1',
               '--network','none','--memory','512m','--memory-swap','512m',
               '-v',f'{runtime}/kernel:/pkg/kernel:ro','-v',f'{runtime}/src:/pkg/src:ro',
               '-v',f'{repo}/benchmarks/field-task:/pkg/benchmarks/field-task:ro',
               '-v',f'{inputs}:/inputs:ro','-v',f'{volume}:/out',image]
    if name in ('alpha10','colocad'):
        command += ['node','/pkg/benchmarks/field-task/colocad.mjs','/inputs','/out','0','256']
    else:
        python = '/opt/cq/bin/python' if name == 'cadquery' else '/opt/bd/bin/python'
        command += [python,'/pkg/benchmarks/field-task/python-backends.py',name,'/inputs','/out','--edits','256']
    print(name, 'start', flush=True)
    # Preserve the volume and logs if a workflow or copy fails.
    with (dest/'run.log').open('w') as log:
        outcome = subprocess.run(command,stdout=log,stderr=subprocess.STDOUT)
    subprocess.run(['docker','run','--rm','--network','none','-v',f'{volume}:/from:ro',
                    '-v',f'{dest}:/to',image,'cp','-r','/from/.','/to'],check=True)
    outcome.check_returncode()
    subprocess.run(['docker','volume','rm',volume],check=True)
    assert json.loads((dest/'result.json').read_text())['ok']
    print(name, 'finished', flush=True)
