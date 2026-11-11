"""Identical parameter queues, a fresh process per hand, one 100 s window per backend."""
import json,subprocess,sys
from pathlib import Path
out=Path(sys.argv[1]).resolve();root=Path(__file__).resolve().parents[2]
directory=out/'windows';directory.mkdir(exist_ok=True)
for b in ('cadquery','colocad','build123d'):
    if (directory/b/'environment.json').exists():continue
    interpreter='/opt/cq/bin/python' if b=='cadquery' else '/opt/bd/bin/python'
    baseline=root/('benchmarks/release/cq-fixture.py' if b=='cadquery' else 'showcase/tendon-hand/bench/baseline.py')
    cmd=['docker','run','--rm','--network','none','--cpus','1','--memory','512m','--memory-swap','512m','--pids-limit','128','-e','BASELINE_PYTHON='+interpreter,'-v',f'{root}:/opt/package:ro','-v',f'{root}/showcase/tendon-hand/bench/window.py:/bench/window.py:ro','-v',f'{baseline}:/bench/baseline.py:ro','-v',f'{root}/skills/colocad/scripts/build.mjs:/bench/build.mjs:ro','-v',f'{out}/inputs/hand.json:/inputs/drive.json:ro','-v',f'{directory}:/outputs','colocad-memory:20261111','python','/bench/window.py',b,'100']
    result=subprocess.run(cmd,capture_output=True,text=True,timeout=300);(directory/(b+'.log')).write_text(result.stdout+result.stderr)
    if result.returncode:print(b,'failed',result.returncode,flush=True)
    else:print(b,'complete',flush=True)
