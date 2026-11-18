"""Run paired GC probes on separate CPU lanes, alternating order within each lane."""
import argparse
import concurrent.futures
import json
from pathlib import Path
import subprocess
import uuid

parser = argparse.ArgumentParser()
parser.add_argument('runtime', type=Path)
parser.add_argument('inputs', type=Path)
parser.add_argument('output', type=Path)
parser.add_argument('--image', default='colocad-memory:20261111')
parser.add_argument('--cold-audit',action='store_true',help='Run one separate serial session and inspect old file residency after edit 256')
args = parser.parse_args()
work = args.output.resolve()
work.mkdir(parents=True, exist_ok=True)
field = Path(__file__).resolve().parents[1]/'field-task'
runtime = args.runtime.resolve()

def lane(cpu, modes):
    for mode in modes:
        dest = work/f'cpu{cpu}-{mode}'
        dest.mkdir()
        volume = 'colocad-attribution-'+uuid.uuid4().hex
        subprocess.run(['docker','volume','create',volume], check=True, stdout=subprocess.DEVNULL)
        mounts = ['-v',f'{runtime}/kernel:/pkg/kernel:ro','-v',f'{runtime}/src:/pkg/src:ro',
                  '-v',f'{runtime}/viewer:/pkg/viewer:ro','-v',f'{field.parent}:/pkg/benchmarks:ro',
                  '-v',f'{volume}:/case','-v',f'{args.inputs.resolve()}:/case/inputs:ro']
        base = ['docker','run','--rm','--network','none','--cpuset-cpus',str(cpu),'--cpus','1']
        with (dest/'run.log').open('w') as log:
            def run(command):
                subprocess.run(command, check=True, stdout=log, stderr=subprocess.STDOUT)
            run(base+['--memory','512m','--memory-swap','512m',*mounts,
                      '-e','COLOCAD_BENCH_GC_PROBE='+mode,
                      *(['-e','COLOCAD_BENCH_COLD_AUDIT=1'] if args.cold_audit else []),args.image,'node','--expose-gc',
                      '/pkg/benchmarks/field-task/colocad.mjs','/case/inputs','/case/colocad','0','256'])
            verify = base+['--memory','2g','--memory-swap','2g',*mounts,'-v',f'{dest}:/results',args.image]
            run(verify+['/opt/cq/bin/python','/pkg/benchmarks/field-task/verify-comparison.py','/case','colocad'])
            run(verify+['python','-c',"import json,pathlib; p=pathlib.Path('/case'); d=json.loads((p/'colocad-checked.json').read_bytes()); (p/'oracle.json').write_text(json.dumps(d['edits']))"])
            run(verify+['node','/pkg/benchmarks/field-task/check-revisions.mjs','/case/colocad','/case/oracle.json','/results/archive.json'])
            run(verify+['cp','/case/colocad/result.json','/case/colocad/cgroup-events.json','/case/colocad-checked.json','/results/'])
            if args.cold_audit:
                run(verify+['cp','/case/colocad/cold-pages.json','/case/colocad/advice-stat.json','/results/'])
        subprocess.run(['docker','volume','rm',volume], check=True, stdout=subprocess.DEVNULL)
        print(f'cpu{cpu} {mode} verified',flush=True)

if args.cold_audit:
    lane(0,['natural'])
    raise SystemExit
with concurrent.futures.ThreadPoolExecutor(2) as pool:
    futures=[pool.submit(lane,0,['natural','forced']),pool.submit(lane,1,['forced','natural'])]
    for future in futures:future.result()
