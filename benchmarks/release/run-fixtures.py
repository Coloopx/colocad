"""Run scored fixtures serially with fresh processes/cgroups; keep unsuccessful rows."""
import argparse,json,random,subprocess,uuid
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('out',type=Path);a=p.parse_args();out=a.out.resolve();root=Path(__file__).resolve().parents[2];image='colocad-memory:20261111'
cases=['sensor-rail','manifold','finger','hand','sensor-rail-edit','manifold-edit','links-1','links-4','links-16','links-43','pins-16']
inputs=out/'inputs'
if not inputs.exists():
    subprocess.run(['node',root/'showcase/tendon-hand/bench/scope-models.mjs',inputs,'--instances'],check=True)
for name,edit in [('sensor-rail',{'step':17,'copies':2}),('manifold',{'pocketRadius':3.5,'instances':3})]:
    source=(root/'examples'/(name+'.json')).read_bytes();(inputs/(name+'.json')).write_bytes(source)
    m=json.loads(source);m['parameters'].update(edit);(inputs/(name+'-edit.json')).write_text(json.dumps(m))
runs=[];dest=out/'fixtures';dest.mkdir(exist_ok=True);index=dest/'runs.json'
if index.exists():runs=json.loads(index.read_text())
for rep in range(3):
    jobs=[(b,c) for c in cases for b in ['colocad','cadquery','build123d']];random.Random(550+rep).shuffle(jobs)
    for b,c in jobs:
        id=f'{c}-{b}-{rep}'
        if any(r['id']==id for r in runs):continue
        d=dest/id;d.mkdir();cmd=['docker','run','--rm','--network','none','--cpus','1','--memory','512m','--memory-swap','512m','--pids-limit','128','-v',f'{root}:/pkg:ro','-v',f'{inputs}:/inputs:ro','-v',f'{d}:/out',image,'python','/pkg/benchmarks/memory/monitor.py','python','/pkg/benchmarks/release/fixture.py',b,c]
        finished=subprocess.run(cmd,capture_output=True,text=True,timeout=240);(d/'container.log').write_text(finished.stderr+finished.stdout)
        m=json.loads((d/'measurement.json').read_text()) if (d/'measurement.json').exists() else {'status':'container_error','exit_code':finished.returncode}
        runs.append(dict(id=id,backend=b,case=c,rep=rep,**m));index.write_text(json.dumps(runs)+'\n')
        print(id,m['status'],round(m.get('wall_s',0),3),flush=True)
