"""Paired first-STEP latency and peak memory for one feature-built link vs the whole hand."""
import argparse,hashlib,json,shutil,statistics,subprocess,tempfile
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('image');p.add_argument('tarball',type=Path);p.add_argument('out',type=Path);p.add_argument('--python',default='/home/agent/workspace/.structure-runtime/venv/bin/python3')
p.add_argument('--instances',action='store_true',help='Sweep one definition at 1, 4, 16 and 43 placements and compare a simple pin at 16.')
p.add_argument('--candidate',type=Path,help='Also run this Linux kernel with the same SDK and inputs.')
a=p.parse_args();a.out.mkdir(parents=True,exist_ok=False)
here=Path(__file__).resolve().parent
def call(*cmd):return subprocess.check_output(cmd,text=True).strip()
def sha(f):return hashlib.sha256(f.read_bytes()).hexdigest()
with tempfile.TemporaryDirectory() as t:
    ctx=Path(t)
    shutil.copyfile(a.tarball,ctx/'bundle.tgz')
    call('node',here/'scope-models.mjs',ctx/'inputs',*(['--instances'] if a.instances else []))
    for f in [here/'baseline.py',here/'scope_trial.py',here.parents[2]/'skills/colocad/scripts/assembly.mjs']:shutil.copy(f,ctx)
    if a.candidate: shutil.copyfile(a.candidate,ctx/'candidate'); (ctx/'candidate').chmod(0o755)
    (ctx/'Dockerfile').write_text('ARG BASE\nFROM ${BASE}\nADD bundle.tgz /opt/\nCOPY inputs/ /inputs/\nCOPY baseline.py scope_trial.py assembly.mjs /bench/\n'+('COPY candidate /opt/candidate\n' if a.candidate else ''))
    img=call('docker','build','-q','--network=none','--build-arg','BASE='+a.image,t).splitlines()[-1]
env=['BASELINE_PYTHON='+a.python,'RAYON_NUM_THREADS=1','OMP_NUM_THREADS=1','OPENBLAS_NUM_THREADS=1','MKL_NUM_THREADS=1','NUMEXPR_NUM_THREADS=1','XDG_CACHE_HOME=/tmp','MPLCONFIGDIR=/tmp/mpl','PYTHONDONTWRITEBYTECODE=1']
docker=['docker','run','--rm','--cpus','1','--memory','512m','--memory-swap','512m','--pids-limit','64','--read-only','--network','none','--cap-drop','ALL','--security-opt','no-new-privileges','--user','1000:1000','--tmpfs','/tmp:rw,nosuid,size=128m','-v',str(a.out.resolve())+':/evidence']
for e in env:docker+=['-e',e]
r={'environment':{'base_image':call('docker','image','inspect','--format','{{.Id}}',a.image),'tarball_sha256':sha(a.tarball),'scope':'one fresh process & cgroup per trial; wall to process exit; whole-cgroup peak; preview off'},'runs':[]}
cases = ['links-1','links-4','links-16','links-43','pins-16'] if a.instances else ['finger','hand']
if a.candidate:
    r['environment']['candidate_sha256']=sha(a.candidate)
    if a.instances: cases += ['hand']
for i in range(6):
    for j,case in enumerate(cases):
        arms=['colocad','build123d'] if (i+j)%2 else ['build123d','colocad']
        if a.candidate: arms.insert((i+j)%3,'candidate')
        for arm in arms:
            row=json.loads(call(*docker,'--entrypoint','python3',img,'/bench/scope_trial.py',arm,case,str(i)))
            r['runs'].append(row);(a.out/'scope.json').write_text(json.dumps(r,indent=2)+'\n')
            print(i,case,arm,round(row['wall_ms']),flush=True)
