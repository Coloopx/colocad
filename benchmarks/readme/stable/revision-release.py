"""Publish the three-backend saved-revision experiment from verified fresh rounds."""
import argparse
import base64
import hashlib
import json
import subprocess
import sys
from html import escape
from pathlib import Path
from statistics import median
from revision_stats import checked_session, summarize_rounds

parser=argparse.ArgumentParser()
parser.add_argument('history',type=Path)
parser.add_argument('runtime',type=Path,help='Extracted npm package used for every Colocad run')
parser.add_argument('output',type=Path)
args=parser.parse_args()
out=args.output.resolve()
out.mkdir(parents=True,exist_ok=True)
here=Path(__file__).resolve().parent
def digest(path):
    with path.open('rb') as f:
        return hashlib.file_digest(f,'sha256').hexdigest()
def get(*cmd):
    return subprocess.check_output(cmd,text=True).strip()
def filecounts(root):
    files=dirs=0
    for path in root.rglob('*'):
        if path.is_file():files+=1
        elif path.is_dir():dirs+=1
    return {'files':files,'directories':dirs}

runtime=args.runtime
version=json.loads((runtime/'package.json').read_bytes())['version']
image='colocad-memory:20261111'
def guest(*cmd):
    return get('docker','run','--rm','--network','none',image,*cmd)
names={'colocad':f'Colocad {version}'}
for key,venv in [('cadquery','cq'),('build123d','bd')]:
    ver=guest(f'/opt/{venv}/bin/python','-c',f'from importlib.metadata import version; print(version("{key}"))')
    names[key]=('CadQuery' if key=='cadquery' else 'build123d')+' '+ver

# Fingerprint executable application content independently of release notes and plots.
manifest={}
for folder in ['bin','src','kernel','viewer']:
    for path in sorted((runtime/folder).rglob('*')):
        if path.is_file():manifest[path.relative_to(runtime).as_posix()]=digest(path)
evidence={'campaign':{'label':version,'platform':guest('uname','-sm'),'image_id':get('docker','image','inspect',image,'--format','{{.Id}}'),
                      'names':names,'runtime_files_sha256':manifest,
                      'storage':'segmented','cpu':1,'cgroup_limit_mib':512,'swap_mib':0,
                      'round_order':[['cadquery','colocad','build123d'],['colocad','build123d','cadquery'],['build123d','cadquery','colocad']]},
          'sessions':{}}
folders=[args.history/f'round-{n}' for n in (1,2,3)]
inputs=[{f.name:digest(f) for f in p.joinpath('inputs').glob('*.step')} for p in folders]
assert len(inputs[0])==3 and all(x==inputs[0] for x in inputs)
evidence['input_sha256']=inputs[0]
for backend in names:
    rounds=[]
    for folder in folders:
        r=checked_session(folder,backend)
        assert r['preview_hashes']==256 and r['preview_bounds']==6 and 'latency_ms' in r
        assert len({e['sha256'] for e in r['edits']})==256
        if backend=='colocad':
            root=folder/'colocad/session'
            assert json.loads((root/'session.json').read_bytes())['format']=='colocad/session/4'
            r['layout']=filecounts(root)
            assert r['archive_audit']['accepted']==260 and len(r['archive_audit']['oldSnapshots'])==6
        rounds.append(r)
    summary=summarize_rounds(rounds)
    summary['last_128_slab_mib']=median(r['last_128_slab_mib'] for r in rounds)
    summary['latency_ms']={k:median(r['latency_ms'][k] for r in rounds) for k in ['p50','p95','total_s']}
    evidence['sessions'][backend]=summary
(out/'engineering.json').write_text(json.dumps(evidence,separators=(',',':'))+'\n')
for script in [here.parent/'candidate/memory-gif.py',here/'growth.py']:
    subprocess.run([sys.executable,script,out],check=True)

def embed(name,mime):
    data=base64.b64encode((out/name).read_bytes()).decode()
    return f'<img width="960" alt="{escape(name)}" src="data:{mime};base64,{data}">'
rows=[]
for name,s in evidence['sessions'].items():
    for i,r in enumerate(s['rounds'],1):
        rows.append(f"<tr><td>{escape(names[name])}</td><td>{i}</td><td>{r['peak_mib']:.2f}</td>"
                    f"<td>{r['last_128_growth_mib']:+.2f}</td><td>{r['last_128_slab_mib']:+.2f}</td>"
                    f"<td>{r['latency_ms']['p95']:.1f}</td><td>{r['latency_ms']['total_s']:.1f}</td></tr>")
(out/'scorecard.html').write_text(f"""<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>KK60 revision history · Colocad {escape(version)}</title><style>body{{max-width:960px;margin:3rem auto;padding:0 1rem;font:16px/1.55 system-ui;color:#192f36;background:#f8f9f6}}img{{max-width:100%;height:auto}}table{{border-collapse:collapse;width:100%;font-variant-numeric:tabular-nums}}td,th{{text-align:right;border-bottom:1px solid #ccd7d5;padding:.45rem}}td:first-child,th:first-child{{text-align:left}}.scroll{{overflow-x:auto}}</style>
<h1>KK60 assembly · 256 saved edits</h1><p>Three rounds per backend. Each session had one vCPU, 512 MiB, zero swap and a new Linux volume. Colocad used segmented storage. All 2,304 edited STEP deliveries passed the same independent geometry checks.</p>
{embed('revision-memory.gif','image/gif')}{embed('revision-growth.svg','image/svg+xml')}
<h2>Individual trials</h2><div class="scroll"><table><thead><tr><th>Backend</th><th>Round</th><th>Peak MiB</th><th>Current Δ MiB</th><th>Slab Δ MiB</th><th>Edit p95 ms</th><th>256 edits s</th></tr></thead><tbody>{''.join(rows)}</tbody></table></div>
<p>Δ subtracts edit 128 from edit 256 in the same session. Slab is the kernel's object cache. Peak includes allocations between samples. Latency runs from rebuilding the edited geometry through the durable save. Preview tolerances differ by backend. Colocad also saves exact query snapshots and native checkpoints.</p></html>""")
