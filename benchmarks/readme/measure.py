"""One run, from inside a fresh cgroup v2 container. Only the CAD call is timed."""
import base64
import json
import os
import subprocess
import sys
import time
from pathlib import Path

backend, case, save = sys.argv[1:]
data = Path('/opt/package/examples') / (case + '.json')
out = Path('/tmp/result')
step = out/'part/model.step' if backend == 'colocad' else Path('/tmp/model.step')
cmd = (['node', '/opt/package/bin/colocad.mjs', 'build', str(data), str(out)] if backend == 'colocad'
       else [os.environ['BASELINE_PYTHON'], '-I', '/opt/bench/baseline.py', str(data), str(step)])
start = time.perf_counter_ns()
run = subprocess.run(cmd, text=True, capture_output=True)
wall = (time.perf_counter_ns()-start)/1e6
cgroup = Path('/sys/fs/cgroup')
row = {'wall_ms': wall, 'peak_bytes': int((cgroup/'memory.peak').read_text()), 'exit': run.returncode,
       'oom_kill': int(dict(line.split() for line in (cgroup/'memory.events').read_text().splitlines())['oom_kill'])}
if run.returncode or not step.is_file():
    raise RuntimeError((row, run.stderr[-1000:]))
if backend == 'colocad':
    row['geometry_ms'] = json.loads((out/'part/report.json').read_text())['timing_ms']['geometry']
else:
    row.update(json.loads(run.stdout.splitlines()[-1]))
if save == 'step':
    row['step_b64'] = base64.b64encode(step.read_bytes()).decode()
print(json.dumps(row))
