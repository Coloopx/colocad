"""Paired fresh-process v0.3 benchmark, alternated backend order, 5 repetitions.
Runs on one macOS host; no rendering, LLM, Docker, or app-level design checks.
"""
import argparse,datetime,hashlib,json,os,platform,re,statistics,subprocess,tempfile,time
from pathlib import Path
from cases import cases,ROOT
p=argparse.ArgumentParser();p.add_argument('--python',required=True);p.add_argument('--repetitions',type=int,default=5);args=p.parse_args()
python=os.path.abspath(args.python);binary=Path(os.environ['AGENT_CAD_BINARY']);definitions=cases()
out=ROOT/'data';raw={n:{b:[] for b in ['rust','build123d']} for n in definitions};warmups=[]
env=dict(os.environ,OMP_NUM_THREADS='1',OPENBLAS_NUM_THREADS='1',MKL_NUM_THREADS='1',VECLIB_MAXIMUM_THREADS='1',NUMEXPR_NUM_THREADS='1')
def run(backend,model,dest):
    dest.mkdir(parents=True,exist_ok=True)
    command=([str(binary),'build','--out',str(dest/'revision')] if backend=='rust' else [python,str(ROOT/'scripts/baseline.py'),str(dest/'model.step')])
    measure=['/usr/bin/time','-l'] if platform.system()=='Darwin' else ['/usr/bin/time','-v']
    t=time.perf_counter()
    try:
        r=subprocess.run(measure+command,input=json.dumps(model),capture_output=True,text=True,env=env,timeout=120)
        wall=(time.perf_counter()-t)*1000
        (dest/'stderr.log').write_text(r.stderr)
        rss=re.search(r'(\d+)\s+maximum resident set size',r.stderr) if platform.system()=='Darwin' else re.search(r'Maximum resident set size \(kbytes\): (\d+)',r.stderr)
        receipt=json.loads(r.stdout)
        result={'ok':r.returncode==0 and receipt.get('ok',False),'wall_ms':wall,'peak_rss_mib':int(rss[1])/(1024**2 if platform.system()=='Darwin' else 1024) if rss else None,'receipt':receipt}
    except Exception as e:result={'ok':False,'error':str(e),'wall_ms':(time.perf_counter()-t)*1000}
    (dest/'run.json').write_text(json.dumps(result,indent=2)+'\n');return result
# One prewarm each; retained separately from measured trials.
box={'version':1,'units':'mm','parameters':{},'features':[{'op':'box','id':'b','origin':[0,0,0],'size':[10,10,10]}],'output':'b'}
for b in ['rust','build123d']:
    with tempfile.TemporaryDirectory(prefix='cad-compare-warmup-') as tmp:warmups.append({'backend':b,**run(b,box,Path(tmp))})
assert all(r['ok'] for r in warmups), 'Backend warmup failed; fix environment before measurement'
for repetition in range(args.repetitions):
    for index,(name,(_,_,_,model)) in enumerate(definitions.items()):
        for backend in (['rust','build123d'] if (repetition+index)%2==0 else ['build123d','rust']):
            if repetition==0:result=run(backend,model,out/name/backend)
            else:
                with tempfile.TemporaryDirectory(prefix='cad-compare-') as tmp:result=run(backend,model,Path(tmp))
            result['repetition']=repetition+1;raw[name][backend].append(result)
            print(name,backend,repetition+1,'ok' if result['ok'] else result.get('error',str(result.get('receipt'))),round(result['wall_ms'],1),flush=True)
        (out/'runs.partial.json').write_text(json.dumps(raw,indent=2))
summary={}
for name,backends in raw.items():
    summary[name]={}
    for backend,runs in backends.items():
        ok=[r for r in runs if r['ok']];row={'successes':len(ok),'attempts':len(runs)}
        if ok:
            row.update(wall_median_ms=statistics.median(r['wall_ms'] for r in ok),wall_range_ms=[min(r['wall_ms'] for r in ok),max(r['wall_ms'] for r in ok)],peak_rss_max_mib=max(r['peak_rss_mib'] for r in ok),phase_median_ms={phase:statistics.median(r['receipt']['data']['timing_ms'].get(phase,0) for r in ok) for phase in ['import','geometry','validation','export','total']})
        summary[name][backend]=row
meta={'generated_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'rust_executor':os.environ['COLOCAD_BENCH_REVISION'],'host':platform.platform(),'machine':platform.machine(),'repetitions':args.repetitions,'python':subprocess.check_output([python,'--version'],text=True).strip(),'environment':{'build123d':subprocess.check_output([python,'-c','from importlib.metadata import version;print(version("build123d"))'],text=True).strip()},'thread_env':{k:env[k] for k in ['OMP_NUM_THREADS','OPENBLAS_NUM_THREADS','MKL_NUM_THREADS','VECLIB_MAXIMUM_THREADS','NUMEXPR_NUM_THREADS']},'method':'Alternating paired fresh processes; one recorded warmup/backend; warm OS cache; native host (no CPU/memory cgroup). External wall includes process start, dependency imports, geometry, each backend validation, STEP export. Backend validation semantics differ. No preview/LLM/Docker/app-level assembly checks. build123d uses batch hole cuts. RSS is max process peak across repeats, not total system memory.','source_sha256':{str(f.relative_to(ROOT)):hashlib.sha256(f.read_bytes()).hexdigest() for f in [ROOT/'scripts/baseline.py',ROOT/'scripts/benchmark.py',ROOT/'scripts/cases.py',*sorted((ROOT/'models').glob('*.json'))]},'rust_binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'summary':summary,'runs':raw,'warmups':warmups}
(out/'benchmark.json').write_text(json.dumps(meta,ensure_ascii=False,indent=2)+'\n')
print('FINISHED',flush=True)
