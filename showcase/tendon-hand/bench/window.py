"""One actual, sequential wall-clock window inside a preinstalled Linux sandbox."""
import hashlib,json,os,shutil,subprocess,sys,threading,time
from pathlib import Path
backend=sys.argv[1]; limit=float(sys.argv[2])
out=Path('/outputs')/backend;out.mkdir()
if backend == 'optimized':
    os.environ['AGENT_CAD_BINARY'] = '/opt/optimized'
native = backend in ('colocad', 'optimized')
project=Path('/tmp/consumer');(project/'node_modules/@coloopx').mkdir(parents=True)
(project/'node_modules/@coloopx/colocad').symlink_to('/opt/package')
doc=json.loads(Path('/inputs/drive.json').read_text())
def make(i):
    # Alternate diameter extremes first, then move inwards. Identical unique queue per arm.
    d=[2.9,3.3,2.94,3.26,2.98,3.22,3.02,3.18,3.06,3.14,3.10][i%11]
    r=round(.65+(i//11)*.01,2)
    doc['parameters'].update(guideRadius=d/2,cableRadius=r)
    return d,r,json.dumps(doc,separators=(',',':')).encode()
def job(i,warm=False):
    scratch=Path('/tmp/job');scratch.mkdir()
    diameter,cable,data=make(i)
    src=scratch/'hand.json';src.write_bytes(data)
    if native: cmd=['node','/bench/build.mjs',str(src),str(scratch/'build')]
    else: cmd=[os.environ['BASELINE_PYTHON'],'-I','/bench/baseline.py',str(src),str(scratch/'model.step')]
    tic=time.perf_counter()
    p=subprocess.run(cmd,cwd=project,capture_output=True,text=True,timeout=110)
    if p.returncode:raise RuntimeError((p.returncode,p.stderr[-1500:]))
    got=json.loads(p.stdout.strip().splitlines()[-1])
    if native:
        rd=Path(got['directory']);step=rd/'model.step'; count=got['geometry']['solids']
    else: step=scratch/'model.step';count=got['solids']
    assert count==43
    # Both backends hand off their STEP to the same persistent output store.
    target=out/('warmup.step' if warm else f'{i:03d}.step')
    shutil.copyfile(step,target)
    end=time.perf_counter()
    row=dict(revision=i,guide_diameter_mm=diameter,cable_radius_mm=cable,solids=count,
             model_sha256=hashlib.sha256(data).hexdigest(),step_sha256=hashlib.sha256(target.read_bytes()).hexdigest(),
             start_s=tic-start,end_s=end-start,job_s=end-tic)
    if not native:row['phases']=got
    else:row['kernel_ms']=json.loads((rd/'report.json').read_text())['timing_ms']
    shutil.rmtree(scratch)
    return row
# Use cgroup memory.current for the entire timed window. It includes this sampler,
# subprocesses and charged cache/output pages. The warm-up has finished before t=0.
start = time.perf_counter()
warm = job(0, True)
cg = Path('/sys/fs/cgroup')
memory_file = cg/'memory.current'
samples = []
stop = threading.Event()
ready = threading.Event()

def measure_memory():
    ready.wait()
    while not stop.wait(.025):
        value = int(memory_file.read_text())
        elapsed = time.perf_counter() - start
        if elapsed > limit:
            break
        samples.append((round(elapsed,6),value))

thread = threading.Thread(target=measure_memory)
thread.start()
initial = int(memory_file.read_text())
start = time.perf_counter()
samples.append((0.,initial))
ready.set()
try:
    for i in range(100):
        if time.perf_counter()-start >= limit:
            break
        row = job(i)
        row['in_window'] = row['end_s'] <= limit
        with (out/'events.jsonl').open('a') as stream:
            stream.write(json.dumps(row)+'\n')
        print(backend,i,round(row['end_s'],2),row['in_window'],flush=True)
        if not row['in_window']:
            break
finally:
    stop.set()
    thread.join()
(out/'memory.json').write_text(json.dumps(samples,separators=(',',':'))+'\n')
binary = Path('/opt/optimized') if backend=='optimized' else Path('/opt/package/kernel/native/linux-arm64/agent-cad')
kernel_sha256 = hashlib.sha256(binary.read_bytes()).hexdigest() if native else None
env = {'window_s':limit,'kernel_sha256':kernel_sha256,'final_clock_s':time.perf_counter()-start,
       'cpu_max':(cg/'cpu.max').read_text().strip(),'memory_max_bytes':int((cg/'memory.max').read_text()),
       'oom_events':(cg/'memory.events').read_text(),'warmup_s':warm['job_s'],'platform':os.uname().machine,
       'sampling_target_ms':25,'sampled_window_peak_bytes':max(value for _,value in samples),
       'sample_count':len(samples)}
(out/'environment.json').write_text(json.dumps(env,indent=2)+'\n')
