"""Record one preselected fresh-process build per backend after separate warm-ups."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile
import threading
import time


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def capture(backend, keep_step):
    cgroup = Path('/sys/fs/cgroup')
    source = Path('/opt/package/examples/manifold.json')
    python = os.environ['BASELINE_PYTHON']
    if backend == 'colocad':
        step = Path('/tmp/result/part/model.step')
        command = ['node', '/opt/package/bin/colocad.mjs', 'build', source, '/tmp/result']
    else:
        step = Path('/tmp/model.step')
        command = [python, '-I', '/opt/bench/baseline.py', source, step]

    samples = []
    stop = threading.Event()
    start_sampling = threading.Event()
    def sample_memory():
        start_sampling.wait()
        while not stop.wait(0.020):
            used = int((cgroup / 'memory.current').read_text())
            samples.append([(time.perf_counter_ns() - start) / 1e6, used])

    sampler = threading.Thread(target=sample_memory)
    sampler.start()
    initial = int((cgroup / 'memory.current').read_text())
    start = time.perf_counter_ns()
    start_sampling.set()
    try:
        result = subprocess.run(command, capture_output=True, text=True, timeout=90)
        elapsed = (time.perf_counter_ns() - start) / 1e6
        peak = int((cgroup / 'memory.peak').read_text())
    finally:
        stop.set()
        sampler.join()
    if result.returncode or not step.is_file():
        raise RuntimeError(result.stderr or 'missing STEP output')
    if keep_step:
        shutil.copyfile(step, Path('/evidence') / f'manifold-{backend}.step')
    return {
        'backend': backend, 'wall_ms': elapsed, 'peak_bytes': peak,
        'samples_ms_bytes': [[0, initial]] + [pair for pair in samples if pair[0] < elapsed],
        'step_sha256': digest(step), 'input_sha256': digest(source),
        'cpu_max': (cgroup / 'cpu.max').read_text().strip(),
        'memory_max': int((cgroup / 'memory.max').read_text()),
        'arch': platform.machine(),
    }


def record(args):
    args.out.mkdir(parents=True, exist_ok=False)
    benchmark = Path(__file__).resolve().parents[1]
    with tempfile.TemporaryDirectory() as tmp:
        context = Path(tmp)
        shutil.copyfile(args.tarball, context / 'bundle.tgz')
        shutil.copy(__file__, context)
        shutil.copy(benchmark / 'baseline.py', context)
        (context / 'Dockerfile').write_text(
            'ARG BASE\nFROM ${BASE}\nADD bundle.tgz /opt/\n'
            'COPY trace.py baseline.py /opt/bench/\n'
        )
        image = subprocess.check_output([
            'docker','build','-q','--network=none','--build-arg','BASE='+args.image,str(context)
        ], text=True).strip().splitlines()[-1]
    flags = ['docker','run','--rm','--network','none','--cpus','1','--memory','512m',
             '--memory-swap','512m','--read-only','--pids-limit','64','--user','1000:1000',
             '--cap-drop','ALL','--security-opt','no-new-privileges','--tmpfs','/tmp:rw,nosuid,size=64m',
             '-v',str(args.out.resolve())+':/evidence']
    env = {'BASELINE_PYTHON': args.python, 'OMP_NUM_THREADS': '1', 'MKL_NUM_THREADS':'1',
           'RAYON_NUM_THREADS':'1','OPENBLAS_NUM_THREADS':'1','NUMEXPR_NUM_THREADS':'1',
           'MPLCONFIGDIR':'/tmp/mpl','XDG_CACHE_HOME':'/tmp','PYTHONDONTWRITEBYTECODE':'1'}
    for name, value in env.items():
        flags += ['-e',name+'='+value]
    def run(backend, keep=False):
        cmd = flags + ['--entrypoint','python3',image,'/opt/bench/trace.py','capture',backend]
        return json.loads(subprocess.check_output(cmd + (['--keep'] if keep else []),text=True))
    warmups = [run(name) for name in ('build123d','colocad')]
    recorded = [run(name,True) for name in ('colocad','build123d')]
    data = {'case':'manifold','sampling_target_ms':20,'tarball_sha256':digest(args.tarball),
            'base_image':subprocess.check_output(['docker','image','inspect','--format','{{.Id}}',args.image],text=True).strip(),
            'warmup_wall_ms':{row['backend']:row['wall_ms'] for row in warmups},'recorded':recorded}
    (args.out / 'trace.json').write_text(json.dumps(data,indent=2)+'\n')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    modes = parser.add_subparsers(dest='mode',required=True)
    capture_parser = modes.add_parser('capture')
    capture_parser.add_argument('backend',choices=('colocad','build123d'))
    capture_parser.add_argument('--keep',action='store_true')
    host = modes.add_parser('record')
    host.add_argument('image')
    host.add_argument('tarball',type=Path)
    host.add_argument('out',type=Path)
    host.add_argument('--python',default='/home/agent/workspace/.structure-runtime/venv/bin/python3')
    args=parser.parse_args()
    if args.mode=='capture':
        print(json.dumps(capture(args.backend,args.keep)))
    else:
        record(args)
