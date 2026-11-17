"""Rotate backend order across three warmed trials with fresh volumes and cgroups."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
import subprocess
import sys

parser = argparse.ArgumentParser()
parser.add_argument('output', type=Path)
parser.add_argument('--runtime', type=Path, required=True, help='Extracted release npm package')
parser.add_argument('--cpu', type=int, default=0)
parser.add_argument('--storage', choices=('segmented','compact'))
args = parser.parse_args()
repo = Path(__file__).resolve().parents[3]
scripts = repo/'benchmarks/field-task'
runtime = args.runtime.resolve()
base = args.output.resolve()
base.mkdir(parents=True, exist_ok=True)
image = 'colocad-memory:20261111'
backends = ('cadquery', 'colocad', 'build123d')

def run(*command, **kwargs):
    subprocess.run(command, check=True, **kwargs)

# Touch the three import/build paths outside the scored cgroups.
warm = ['docker', 'run', '--rm', '--network', 'none', '--cpuset-cpus', str(args.cpu),
        '--cpus', '1', '--memory', '512m', '--memory-swap', '512m',
        '-v', f'{runtime}:/pkg:ro', image]
run(*warm, '/opt/cq/bin/python', '-c', 'import cadquery')
run(*warm, '/opt/bd/bin/python', '-c', 'import build123d')
run(*warm, 'node', '/pkg/bin/colocad.mjs', 'build', '/pkg/examples/manifold.json', '/tmp/warm',
    stdout=subprocess.DEVNULL)
def verify_round(dest):
    if args.storage=='segmented':
        assert json.loads((dest/'colocad/session/session.json').read_text())['format']=='colocad/session/4'
    for name in backends:
        run(sys.executable, scripts/'verify-comparison.py', dest, name)
    oracle = dest/'native-oracle.json'
    oracle.write_text(json.dumps(json.loads((dest/'colocad-checked.json').read_text())['edits']))
    run('node', scripts/'check-revisions.mjs', dest/'colocad', oracle, dest/'colocad-archive.json')

# OCCT reopens a completed round on the host, outside the timed guest cgroups.
with ThreadPoolExecutor(max_workers=1) as validator:
    checks = []
    for rotation in range(len(backends)):
        dest = base/f'round-{rotation+1}'
        run(sys.executable, scripts/'run-comparison.py', dest, '--candidate', runtime,
            '--cpu', str(args.cpu), '--rotation', str(rotation), *(['--storage',args.storage] if args.storage else []))
        checks.append(validator.submit(verify_round, dest))
    for check in checks:
        check.result()
