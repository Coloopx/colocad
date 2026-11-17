"""Real KK60 paired sessions. Alternate on one CPU, verify only after stopping each measured container."""
import argparse
import json
from pathlib import Path
import subprocess
import uuid

p=argparse.ArgumentParser()
p.add_argument('baseline',type=Path)
p.add_argument('candidate',type=Path)
p.add_argument('inputs',type=Path)
p.add_argument('output',type=Path)
p.add_argument('--pairs',type=int,default=5)
p.add_argument('--first-pair',type=int,default=1)
p.add_argument('--cpu',type=int,default=0)
p.add_argument('--image',default='colocad-memory:20261111')
args=p.parse_args()
bench=Path(__file__).resolve().parents[1]
for pair in range(args.first_pair,args.first_pair+args.pairs):
    for mode in (('candidate','baseline') if pair%2 else ('baseline','candidate')):
        dest=args.output.resolve()/f'{pair}-{mode}'
        if (dest/'archive.json').is_file():continue
        dest.mkdir(parents=True,exist_ok=True)
        (dest/'run-config.json').write_text(json.dumps(dict(cpu=args.cpu,mode=mode,pair=pair)))
        volume='colocad-segment-test-'+uuid.uuid4().hex
        runtime=getattr(args,mode).resolve()
        subprocess.run(['docker','volume','create',volume],check=True,stdout=subprocess.DEVNULL)
        base=['docker','run','--rm','--network','none','--cpuset-cpus',str(args.cpu),'--cpus','1',
              '-v',f'{volume}:/case','-v',f'{args.inputs.resolve()}:/case/inputs:ro','-v',f'{bench}:/pkg/benchmarks:ro',
              '-v',f'{runtime}/src:/pkg/src:ro','-v',f'{runtime}/kernel:/pkg/kernel:ro','-v',f'{dest}:/results']
        if mode=='candidate':base+=['-e','COLOCAD_BENCH_STORAGE=segmented']
        with (dest/'run.log').open('w') as log:
            def run(cmd,limit='2g'):
                subprocess.run([*base,'--memory',limit,'--memory-swap',limit,args.image,*cmd],check=True,stdout=log,stderr=subprocess.STDOUT)
            try:
                run(['node','/pkg/benchmarks/field-task/colocad.mjs','/case/inputs','/case/colocad','0','256'],'512m')
                run(['cp','/case/colocad/result.json','/case/colocad/cgroup-events.json','/results/'])
                print(pair,mode,'measured',flush=True)
                run(['python','-c',"import pathlib,json; p=pathlib.Path('/case/colocad/session'); e=list(p.rglob('*')); pathlib.Path('/results/layout.json').write_text(json.dumps(dict(files=sum(f.is_file() for f in e),directories=sum(f.is_dir() for f in e))))"])
                run(['/opt/cq/bin/python','/pkg/benchmarks/field-task/verify-comparison.py','/case','colocad'])
                run(['python','-c',"import json,pathlib; p=pathlib.Path('/case'); d=json.loads((p/'colocad-checked.json').read_bytes()); (p/'oracle.json').write_text(json.dumps(d['edits']))"])
                run(['node','/pkg/benchmarks/field-task/check-revisions.mjs','/case/colocad','/case/oracle.json','/case/archive.json'])
                run(['cp','/case/colocad-checked.json','/case/archive.json','/results/'])
            finally:
                subprocess.run(['docker','volume','rm',volume],check=True,stdout=subprocess.DEVNULL)
            print(pair,mode,'verified',flush=True)
