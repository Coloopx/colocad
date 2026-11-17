"""Add the work-order and revision-history readbacks to a newly generated offline scorecard."""
import base64,html,json,sys
from pathlib import Path
dest=Path(sys.argv[1]);page=dest/'scorecard.html';s=page.read_text()
assert 'id="engineering-results"' not in s and 'id="download"' in s
def gif(name,alt):
    encoded=base64.b64encode((dest/name).read_bytes()).decode()
    return f'<figure><img src="data:image/gif;base64,{encoded}" width="100%" alt="{alt}"></figure>'
e=json.loads((dest/'engineering.json').read_text());shapes=json.loads((dest/'shapes.json').read_text())
extra='<section id="engineering-results"><h2>Change orders on two installations</h2><p>Each of the three backends delivered seven valid assemblies and rejected three incompatible inputs. The source saddle is a catalog reconstruction. Panels show independent STEP readbacks.</p>'+gif('engineering-changes.gif','Two engineering tasks with three backend results')+'</section>'
extra+='<h2>Working memory while saving 256 edits</h2><p>One persistent session per implementation. Every edit saves a reopenable assembly STEP and a preview. Python uses an explicit SQLite ledger, fsync and Linux cache advice. Colocad also saves native checkpoints and exact query snapshots.</p>'+gif('revision-memory.gif','Memory aligned by edit number')+'<section><table><tr><th>Implementation<th>Peak MiB<th>Current at edit 128<th>Current at edit 256</tr>'
if 'initial_sessions' in e:
    extra=extra.replace('One persistent session per implementation.', 'Three warmed sessions per implementation. Lines show medians and shading shows the range. The initial run is included separately in the download.')
for key,label in [('colocad','Colocad '+e['campaign']['label']),('cadquery','CadQuery'),('build123d','build123d'),('alpha10','Colocad alpha.10')]:
    if key not in e['sessions']:continue
    row=e['sessions'][key];values={r['edits']:r for r in row['checkpoints']}
    extra+=f'<tr><td>{label}<td>{row["peak_mib"]:.1f}<td>{values[128]["current_mib"]:.1f}<td>{values[256]["current_mib"]:.1f}</tr>'
extra+='</table></section><p><button id="workdata">Download engineering evidence (JSON)</button></p>'
if 'initial_sessions' in e:
    growth=base64.b64encode((dest/'revision-growth.svg').read_bytes()).decode()
    extra+=f'<figure><img src="data:image/svg+xml;base64,{growth}" width="100%" alt="Paired memory growth after edit 128, with three-session ranges"></figure>'
    extra+='<p>Subtract each session’s reading at edit 128 before finding the median. These changes describe this 128-edit interval.</p>'
if (dest/'appearance.json').is_file():
    appearance=json.loads((dest/'appearance.json').read_text())
    extra+='<h2>Five product appearance models</h2><p>macOS arm64. Each cell lists Colocad / CadQuery. Timings are medians of three rebuilds after one warm-up. Peak RSS was sampled across the fresh process tree.</p>'
    extra+=gif('appearance.gif','Four views of five product assemblies reopened from their STEP files')
    extra+='<section><table><tr><th>Product<th>Fresh to STEP (s)<th>Peak RSS (MiB)<th>Loaded host to STEP (s)</tr>'
    for case,backends in appearance['runs'].items():
        cols=[]
        for metric in ('wall','rss','loaded'):
            values=[appearance['warmed_process'][case][b]['median'] if metric=='loaded' else backends[b][metric] for b in ('colocad','cadquery')]
            cols.append(' / '.join(format(v,'.1f' if metric=='rss' else '.2f') for v in values))
        extra+='<tr><td>'+html.escape(case)+'</td>'+''.join('<td>'+cell+'</td>' for cell in cols)+'</tr>'
    extra+='</table></section><p>Geometry checks cover 80 exports. Both timed backends use the same circular substitution for the SenseCAP base. The specified ellipse failed Colocad validation. Of 139 declared dimensions, 109 met the 0.25 mm comparison threshold.</p>'
extra+='<h2>Six shape controls</h2><p>Separate macOS arm64 run, Colocad and build123d 0.11.1, five timed processes per backend and shape. Images are independent STEP readbacks.</p><label>Shape <select id="shape">'
for key,row in shapes['cases'].items():extra+=f'<option value="{key}">{html.escape(row["title"])}</option>'
extra+='</select></label>'
for i,(key,row) in enumerate(shapes['cases'].items()):
    assert row['acceptance']['geometry_pass']
    extra+=f'<div class="shape-pair" data-shape="{key}"'+(' hidden' if i else '')+'>'
    for b,label in [('rust','Colocad'),('build123d','build123d')]:
        v=row['backends'][b];extra+=f'<figure><img src="data:image/png;base64,{v["png"]}" alt="{label} STEP"><figcaption>{label} · {v["metrics"]["wall_median_ms"]/1000:.3f} s · {v["metrics"]["peak_rss_max_mib"]:.1f} MiB maximum RSS</figcaption></figure>'
    extra+='</div>'
payload=json.dumps(e,separators=(',',':')).replace('<','\\u003c')
extra+=f'<script id="engineering-data" type="application/json">{payload}</script>'
extra+='''<script>document.getElementById('shape').onchange=event=>{for(const panel of document.querySelectorAll('[data-shape]'))panel.hidden=panel.dataset.shape!==event.target.value};document.getElementById('workdata').onclick=()=>{let url=URL.createObjectURL(new Blob([document.getElementById('engineering-data').textContent],{type:'application/json'})),a=document.createElement('a');a.href=url;a.download='colocad-engineering.json';a.click();setTimeout(()=>URL.revokeObjectURL(url),1000)}</script>'''
s=s.replace('<h2>One 100-second queue',extra+'<h2>One 100-second queue').replace('</style>','select{font:inherit;padding:.4em}.shape-pair{display:flex;flex-wrap:wrap;gap:10px}.shape-pair[hidden]{display:none}.shape-pair figure{margin:10px 0;padding:6px;max-width:100%}.shape-pair img{max-width:100%}</style>')
page.write_text(s)
