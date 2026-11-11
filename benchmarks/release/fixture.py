"""Container entry point. Each backend constructs geometry from the same JSON."""
import json,os,subprocess,sys
from pathlib import Path
backend,case=sys.argv[1:];out=Path('/out');src=Path('/inputs',case+'.json');model=json.loads(src.read_bytes())
if backend=='colocad':
    if 'assembly' in model:
        pkg=Path('/tmp/consumer/node_modules/@coloopx');pkg.mkdir(parents=True);(pkg/'colocad').symlink_to('/pkg')
        cmd=['node','/pkg/skills/colocad/scripts/build.mjs',src,out/'build'];cwd='/tmp/consumer'
    else:cmd=['node','/pkg/bin/colocad.mjs','build',src,out/'build'];cwd='/tmp'
else:
    script=('/pkg/benchmarks/release/cq-fixture.py' if backend=='cadquery' else
            '/pkg/showcase/tendon-hand/bench/baseline.py' if 'assembly' in model else '/pkg/benchmarks/readme/baseline.py')
    cmd=[{'cadquery':'/opt/cq/bin/python','build123d':'/opt/bd/bin/python'}[backend],script,src,out/'model.step'];cwd='/tmp'
# waitid(WNOWAIT) keeps the exited child's /proc entry readable. Its I/O
# includes descendants it waited for (Node waits for its Rust process).
with open('/tmp/fixture.stdout','w') as log:
    child=subprocess.Popen(cmd,cwd=cwd,stdout=log)
    os.waitid(os.P_PID,child.pid,os.WEXITED|os.WNOWAIT)
    io={k:int(v) for k,v in (s.split(':') for s in Path(f'/proc/{child.pid}/io').read_text().splitlines())}
    if child.wait():raise RuntimeError('backend failed')
step,=out.rglob('model.step')
(out/'result.json').write_text(json.dumps({'ok':True,'step_path':step.relative_to(out).as_posix(),'step_bytes':step.stat().st_size,'process_io':io}))
