"""Serial fresh-container campaign; no competing benchmark jobs on this host."""
import argparse,hashlib,json,subprocess,time,uuid
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--plan',type=Path,required=True);p.add_argument('--out',type=Path,required=True);p.add_argument('--image',default='colocad-memory:20261111');p.add_argument('--history',type=Path);a=p.parse_args()
here=Path(__file__).resolve().parent;repo=here.parent.parent;a.out=a.out.resolve();a.out.mkdir(parents=True,exist_ok=True)
plan=json.loads(a.plan.read_text());rows=[]
if (a.out/'runs.json').exists():rows=json.loads((a.out/'runs.json').read_text())['runs']
metadata={'image_id':subprocess.check_output(['docker','image','inspect',a.image,'--format','{{.Id}}'],text=True).strip(),'plan_sha256':hashlib.sha256(a.plan.read_bytes()).hexdigest(),'source_sha256':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in here.iterdir() if p.suffix in ('.py','.mjs')},'cpu':1,'swap':0,'sampling_ms':20,'architecture':'native Linux arm64','accounting':'cgroup v2 total peak and sampled anon/file/kernel; summed RSS is supplementary; includes monitor and complete backend process tree; excludes image installation, agent inference and verifier'}
def save(): (a.out/'runs.json').write_text(json.dumps({'environment':metadata,'runs':rows},indent=2)+'\n')
for j in plan:
    if any(r['id']==j['id'] for r in rows):continue
    if j.get('stop_after_failure') and any(r['backend']==j['backend'] and r['family']==j['family'] and r['status'] not in ('ok','not_run_after_boundary') for r in rows):
        rows.append({**j,'status':'not_run_after_boundary'});save();continue
    dest=a.out/j['id'];dest.mkdir(exist_ok=False);name='colocad-mem-'+uuid.uuid4().hex[:12]
    executable={'colocad':'python','build123d':'/opt/bd/bin/python','cadquery':'/opt/cq/bin/python'}[j['backend']]
    args=[executable,'/bench/workload.py',j['backend'],j['family'],str(j['n'])]
    if j.get('mode')=='compositional':args[1]='/bench/workload-compositional.py'
    if j.get('mode')=='instrumented':args[1]='/bench/workload-instrumented.py'
    if j.get('queryable'):args+=['--queryable']
    if j.get('mode')=='sharded':args=['python','/bench/sharded.py',j['backend'],str(j['n']),'--batch',str(j.get('batch',32))]
    if j.get('mode')=='history':args=(['node','/bench/history.mjs'] if j['variant']=='sdk' else ['python','/bench/history.py'])+[str(j['n'])]
    if j.get('variant')=='indexed-node':args=['node','/bench/history-indexed.mjs',str(j['n'])]
    if j.get('mode')=='native-control':args=['python','/bench/native-control.py',j['variant']]
    cmd=['docker','create','--name',name,'--network','none','--cpus','1','--memory',str(j.get('memory_mib',2048))+'m','--memory-swap',str(j.get('memory_mib',2048))+'m','--pids-limit','128','-e','CASE_TIMEOUT='+str(j.get('timeout_s',180)),'-v',str(repo)+':/pkg:ro','-v',str(here)+':/bench:ro','-v',str(dest)+':/out',a.image,'python','/bench/monitor.py',*args]
    if a.history:
        i=cmd.index(a.image);cmd[i:i]=['-v',str(a.history.resolve())+':/history']
    cid=subprocess.check_output(cmd,text=True).strip()
    try:
        try:r=subprocess.run(['docker','start','-a',cid],capture_output=True,text=True,timeout=j.get('timeout_s',180)+45)
        except subprocess.TimeoutExpired:
            subprocess.run(['docker','kill',cid],capture_output=True);r=None
        state=json.loads(subprocess.check_output(['docker','inspect',cid,'--format','{{json .State}}'],text=True))
        (dest/'container-state.json').write_text(json.dumps(state,indent=2)+'\n')
        (dest/'container.log').write_text((r.stdout+r.stderr) if r else 'host timeout\n')
        result=json.loads((dest/'measurement.json').read_text()) if (dest/'measurement.json').exists() else {'status':'oom' if state['OOMKilled'] else 'monitor_failed','exit_code':state['ExitCode']}
        rows.append({**j,**result,'container_oom_killed':state['OOMKilled']});save()
        print(j['id'],result['status'],round(result.get('wall_s',0),2),round(result.get('peak_bytes',0)/2**20,2),'MiB',flush=True)
    finally:subprocess.run(['docker','rm','-f',cid],capture_output=True)
