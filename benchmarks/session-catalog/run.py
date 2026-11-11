"""Serial alpha.9 M1 checks using the original CAD benchmark cgroup monitor."""
import argparse
import json
import subprocess
import uuid
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument('fixtures', type=Path, help='v1 metadata ledgers from benchmarks/memory/history-prepare.py')
p.add_argument('output', type=Path)
p.add_argument('--data-directory', type=Path, help='new host directory for persistent project data on a bind mount')
p.add_argument('--image', default='colocad-memory:20261111')
args = p.parse_args()
repo = Path(__file__).resolve().parents[2]
out = args.output.resolve()
out.mkdir(parents=True, exist_ok=True)
volume = 'colocad-catalog-' + uuid.uuid4().hex
if args.data_directory:
    args.data_directory.mkdir(parents=True, exist_ok=False)
    storage = str(args.data_directory.resolve())
else:
    subprocess.run(['docker', 'volume', 'create', volume], check=True)
    storage = volume
try:
    cases = [('append', 10000, 0)]
    for count in (10000, 100000, 500000):
        cases.append(('prepare', count, 0))
        cases.append(('migrate', count, 0))
        cases.extend(('head', count, rep) for rep in range(3))
    for op, n, rep in cases:
        dest = out / f'{op}-{n}-{rep}'
        dest.mkdir()
        directory = '/data/edits' if op == 'append' else f'/data/{n}'
        subprocess.run([
            'docker', 'run', '--rm', '--network', 'none', '--cpus', '1',
            '--memory', '512m', '--memory-swap', '512m', '-e', 'CASE_TIMEOUT=1800',
            '-v', f'{repo}:/pkg:ro', '-v', f'{storage}:/data',
            '-v', f'{args.fixtures.resolve()}:/fixtures:ro', '-v', f'{dest}:/out',
            args.image, 'python', '/pkg/benchmarks/memory/monitor.py', 'node',
            '/pkg/benchmarks/session-catalog/workload.mjs', op, str(n), directory,
        ], check=True)
        row = json.loads((dest/'measurement.json').read_text())
        if row['status'] != 'ok':
            raise RuntimeError(f'{dest.name}: {row["status"]}, inspect retained logs')
finally:
    if not args.data_directory:
        subprocess.run(['docker', 'volume', 'rm', volume], check=True)
