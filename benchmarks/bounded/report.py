"""Collect evidence and render the self-contained report, without network dependencies."""
import argparse,datetime,hashlib,json,os
from pathlib import Path
P=argparse.ArgumentParser();P.add_argument('geometry',type=Path);P.add_argument('level1',type=Path)
P.add_argument('destination',type=Path)
for v in ('before','after'):
    for k in ('session','project','audit','binary','catalog'):P.add_argument('--'+v+'-'+k,type=Path,required=True)
P.add_argument('--frozen',nargs=2,type=Path,required=True);P.add_argument('--image-id',required=True)
a=P.parse_args();repo=Path(__file__).resolve().parents[2];out=a.destination;out.mkdir(parents=True,exist_ok=True)
load=lambda p:json.loads(p.read_text())
def digest(p):
    h=hashlib.sha256()
    with p.open('rb') as f:
        for b in iter(lambda:f.read(1<<20),b''):h.update(b)
    return h.hexdigest()
def session(run,project,audit):
    m=load(run/'measurement.json');check=load(audit)
    assert m['status']=='ok' and m['result']['verifiedInOrder']==check['count']==10001
    trace=[json.loads(l) for l in (run/'memory.jsonl').open()];point=lambda r:{k:round(r[k]/2**20,3) for k in ('current','anon','file','kernel')}
    # The monitor creates its trace immediately before launching the child. On the
    # original bind mount this filesystem timestamp and workload timestamps share UTC.
    stamp=getattr((run/'memory.jsonl').stat(),'st_birthtime',None);marks=[]
    if stamp is not None:
        for line in (run/'stdout.log').read_text().splitlines():
            n,iso=line.split();t=datetime.datetime.fromisoformat(iso.replace('Z','+00:00')).timestamp()-stamp
            if not (0<=t<=m['wall_s']+1):marks=[];break
            nearest=min(trace,key=lambda r:abs(r['t']-t));marks.append(dict(steps=int(n),elapsed_s=round(nearest['t'],2),**point(nearest)))
    trace_sample=[{'s':round(r['t'],2),'mib':round(r['current']/2**20,2)} for r in trace[::max(1,len(trace)//200)]]
    retain=0;files=0
    for d,_,names in os.walk(project):
        for name in names:files+=1;retain+=(Path(d)/name).stat().st_size
    return dict(peak_mib=m['peak_bytes']/2**20,sampled_max=point(m['sampled_max']),wall_s=m['wall_s'],cpu_s=m['cpu']['usage_usec']/1e6,files=files,retained_mib=retain/2**20,events=m['events'],milestones=marks,series=trace_sample,readback=check,monitor_sha256=digest(run/'measurement.json'),history=m['result'])
rows=[]
for f in sorted(a.geometry.glob('*/comparison.json')):
    r=load(f);m=load(f.parent/'measurement.json');assert m['status']=='ok' and m['limit_bytes']=='536870912' and m['swap_max']=='0' and not m['events']['oom_kill']
    r.pop('path');r['cgroup_cpu_s']=m['cpu']['usage_usec']/1e6;r['measurement_sha256']=digest(f.parent/'measurement.json');rows.append(r)
verify=load(a.geometry/'occt.json');assert len(verify)==12 and len(rows)==72
gates={r['id']:r for r in verify}
for r in rows:
    g=gates[f"{r['family']}-{r['n']}"];assert g['sha256']==r['sha256'] and g['passed']
phases=load(a.level1);assert len(phases)==6
assert len({r['step_sha256'] for r in phases})==1 and len({r['input_sha256'] for r in phases})==1
frozen=[load(p) for p in a.frozen];assert all(i['passed'] for i in frozen) and sum(len(i['rows']) for i in frozen)==24
e=dict(schema='colocad.bounded-comparison/1',candidate='9f53a4eb70a7acdeec9b8c1653e737951f8e362c',baseline='v0.1.0-alpha.9 (a2d39c576bc1c2ce6b13207b1ffed3acc000cae3)',environment=dict(platform='linux/arm64',vcpus=1,memory_mib=512,swap_mib=0,image_id=a.image_id,geometry_repeats=3,session_repeats=1,isolated_level1_platform='darwin/arm64'),native_hashes={v:{k:digest(getattr(a,v+'_'+k)) for k in ('binary','catalog')} for v in ('before','after')},protocol_hashes={f:digest(repo/f) for f in ['benchmarks/memory/workload.mjs','benchmarks/memory/monitor.py','benchmarks/memory/verify.py','benchmarks/session-catalog/workload.mjs','third_party/solidcore.lock.json','src/session.mjs','src/session-payload.mjs']},sessions={v:session(*(getattr(a,v+'_'+k) for k in ('session','project','audit'))) for v in ('before','after')},geometry=rows,level1=phases,occt=verify,frozen=[dict(split=r['split'],passed=r['passed'],cases=len(r['rows']),manifest_sha256=r['manifest_sha256']) for r in frozen])
(out/'observations.json').write_text(json.dumps(e,indent=2,ensure_ascii=False)+'\n')
template=(Path(__file__).with_name('report-template.html')).read_text()
text=json.dumps(e,separators=(',',':'),ensure_ascii=False).replace('<','\\u003c')
(out/'comparison.html').write_text(template.replace('/*EVIDENCE*/',text))
