"""Add the candidate's paired experiments, curves and six STEP controls to the offline scorecard."""
import argparse,base64,html,json,statistics
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument("directory",type=Path);a=p.parse_args();root=a.directory
read=lambda name:json.loads((root/name).read_text())
e=read("observations.json");shapes=read("shapes.json");controls=read("controls.json")
public_release=e["campaign"]["runtime_kind"]=="public npm release"
page=(root/"scorecard.html").read_text()
assert 'id="rawdata"' in page and 'id="shape-selector"' not in page
def table(head,rows,tid=""):
    return '<section><table'+(f' id="{tid}"' if tid else '')+'><thead><tr>'+''.join('<th>'+html.escape(h)+'</th>' for h in head)+'</tr></thead><tbody>'+''.join('<tr>'+''.join('<td>'+html.escape(str(v))+'</td>' for v in row)+'</tr>' for row in rows)+'</tbody></table></section>'
gallery='<h2>STEP readbacks at the same camera and scale</h2><p>Six macOS arm64 controls, five fresh builds per case and backend. Colocad and build123d 0.11.1 use their original independent implementations. The figures report median wall time and maximum process RSS.</p><label for="shape-selector">Model </label><select id="shape-selector">'
for key,s in shapes["cases"].items():gallery+=f'<option value="{key}">{html.escape(s["title"])}</option>'
gallery+='</select>'
for key,s in shapes['cases'].items():
    gallery+=f'<section class="shape-pair" data-shape="{key}"'+('' if key=='plate4' else ' hidden')+'>'
    for backend,label in [('rust','Colocad'),('build123d','build123d')]:
        b=s["backends"][backend];m=b['metrics'];g=s["readback"][backend];gallery+=f'<figure><img width="350" height="175" src="data:image/png;base64,{b["png"]}" alt="{html.escape(s["title"])} {label} STEP readback"><figcaption><b>{label}</b> · {m["wall_median_ms"]/1000:.3f} s · {m["peak_rss_max_mib"]:.1f} MiB RSS<br>{g["solids"]} solids · {g["faces"]} faces</figcaption></figure>'
    gallery+='</section>'
gallery+='''<script>document.getElementById("shape-selector").onchange=e=>{for(const pair of document.querySelectorAll("[data-shape]"))pair.hidden=pair.dataset.shape!==e.target.value}</script>'''
if public_release:
    acceptance=read('acceptance.json')
    samples=acceptance['hybrid']
    extra='<h2>Hybrid modeling in a 512 MiB sandbox</h2><p>One fresh Linux process per case, including mixed STEP, GLB and offline HTML export. Memory includes charged file cache. These two builds use Colocad’s local hybrid executor.</p>'+table(
        ['Input','Time (s)','Cgroup peak (MiB)','Mixed STEP (MiB)'],
        [[name,f"{samples[key]['wall_s']:.2f}",f"{samples[key]['peak_mib']:.1f}",f"{samples[key]['step_bytes']/2**20:.2f}"] for key,name in [('housing','Spline housing + mechanism'),('scan','345,944-triangle Armadillo + probes')]])
    paired=acceptance['revision_comparison']
    extra+='<h2>Matched release comparison</h2><p>One CPU and 512 MiB. All six retained STEP digests match for each input. An independent OCCT reader checked each distinct result.</p>'+table(
        ['Case','alpha.9 time (s)','alpha.10 time (s)','alpha.9 peak (MiB)','alpha.10 peak (MiB)'],
        [[case]+[f"{row[v][metric]:.2f}" for metric in ('wall_s','peak_mib') for v in ('before','after')] for case,row in paired['cases'].items()])
    extra+='<p>Each session executed 100 edits and 9,900 queries. The audit reopened and checked every receipt.</p>'+table(
        ['Release','Session steps','Peak (MiB)','Retained files'],
        [[name,session['entries'],f"{session['peak_mib']:.1f}",session['files']] for name,session in paired['sessions'].items()])
    extra+='<h2>64 independently dimensioned plates</h2><figure><svg'+(root/'unique-phases.svg').read_text().split('<svg',1)[1]+'</figure>'
    raw={'acceptance':acceptance,'phase_trials':read('unique-phases.json')}
else:
    iteration=read('iterations.json')
    phase=iteration["unique64"]
    def median(name,key):
        rows=[r for r in phase["trials"] if r["variant"]==name and r["n"]==64]
        return statistics.median(r[key] for r in rows)
    phases=table(["64 independent plates","Median seconds","Median cgroup MiB"],[
        [name,f'{median(v,"seconds"):.2f}',f'{median(v,"peak_mib"):.2f}'] for v,name in [("before","Before the spline optimization"),("after","After the spline optimization")]])
    base={"before":iteration["alpha9_pair"]["sessions"]["before"],"after":iteration["final_session"]}
    write=table(["10,001-step project","cgroup peak MiB","Files","Verified full receipts"],[[label,f'{base[v]["peak_mib"]:.2f}',base[v]["files"],base[v]["readback"]["count"]] for v,label in [('before','alpha.9'),('after','Current candidate')]])
    extra='<h2>Assembly validation and session writes</h2>'+phases+'<p>These paired builds use 64 independently dimensioned plates with 16 bores each. Each occurrence still undergoes Level‑1 and manifold checks. Certified quadratic trim spans and reused pure spline analyses reduce repeated numerical searches. OCCT read back the same STEP bytes on both sides.</p>'+write+'<p>The separate write experiment executed 100 edits and 9,900 queries. Each recorded receipt and accepted geometry hash was checked after reopening the project.</p>'
    extra += '<figure><svg'+(root/'unique-phases.svg').read_text().split('<svg',1)[1]+'</figure>'
    raw=dict(assembly_phase_runs=read('unique-phases.json'),iterations=iteration)
extra+='<h2>Memory across all twelve scaling points</h2><figure>'+'<svg'+(root/'memory-scaling.svg').read_text().split('<svg',1)[1]+'</figure>'
extra+=gallery
counts={k:dict(x['outcomes']) for k,x in controls['campaigns'].items()}
stress_label=html.escape(e['version'] if public_release else 'source revision 9f53a4e')
extra+=f'<h2>Stress and recovery controls</h2><p>The extended campaign used {stress_label}. Timeouts, policy rejections, OOMs and jobs stopped after a boundary remain in the recorded outcomes.</p>'+table(['Suite','Scheduled','Outcomes'],[[k,controls['campaigns'][k]['planned'],', '.join(f'{status}: {count}' for status,count in value.items())] for k,value in counts.items()])
raw.update(controls=controls,shape_trials=shapes['observations'],shapes={k:{'acceptance':v['acceptance'],'readback':v['readback']} for k,v in shapes['cases'].items()})
extra+='<details><summary>Paired trials, receipts and stress outcomes (JSON)</summary><pre id="aux-values"></pre></details><script type="application/json" id="aux-data">'+json.dumps(raw,separators=(',',':')).replace('<','\\u003c')+'</script><script>document.querySelector("details:last-of-type").ontoggle=e=>{if(e.target.open)document.getElementById("aux-values").textContent=JSON.stringify(JSON.parse(document.getElementById("aux-data").textContent),null,2)}</script>'
page=page.replace('</style>','select{padding:.5em;font:inherit}.shape-pair{display:flex;gap:1em;flex-wrap:wrap}.shape-pair[hidden]{display:none}.shape-pair figure{background:white;margin:12px 0;padding:8px}.shape-pair img{max-width:100%;object-fit:contain}pre{max-height:22em;overflow:auto}</style>')
page=page.replace('<p><button id="download">',extra+'<p><button id="download">')
(root/'scorecard.html').write_text('\n'.join(line.rstrip() for line in page.splitlines())+'\n')
