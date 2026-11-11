"""Matched Linux arm64 builds; the alpha.9 workload, monitor and OCCT verifier are unchanged."""
import argparse, hashlib, json, subprocess
from pathlib import Path
p=argparse.ArgumentParser()
p.add_argument('output',type=Path)
p.add_argument('--baseline-package',required=True,type=Path)
p.add_argument('--candidate-binary',type=Path)
p.add_argument('--variants',nargs='+',choices=['before','after'],default=['before','after'])
p.add_argument('--repeats',type=int,default=3)
p.add_argument('--cases',nargs='+',default=['part:16','unique:1','part:256','unique:16','part:1024','unique:64','repeated:128','chain:64','repeated:1','repeated:32','chain:8','chain:32'])
p.add_argument('--image',default='colocad-memory:20261111')
a=p.parse_args()
if 'after' in a.variants and (a.candidate_binary is None or not a.candidate_binary.is_file()):p.error('--candidate-binary must name the candidate Linux arm64 executable')
if a.repeats<1:p.error('--repeats must be positive')
root=Path(__file__).resolve().parents[2];out=a.output.resolve();out.mkdir(parents=True,exist_ok=True)
def digest(path):
    h=hashlib.sha256()
    with path.open('rb') as f:
        for block in iter(lambda:f.read(1<<20),b''):h.update(block)
    return h.hexdigest()
for rep in range(a.repeats):
    for case in a.cases:
        family,n=case.split(':')
        for v in (a.variants if rep%2==0 else a.variants[::-1]):
            label=f'{family}-{n}-{v}-{rep}';dest=out/label;dest.mkdir(exist_ok=True)
            if (dest/'comparison.json').exists():continue
            pkg=(a.baseline_package if v=='before' else root).resolve()
            cmd=['docker','run','--rm','--network','none','--cpus','1','--memory','512m','--memory-swap','512m','--pids-limit','128','-e','CASE_TIMEOUT=240','-v',f'{pkg}:/pkg:ro','-v',f'{root}/benchmarks/memory:/bench:ro','-v',f'{dest}:/out']
            binary=(pkg/'kernel/native/linux-arm64/agent-cad') if v=='before' else a.candidate_binary.resolve()
            cmd+=['-v',f'{binary}:/cad-kernel:ro','-e','AGENT_CAD_BINARY=/cad-kernel']
            if not (dest/'measurement.json').exists():subprocess.run(cmd+[a.image,'python','/bench/monitor.py','node','/bench/workload.mjs',family,n,'0','/out','false'],check=True)
            m=json.loads((dest/'measurement.json').read_text());assert m['status']=='ok',label
            g=m['result']['data']; step=next(dest.rglob('model.step')); sha=digest(step)
            assert sha==g['artifact_sha256']['model.step']
            # One retained specimen per content hash; preserve all raw memory traces.
            archive=out/(sha+'.step')
            if archive.exists():step.unlink()
            else:step.rename(archive)
            row={'id':label,'family':family,'n':int(n),'rep':rep,'variant':v,'path':str(archive),'sha256':sha,'input_sha256':digest(dest/'input.json'),'seconds':m['wall_s'],'cgroup_cpu_s':m['cpu']['usage_usec']/1e6,'peak_mib':m['peak_bytes']/2**20,'timing_ms':g['timing_ms'],'live':g['peak_live_feature_bodies'],'early':g.get('early_released_definitions'),'validation':g['validation'],'geometry':g['geometry']}
            (dest/'comparison.json').write_text(json.dumps(row,indent=2)+'\n');print(label,round(row['seconds'],2),round(row['peak_mib'],2),flush=True)
