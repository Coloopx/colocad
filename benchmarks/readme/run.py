"""Stage the published tarball, alternate backends, keep every observation and STEP pair."""
import argparse
import base64
import hashlib
import json
import shutil
import statistics
import subprocess
import tempfile
from pathlib import Path

here = Path(__file__).resolve().parent
p = argparse.ArgumentParser()
p.add_argument('--tarball', type=Path, required=True)
p.add_argument('--image', required=True, help='Linux image with Node 22+, Python 3, build123d and cgroup v2')
p.add_argument('--python', required=True, help='build123d interpreter path inside the image')
p.add_argument('--out', type=Path, required=True)
p.add_argument('--repeats', type=int, default=5)
a = p.parse_args()
a.out.mkdir(parents=True, exist_ok=False)
def call(*cmd):
    return subprocess.check_output(cmd, text=True).strip()
def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

with tempfile.TemporaryDirectory() as tmp:
    ctx = Path(tmp)
    shutil.copyfile(a.tarball, ctx/'bundle.tgz')
    for name in ('baseline.py', 'measure.py'):
        shutil.copyfile(here/name, ctx/name)
    (ctx/'Dockerfile').write_text('ARG BASE\nFROM ${BASE}\nADD bundle.tgz /opt/\nCOPY baseline.py measure.py /opt/bench/\n')
    tag = call('docker','build','--network=none','--build-arg','BASE='+a.image,'-q',tmp).splitlines()[-1]

env = ['BASELINE_PYTHON='+a.python, 'PYTHONDONTWRITEBYTECODE=1', 'OMP_NUM_THREADS=1',
       'OPENBLAS_NUM_THREADS=1','MKL_NUM_THREADS=1','RAYON_NUM_THREADS=1','NUMEXPR_NUM_THREADS=1',
       'XDG_CACHE_HOME=/tmp','MPLCONFIGDIR=/tmp/mpl']
docker = ['docker','run','--rm','--network','none','--cpus','1','--memory','512m','--memory-swap','512m',
          '--pids-limit','64','--read-only','--user','1000:1000','--cap-drop','ALL',
          '--security-opt','no-new-privileges','--tmpfs','/tmp:rw,nosuid,size=64m']
for e in env:
    docker += ['-e',e]
def invoke(program, *cmd):
    return call(*docker,'--entrypoint',program,tag,*cmd)
def package_ver():
    return json.loads(invoke(a.python,'-c',"""import json, platform, importlib.metadata as m
print(json.dumps(dict(arch=platform.machine(),python=platform.python_version(),build123d=m.version('build123d'),ocp=m.version('cadquery-ocp-novtk'))))"""))
meta = {'base_image_id':call('docker','image','inspect','--format','{{.Id}}',a.image),
        'tarball_sha256':sha(a.tarball),'package':json.loads(invoke('cat','/opt/package/package.json'))['version'],
        'node':invoke('node','-v'), **package_ver(), 'cpu_limit':1,'memory_limit_mib':512,
        'source_sha256':{name:sha(here/name) for name in ('run.py','measure.py','baseline.py')}}
cases = ('sensor-rail','manifold')
backends = ('colocad','build123d')
rows = []
for rep in range(a.repeats+1):
    for ix, case in enumerate(cases):
        for b in (backends if (rep+ix)%2 else backends[::-1]):
            row = json.loads(invoke('python3','/opt/bench/measure.py',b,case,'step' if rep==1 else 'skip'))
            if 'step_b64' in row:
                dest = a.out/(case+'-'+b+'.step')
                dest.write_bytes(base64.b64decode(row.pop('step_b64')))
                row['step_sha256'] = sha(dest)
            rows.append(dict(rep=rep,case=case,backend=b,**row))
            (a.out/'runs.json').write_text(json.dumps({'environment':meta,'runs':rows},indent=2)+'\n')
            print(case,b,rep,round(row['wall_ms']),round(row['peak_bytes']/2**20,1),flush=True)
