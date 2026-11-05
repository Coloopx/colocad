"""Inside a fresh cgroup, after installation. Read memory.peak before staging evidence."""
import hashlib,json,os,shutil,subprocess,sys,time
from pathlib import Path
arm,case,rep=sys.argv[1:]; src=Path('/inputs')/(case+'.json')
proj=Path('/tmp/consumer'); (proj/'node_modules/@coloopx').mkdir(parents=True)
(proj/'node_modules/@coloopx/colocad').symlink_to('/opt/package')
step=Path('/tmp/model.step')
cmd=['node','/bench/assembly.mjs',str(src),'/tmp/part'] if arm=='colocad' else [os.environ['BASELINE_PYTHON'],'-I','/bench/baseline.py',str(src),str(step)]
t=time.perf_counter_ns();p=subprocess.run(cmd,cwd=proj,capture_output=True,text=True,timeout=100)
elapsed=(time.perf_counter_ns()-t)/1e6
cg=Path('/sys/fs/cgroup');peak=int((cg/'memory.peak').read_text())
if p.returncode: raise RuntimeError(p.stderr[-1000:])
r=json.loads(p.stdout.strip().splitlines()[-1])
if arm=='colocad': step=Path(r['directory'])/'model.step';count=r['geometry']['solids']
else:count=r['solids']
assert step.is_file() and count=={'hand':43,'finger':1}[case]
if rep=='1':shutil.copyfile(step,Path('/evidence')/(case+'-'+arm+'.step'))
print(json.dumps(dict(backend=arm,case=case,rep=int(rep),wall_ms=elapsed,peak_bytes=peak,solids=count,source_sha256=hashlib.sha256(src.read_bytes()).hexdigest(),step_sha256=hashlib.sha256(step.read_bytes()).hexdigest(),oom_kill=int(dict(l.split() for l in (cg/'memory.events').read_text().splitlines())['oom_kill']))))
