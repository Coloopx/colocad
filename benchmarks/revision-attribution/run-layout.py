"""Three paired, sequential runs on one pinned Linux CPU and fresh native volumes."""
import argparse
from pathlib import Path
import subprocess
import uuid

parser=argparse.ArgumentParser()
parser.add_argument('capture',type=Path)
parser.add_argument('output',type=Path)
parser.add_argument('--sync-pairs',action='store_true',help='Compare per-member and per-revision segment syncs')
args=parser.parse_args()
source=Path(__file__).resolve().parent
image='colocad-memory:20261111'
args.output.mkdir(parents=True,exist_ok=False)
pair=('segments','segments-commit') if args.sync_pairs else ('loose','segments')
for n,modes in enumerate([pair,pair[::-1],pair],1):
    for mode in modes:
        v='colocad-layout-'+uuid.uuid4().hex
        subprocess.run(['docker','volume','create',v],check=True,stdout=subprocess.DEVNULL)
        command=['docker','run','--rm','--cpuset-cpus','3','--cpus','1','--memory','512m','--memory-swap','512m','--network','none',
                 '-v',f'{source}:/bench:ro','-v',f'{args.capture.resolve()}:/input:ro','-v',f'{v}:/store',
                 '-v',f'{args.output.resolve()}:/results',image,'python','/bench/layout.py']
        for operation in ('write','verify'):
            subprocess.run(command+[operation,mode,'/input','/store',f'/results/r{n}-{mode}-{operation}.json'],check=True)
        subprocess.run(['docker','volume','rm',v],check=True,stdout=subprocess.DEVNULL)
        print(n,mode,'verified',flush=True)
