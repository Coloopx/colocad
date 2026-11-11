"""Repeat frozen suites on a shared 8-vCPU Linux host. Each lane is serial and pinned.
Start after plan-v1 has finished. Prior indexed baseline/closed seed are needed
only for the internal query ablation. Output roots must be fresh.
"""
import argparse,concurrent.futures,json,os,shutil,subprocess,sys,tempfile,time
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('out',type=Path);p.add_argument('--history',required=True,type=Path);p.add_argument('--old-indexed',required=True,type=Path);p.add_argument('--query-seed',required=True,type=Path);a=p.parse_args()
root=Path(__file__).resolve().parents[2];out=a.out.resolve();hist=a.history.resolve();docker=shutil.which('docker');image='colocad-memory:20261111'
common=out/'main/runs.json'
while not common.exists() or len(json.loads(common.read_text())['runs'])<108:time.sleep(2)
def scale(name):
    args=['python3','benchmarks/memory/run.py','--plan','benchmarks/memory/plan-'+name+'.json','--out',out/name]
    if name.startswith('history'):args+=['--history',hist]
    return name,args
series={
0:[('fixtures',['python3','benchmarks/release/run-fixtures.py',out]),('windows',['python3','benchmarks/release/run-windows.py',out]),('preview',['docker','run','--rm','--network','none','--cpus','1','--memory','512m','--memory-swap','512m','-v',str(root)+':/opt/package:ro',image,'node','/opt/package/benchmarks/readme/preview.mjs'])],
2:[scale(n) for n in ('extreme','history','history-node')],
4:[scale(n) for n in ('boundary','compositional','native-control')]+[('queries',['python3','benchmarks/query-io/run-query.py',a.old_indexed.resolve(),root,a.query_seed.resolve(),out/'queries']),('catalog',['python3','benchmarks/session-catalog/run.py',hist,out/'catalog','--data-directory',out/'catalog-projects'])],
6:[scale('sharded')]}
def run(lane):
    with tempfile.TemporaryDirectory() as tmp:
        wrapper=Path(tmp)/'docker'
        wrapper.write_text("#!"+sys.executable+"\nimport os,sys\na=sys.argv[1:]\nif a[0] in ('create','run'):a[1:1]=['--cpuset-cpus','"+str(lane)+"']\nos.execv("+repr(docker)+",['docker',*a])\n");wrapper.chmod(0o755)
        env={**os.environ,'PATH':tmp+':'+os.environ['PATH']}
        for label,cmd in series[lane]:
            log=out/(label+'.log')
            with log.open('w') as f:result=subprocess.run(cmd,cwd=root,env=env,stdout=f,stderr=subprocess.STDOUT)
            print(lane,label,result.returncode,flush=True)
            if result.returncode:print(label,'inspect retained log',flush=True)
with concurrent.futures.ThreadPoolExecutor(max_workers=len(series)) as pool:list(pool.map(run,series))
