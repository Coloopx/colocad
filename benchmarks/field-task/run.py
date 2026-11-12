"""Fresh, serial resource-capped containers. Supply a previously empty output directory."""
import argparse, json, subprocess
from pathlib import Path
p=argparse.ArgumentParser()
p.add_argument('output',type=Path);p.add_argument('--runtime',type=Path,help='unpacked public npm package (defaults to source checkout)')
p.add_argument('--cpu',type=int,default=0);p.add_argument('--history-queries',type=int,default=0)
p.add_argument('--history-edits',type=int,default=0)
p.add_argument('--cache-probe',action='store_true',help='after 64 geometry edits, test Linux cache advice on expired revisions in this private output')
a=p.parse_args()
if min(a.history_queries,a.history_edits,a.cpu)<0 or (a.history_queries and a.history_edits):p.error('select one nonnegative history workload')
if a.cache_probe and a.history_edits!=64:p.error('the cache probe requires 64 edits')
here=Path(__file__).resolve().parent;repo=here.parents[1];out=a.output.resolve();out.mkdir(parents=True,exist_ok=False)
pkg=(a.runtime or repo).resolve();inputs=out/'inputs'
subprocess.run(['node',here/'prepare.mjs',inputs],check=True)
for backend in (['history'] if (a.history_queries or a.history_edits) else ['colocad','cadquery','build123d']):
    dest=out/backend;dest.mkdir()
    cmd=['docker','run','--rm','--network','none','--cpus','1','--cpuset-cpus',str(a.cpu),'--memory','512m','--memory-swap','512m',
        '-v',f'{pkg}:/pkg:ro','-v',f'{repo}/benchmarks:/pkg/benchmarks:ro','-v',f'{inputs}:/inputs:ro','-v',f'{dest}:/out','colocad-memory:20261111']
    if a.cache_probe:cmd+=['python','/pkg/benchmarks/field-task/cache-probe.py']
    if backend!='history':cmd+=['python','/pkg/benchmarks/memory/monitor.py']
    cmd+=(['node','/pkg/benchmarks/field-task/colocad.mjs','/inputs','/out',str(a.history_queries),str(a.history_edits)] if backend in ['history','colocad'] else
          ['/opt/'+{'cadquery':'cq','build123d':'bd'}[backend]+'/bin/python','/pkg/benchmarks/field-task/python-backends.py',backend,'/inputs','/out'])
    subprocess.run(cmd,check=True)
    if backend!='history':
        m=json.loads((dest/'measurement.json').read_text());assert m['status']=='ok', (backend,(dest/'stderr.log').read_text())
