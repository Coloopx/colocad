"""Publish charts from verified release observations only (standard library)."""
import argparse,collections,hashlib,html,json,math,statistics
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('dest',type=Path);a=p.parse_args();d=a.root;dest=a.dest;dest.mkdir(parents=True,exist_ok=True)
def read(path):return json.loads(path.read_text())
def save(name,obj):(dest/name).write_text(json.dumps(obj,indent=2,allow_nan=False)+'\n')
def median(xs):return statistics.median(xs)
# Historical campaigns predate the identity file. A new checkout records its
# executable hashes and source commit before rendering any result.
identity_file = d/'campaign.json'
identity = read(identity_file) if identity_file.is_file() else {
    'label':'0.1.0-alpha.9', 'engine_commit':'16c0f57f4de422fedc15842e7344cb0f011d0015',
    'artifact_slug':'alpha9', 'runtime_kind':'release',
}
label = identity['label']
B=['colocad','cadquery','build123d'];labels=['Colocad','CadQuery','build123d'];colors=['#006a66','#85541d','#534e9d']
M=2**20
measured=[];summaries=[]
for suite in ('main','fixtures'):
    data=read(d/suite/'runs.json');rows=data['runs'] if suite=='main' else data
    verified=read(d/(suite+'-verified.json'));v={r['id']:r for r in verified}
    assert len(rows)==(108 if suite=='main' else 99) and len({r['id'] for r in rows})==len(rows)
    for r in rows:
        name=(r['family']+'/'+str(r['n'])) if suite=='main' else r['case'];ok=r['status']=='ok';receipt=r.get('result',{});io=receipt.get('process_io',{})
        if ok:assert r['id'] in v,(suite,r['id'])
        measured.append(dict(suite=suite,case=name,id=r['id'],backend=r['backend'],rep=r['rep'],status=r['status'],wall_s=r.get('wall_s'),peak_mib=r.get('peak_bytes',0)/M,step_bytes=v[r['id']]['step_bytes'] if ok and suite=='main' else receipt.get('step_bytes'),retained_bytes=r.get('retained_output_bytes'),process_io=io,sha256=v[r['id']]['sha256'] if ok else None,verified=ok and r['id'] in v))
    for case in sorted({r['case'] for r in measured if r['suite']==suite}):
        by={}
        for b in B:
            allrows=[r for r in measured if (r['suite'],r['case'],r['backend'])==(suite,case,b)];rs=[r for r in allrows if r['verified']]
            record={'verified':len(rs),'scheduled':len(allrows),'statuses':dict(collections.Counter(r['status'] for r in allrows))}
            for key in ('wall_s','peak_mib','step_bytes','retained_bytes','rchar','wchar','syscr','syscw','read_bytes','write_bytes'):
                nums=[r.get(key,r['process_io'].get(key)) for r in rs];nums=[n for n in nums if n is not None]
                record[key]=dict(median=median(nums),min=min(nums),max=max(nums)) if nums else None
            by[b]=record
        summaries.append(dict(suite=suite,case=case,backends=by))
assert len(summaries)==23
window={}
wv={r['id']:r for r in read(d/'windows-verified.json')}
for b in B:
    wind=d/'windows'/b
    if not (wind/'environment.json').exists():
        window[b]=dict(status='window_failed');continue
    env=read(wind/'environment.json');assert env['window_s']==100 and int(dict(l.split() for l in env['oom_events'].splitlines())['oom_kill'])==0
    ev=[json.loads(s) for s in (wind/'events.jsonl').read_text().splitlines()]
    counted=[r for r in ev if r['in_window']];assert all(b+'/'+str(r['revision']) in wv for r in counted) and all(r['end_s']<=100 for r in counted)
    mem=read(wind/'memory.json');assert max(x[1] for x in mem)==env['sampled_window_peak_bytes']
    window[b]=dict(status='ok',count=len(counted),first_s=counted[0]['end_s'],peak_sample_mib=env['sampled_window_peak_bytes']/M,deliveries=[e['end_s'] for e in counted],memory=mem,source_hashes=[r['model_sha256'] for r in counted])
for k in range(min((w['count'] for w in window.values() if w['status']=='ok'),default=0)):
    assert len({w['source_hashes'][k] for w in window.values() if w['status']=='ok'})==1
payload=dict(version=label,engine_commit=identity['engine_commit'],campaign=identity,environment=read(d/'main/runs.json')['environment'],backends={'colocad':label,'cadquery':'2.8.0','build123d':'0.13.0'},observations=measured,summary=summaries,windows=window)
save('observations.json',payload)
def get(suite,key):return next(r['backends'] for r in summaries if r['suite']==suite and r['case']==key)
def values(key,field):
    suite='main' if '/' in key else 'fixtures'
    return [(get(suite,key)[b][field] or {}).get('median') for b in B]
def score(field):
    rows=[r['backends'] for r in summaries if r['suite']=='main' and all(r['backends'][b]['verified']==3 for b in B)]
    return sum(r[B[0]][field]['median']<min(r[b][field]['median'] for b in B[1:]) for r in rows)
def svg(title,subtitle,rows,metrics):
    W=1080;H=167+len(rows)*50+53
    buf=[f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" role="img" aria-label="{html.escape(title)}"><rect width="100%" height="100%" fill="#f6f8f5"/><g font-family="Arial,Helvetica,sans-serif" fill="#192f36">']
    def t(x,y,s,size=16,bold=False,fill='#192f36',anchor='start'):
        buf.append(f'<text x="{x}" y="{y}" font-size="{size}" font-weight="{700 if bold else 400}" fill="{fill}" text-anchor="{anchor}">{html.escape(str(s))}</text>')
    t(27,30,'Colocad '+label+'  /  CadQuery 2.8.0  /  build123d 0.13.0',13,True,'#006a66')
    t(27,69,title,31,True);t(27,95,subtitle,15,False,'#435963')
    colw=(W-218)/len(metrics)
    for j,(name,field,div) in enumerate(metrics):
        xx=218+j*colw;t(xx,127,name,17,True)
        for k,l in enumerate(labels):t(xx+k*(colw/3),149,l,13,True,colors[k])
    for i,(key,desc) in enumerate(rows):
        yy=165+50*i;buf.append(f'<rect x="14" y="{yy}" width="1050" height="46" rx="5" fill="{["#ffffff","#eef2ee"][i%2]}"/>');t(27,yy+28,desc,17,True)
        for j,(_,field,div) in enumerate(metrics):
            nums=values(key,field);scaled=[v/div if v is not None else None for v in nums];maxv=max([v for v in scaled if v is not None] or [1])
            for k,v in enumerate(scaled):
                xx=218+j*colw+k*colw/3
                if v is None:t(xx,yy+26,'—');continue
                t(xx,yy+22,(f'{v:.2f}' if v<100 else f'{v:,.1f}'),16,False,colors[k]);buf.append(f'<rect x="{xx}" y="{yy+29}" width="{max(1,(colw/3-17)*v/(maxv or 1)):.2f}" height="4" rx="2" fill="{colors[k]}"/>')
    t(27,H-17,'Three fresh processes per cell. Bars share a scale within each row and metric. Lower is smaller. Full ranges in the HTML.',13,False,'#435963')
    return ''.join(buf)+'</g></svg>'
primary=[('sensor-rail','Sensor rail'),('manifold','Port manifold'),('finger','Finger link'),('hand','43-solid hand'),('repeated/128','128 shared instances'),('unique/64','64 distinct plates'),('part/1024','1,024-hole solid'),('chain/64','64 sequential cuts')]
chart=svg('Time to STEP and peak sandbox memory', 'Native Linux arm64 · 1 vCPU · fresh tools · median elapsed seconds and median cgroup peak MiB',primary,[('Time to STEP  /  s','wall_s',1),('Peak memory  /  MiB','peak_mib',1)])
iochart=svg('One STEP delivery, three measures of I/O','Logical writes include checkpoints, receipts and tool output. Device traffic depends on mount and page cache.',primary[:4],[('STEP  /  KiB','step_bytes',1024),('Retained files  /  KiB','retained_bytes',1024),('Logical writes  /  KiB','wchar',1024)])
(dest/'scorecard.svg').write_text(chart+'\n');(dest/'build-io.svg').write_text(iochart+'\n')
def field(r,k,div=1):
    v=r.get(k)
    return '—' if not v else f"{v['median']/div:,.2f} <small>({v['min']/div:,.2f}–{v['max']/div:,.2f})</small>"
table=''.join('<tr><td>'+html.escape(str(r['case']))+'</td><td>'+labels[i]+'</td><td>'+str(v['verified'])+'/3</td>'+''.join('<td>'+field(v,k,div)+'</td>' for k,div in [('wall_s',1),('peak_mib',1),('step_bytes',1024),('retained_bytes',1024),('wchar',1024)])+'</tr>' for r in summaries for i,b in enumerate(B) for v in [r['backends'][b]])
wrows=''.join('<tr><td>'+labels[i]+'</td>'+(''.join(f'<td>{v}</td>' for v in [w['count'],f"{w['first_s']:.2f}",f"{w['peak_sample_mib']:.1f}"]) if w['status']=='ok' else '<td colspan="3">window failed</td>')+'</tr>' for i,b in enumerate(B) for w in [window[b]])
page='''<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><link rel="icon" href="data:,"><title>Colocad · CAD benchmark</title><style>*{box-sizing:border-box}body{font:16px/1.5 system-ui;color:#192f36;background:#f6f8f5;margin:0}main{max-width:1220px;margin:auto;padding:clamp(14px,3vw,44px)}h1{font-size:clamp(30px,5vw,58px);line-height:1.05;margin:14px 0}h2{font-size:24px;margin-top:36px}p{max-width:95ch}svg{width:100%;height:auto}figure{margin:22px 0;border:1px solid #d4dfda;border-radius:10px;overflow:auto}figure svg{min-width:670px;display:block}table{border-collapse:collapse;width:100%;font-size:14px}th,td{text-align:left;padding:7px;border-bottom:1px solid #d8e2dc;white-space:nowrap}small{color:#627177}input{padding:8px;font:inherit}section{overflow-x:auto}strong,label{color:#006a66}summary{cursor:pointer}a{color:#006a66}</style><main><h1>Time and memory by modeling task</h1>'''
page+=f'<p>Colocad {html.escape(label)}, CadQuery 2.8.0 and build123d 0.13.0. On the 12 common scaling points, Colocad had the lowest median peak memory in <b>{score("peak_mib")}/12</b> and the shortest median wall time in <b>{score("wall_s")}/12</b>. These counts give each tested size one vote and are not a distribution of real projects.</p><figure>{chart}</figure><figure>{iochart}</figure>'
page+='<h2>One 100-second queue of full hand assemblies</h2><p>Each job starts a new process. The same ordered edits change guide diameter and cable radius. Count only complete, independently reopened 43-solid STEP handoffs by the deadline. Memory is the largest 25 ms sample, not a per-job kernel peak. One window per backend.</p><section><table><tr><th>Backend<th>Accepted STEPs<th>First STEP (s)<th>Sampled peak (MiB)</tr>'+wrows+'</table></section>'
page+='<h2>All 23 comparable tasks</h2><p>Cold process with warmed host caches. Scored fixtures: 512 MiB; scaling matrix: 2 GiB; both 1 vCPU, no swap/network. Scored fixture lanes share a host with pinned stress controls. Wall times include startup, native checks and export; inference, installation, independent OCCT readback and rendering are outside timing. Values are medians (min–max); a dash means no measurement. Lower retained output is not a measure of API flexibility.</p><label>Filter <input id="filter" placeholder="e.g. cadquery or part/1024"></label><section><table><thead><tr><th>Task<th>Backend<th>Read back<th>Seconds<th>Peak MiB<th>STEP KiB<th>Retained KiB<th>Logical writes KiB</tr></thead><tbody>'+table+'</tbody></table></section><p>Logical writes come from Linux /proc/PID/io at process exit including waited-for children. Retained files include Colocad checkpoints but exclude instrumentation and supplied inputs. Warm-cache block counters on a host bind mount cannot establish physical SSD bandwidth. All three tools are tested on real edits; there is no claim of universal CAD API parity.</p><script>document.querySelector("#filter").oninput=e=>{for(const r of document.querySelectorAll("tbody tr"))r.hidden=!r.textContent.toLowerCase().includes(e.target.value.toLowerCase())}</script></main></html>'
(dest/'scorecard.html').write_text(page)
print('scored',len(measured),'common wins',score('wall_s'),score('peak_mib'),'window',{k:v.get('count') for k,v in window.items()})
