"""Fresh cgroup per invocation. Peaks include monitor, descendants, and file cache."""
import json,os,signal,subprocess,sys,time
from pathlib import Path
out=Path('/out');out.mkdir(exist_ok=True)
def read(name):return Path('/sys/fs/cgroup',name).read_text().strip()
def pairs(name):return {k:int(v) for k,v in (x.split() for x in read(name).splitlines())}
def io_counters():
    # Preserve per-device counters: summing stacked devices can double-count writes.
    path=Path('/sys/fs/cgroup/io.stat')
    if not path.exists():return None
    return {dev:{k:int(v) for k,v in (field.split('=') for field in fields)}
            for dev,*fields in (line.split() for line in path.read_text().splitlines())}
def sample():
    st=pairs('memory.stat');rss=0
    for p in Path('/proc').glob('[0-9]*/status'):
        try:
            for l in p.read_text().splitlines():
                if l.startswith('VmRSS:'):rss+=int(l.split()[1])*1024
        except (OSError,ProcessLookupError):pass
    return {'t':time.monotonic()-t,'current':int(read('memory.current')),'anon':st['anon'],'file':st['file'],'rss_sum':rss,'kernel':st.get('kernel',0)}
def child_setup():
    os.setsid()
    Path('/proc/self/oom_score_adj').write_text('500')
io_before=io_counters();t=time.monotonic();baseline=sample();timeout=int(os.environ.get('CASE_TIMEOUT','180'))
with (out/'stdout.log').open('w') as stdout,(out/'stderr.log').open('w') as stderr,(out/'memory.jsonl').open('w',buffering=1) as trace:
    p=subprocess.Popen(sys.argv[1:],stdout=stdout,stderr=stderr,preexec_fn=child_setup)
    maxima={'anon':0,'file':0,'rss_sum':0,'current':0,'kernel':0};timed_out=False
    while True:
        s=sample();trace.write(json.dumps(s)+'\n')
        for k in maxima:maxima[k]=max(maxima[k],s[k])
        if p.poll() is not None:break
        if time.monotonic()-t>timeout:
            timed_out=True;os.killpg(p.pid,signal.SIGKILL);p.wait();break
        time.sleep(.02)
    events=pairs('memory.events');result={}
    if (out/'result.json').exists():result=json.loads((out/'result.json').read_text())
    status='timeout' if timed_out else 'oom' if events.get('oom_kill',0) else 'ok' if p.returncode==0 and result.get('ok') else 'rejected' if result.get('error') else 'error'
    row={'status':status,'exit_code':p.returncode,'wall_s':time.monotonic()-t,'peak_bytes':int(read('memory.peak')),'sampled_max':maxima,'baseline':baseline,'events':events,'cpu':pairs('cpu.stat'),'limit_bytes':read('memory.max'),'swap_max':read('memory.swap.max'),'result':result}
    row['block_io']={'before':io_before,'after':io_counters()}
    # Apparent retained bytes, not total writes: rewrites, discarded temporaries,
    # cache hits and write amplification cannot be inferred from this number.
    ignored={'stdout.log','stderr.log','memory.jsonl','events.jsonl','result.json','input.json'}
    row['retained_output_bytes']=sum(f.stat().st_size for f in out.rglob('*') if f.is_file() and f.relative_to(out).as_posix() not in ignored)
    (out/'measurement.json').write_text(json.dumps(row,indent=2)+'\n')
print(json.dumps({k:row[k] for k in ['status','wall_s','peak_bytes','events']}),flush=True)
