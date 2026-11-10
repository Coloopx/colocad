"""Generate portable evidence, descriptive plots and a standalone HTML report."""
import argparse,csv,hashlib,html,json,statistics
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('dest',type=Path);a=p.parse_args();a.dest.mkdir(parents=True,exist_ok=True)
colors={'colocad':'#2678c9','cadquery':'#df7a24','build123d':'#168977','sdk':'#bf4b45','indexed':'#168977','indexed-node':'#754ac3'}
campaigns=['main','extreme','history','history-node','sharded','boundary','compositional','native-control'];rows=[];env={}
for campaign in campaigns:
    data=json.loads((a.root/campaign/'runs.json').read_text());env[campaign]=data['environment']
    for r in data['runs']:
        d=a.root/campaign/r['id'];stderr=(d/'stderr.log').read_text() if (d/'stderr.log').exists() else ''
        status=r['status'];error=r.get('result',{}).get('error')
        if status=='error' and ('heap out of memory' in stderr.lower() or 'allocation failed' in stderr.lower()):status='runtime_oom'
        if status=='rejected' and error and error['code'] in ('RESOURCE_LIMIT','QUERY_LIMIT'):status='policy_limit'
        events=[json.loads(l) for l in (d/'events.jsonl').read_text().splitlines()] if (d/'events.jsonl').exists() else []
        row={k:r[k] for k in ['id','backend','family','n','rep','mode','variant','batch','memory_mib'] if k in r}
        row.update(campaign=campaign,status=status,raw_status=r['status'],wall_s=r.get('wall_s'),peak_mib=r.get('peak_bytes',0)/2**20,anon_mib=r.get('sampled_max',{}).get('anon',0)/2**20,file_mib=r.get('sampled_max',{}).get('file',0)/2**20,rss_sum_mib=r.get('sampled_max',{}).get('rss_sum',0)/2**20,events=r.get('events',{}),last_phase=events[-1]['phase'] if events else None,phases=events,error=error)
        if campaign=='history-node':row['campaign']='history';row['source_campaign']='history-node'
        result=r.get('result',{});native=result.get('data',{})
        row['native_timing_ms']=native.get('timing_ms');row['step_bytes']=result.get('step_bytes',native.get('step_bytes'));row['deliverable']=result.get('deliverable','head metadata lookup' if row['campaign']=='history' else 'exact assembly/part STEP')
        if status=='not_run_after_boundary':
            for metric in ['peak_mib','anon_mib','file_mib','rss_sum_mib']:row[metric]=None
        row['step_mib']=row['step_bytes']/2**20 if row['step_bytes'] is not None else None
        rows.append(row)
summaries=[]
for campaign in ['main','history','sharded']:
    keys=sorted({(r['backend'],r['family'],r.get('variant',''),r['n']) for r in rows if r['campaign']==campaign})
    for b,f,v,n in keys:
        selected=[r for r in rows if (r['campaign'],r['backend'],r['family'],r.get('variant',''),r['n'])==(campaign,b,f,v,n)]
        stats={'campaign':campaign,'backend':b,'family':f,'variant':v,'n':n,'runs':len(selected),'statuses':[r['status'] for r in selected]}
        for metric in ['peak_mib','anon_mib','rss_sum_mib','file_mib','wall_s','step_mib']:
            vals=[r[metric] for r in selected if r[metric] is not None]
            if not vals:continue
            stats[metric]={'median':statistics.median(vals),'min':min(vals),'max':max(vals)}
        summaries.append(stats)
evidence={'date':'2026-11-11','baseline_commit':'7b056ed3c6ed39acad7bfe082cd311d957916fb9','published_version':'0.1.0-alpha.8','environments':env,'accounting':'MiB = 2^20 bytes; cgroup kernel total peak, sampled anonymous/file/RSS. File/anon separate maxima need not occur together. Three-run bands are observed ranges, not confidence intervals.','scheduled':len(rows),'executed':sum(r['status']!='not_run_after_boundary' for r in rows),'rows':rows,'summaries':summaries}
(a.dest/'memory-study.json').write_text(json.dumps(evidence,indent=2)+'\n')
with (a.dest/'memory-study.csv').open('w') as f:
    cols=['campaign','id','backend','family','n','status','peak_mib','anon_mib','file_mib','wall_s','step_mib','last_phase'];w=csv.DictWriter(f,cols,extrasaction='ignore',lineterminator='\n');w.writeheader();w.writerows(rows)
plt.rcParams.update({'font.size':10,'svg.fonttype':'none','axes.spines.top':False,'axes.spines.right':False,'figure.facecolor':'white'})
def series(ax,campaign,family,metric,by='backend'):
    groups=sorted({s['variant'] if by=='variant' else s['backend'] for s in summaries if s['campaign']==campaign and s['family']==family})
    for g in groups:
        ss=[s for s in summaries if s['campaign']==campaign and s['family']==family and s[by]==g and all(v=='ok' for v in s['statuses'])]
        ss.sort(key=lambda s:s['n']);xs=[s['n'] for s in ss]
        ax.plot(xs,[s[metric]['median'] for s in ss],'o-',color=colors[g],label=g)
        ax.fill_between(xs,[s[metric]['min'] for s in ss],[s[metric]['max'] for s in ss],color=colors[g],alpha=.15)
    ax.grid(alpha=.2);ax.set_ylabel('MiB' if metric.endswith('mib') else 'Seconds');ax.legend(fontsize=8)
fig,axes=plt.subplots(2,2,figsize=(12,8),layout='constrained')
for ax,f,title in zip(axes.flat[:3],['unique','part','repeated'],['Independent 16-hole parts','Holes in one exact solid','Shared-definition instances']):
    series(ax,'main',f,'peak_mib');ax.set_title(title);ax.set_xlabel('Parts' if f!='part' else 'Holes')
ax=axes.flat[3];series(ax,'history','history','peak_mib','variant');ax.set_xscale('log');ax.set_title('Head lookup: synthetic history metadata');ax.set_xlabel('Entries')
failed=[s for s in summaries if s['campaign']=='history' and not all(v=='ok' for v in s['statuses'])]
for s in failed:ax.plot(s['n'],s['peak_mib']['median'],'x',color=colors[s['variant']],markersize=10);ax.annotate('/'.join(sorted(set(s['statuses']))),(s['n'],s['peak_mib']['median']),fontsize=8,xytext=(-90,-20),textcoords='offset points')
fig.suptitle('Colocad / CadQuery / build123d: measured memory, not universal rankings\n1 vCPU, native Linux arm64; fresh processes; medians and observed ranges',fontsize=13)
fig.savefig(a.dest/'memory-scaling.svg');fig.savefig(a.dest/'memory-scaling.png',dpi=150);plt.close(fig)
fig,axes=plt.subplots(2,2,figsize=(12,8),layout='constrained')
series(axes[0,0],'sharded','unique','anon_mib');axes[0,0].set_title('8-part chunks: anonymous working set');axes[0,0].set_xlabel('Total independently modeled parts')
series(axes[0,1],'sharded','unique','wall_s');axes[0,1].set_title('Chunked project: latency cost');axes[0,1].set_xlabel('Total parts (different output contract)')
selected=[r for r in rows if r['campaign']=='boundary'];ax=axes[1,0]
for i,r in enumerate(selected):
    ax.bar(i,r['peak_mib'],color=colors[r['backend']],alpha=.8);ax.text(i,r['peak_mib']+20,r['status'],ha='center',fontsize=8)
ax.set_xticks(range(len(selected)),[r['backend']+'\ncap '+str(r['memory_mib']) for r in selected],fontsize=8);ax.axhline(2048,color='black',ls='--',lw=.8);ax.set_ylabel('MiB');ax.set_title('Same 2,048-part task: 4 GiB / 2 GiB')
selected=[r for r in rows if r['campaign']=='compositional'];ax=axes[1,1]
for i,r in enumerate(selected):
    ax.bar(i,r['peak_mib'],color=colors[r['backend']],alpha=.8);ax.text(i,r['peak_mib']+20,r['status'],ha='center',fontsize=7)
ax.set_xticks(range(len(selected)),[r['backend']+'\n'+r['family']+' '+str(r['n']) for r in selected],fontsize=7);ax.axhline(2048,color='black',ls='--',lw=.8);ax.set_ylabel('MiB');ax.set_title('Control: validate definitions before placement')
fig.suptitle('Persistence and validation controls — prototypes, not released alpha.9 functionality',fontsize=13)
fig.savefig(a.dest/'memory-controls.svg');plt.close(fig)
fig,axes=plt.subplots(1,2,figsize=(12,4),layout='constrained')
series(axes[0],'sharded','unique','step_mib');axes[0].set_title('Chunked STEP output remains linear');axes[0].set_xlabel('Parts');axes[0].set_ylabel('STEP output MiB (not RAM)')
selected=[r for r in rows if r['campaign']=='native-control'];ax=axes[1]
for i,r in enumerate(selected):ax.bar(i,r['peak_mib'],color=colors['colocad']);ax.text(i,r['peak_mib']+3,f"{r['peak_mib']:.1f}",ha='center')
ax.set_xticks(range(len(selected)),[r['variant'] for r in selected]);ax.set_ylabel('Peak MiB');ax.set_title('64 unique parts: same native entry point')
fig.suptitle('Persistence costs: disk tradeoff and native artifact ablation',fontsize=13);fig.savefig(a.dest/'memory-storage.svg');plt.close(fig)
for svg in a.dest.glob('memory-*.svg'):
    svg.write_text('\n'.join(line.rstrip() for line in svg.read_text().splitlines())+'\n')
svgs=[(a.dest/f).read_text().split('<svg',1)[1] for f in ['memory-scaling.svg','memory-controls.svg','memory-storage.svg']]
table=''.join('<tr>'+''.join('<td>'+html.escape(str(r.get(k,'')))+'</td>' for k in ['campaign','backend','family','variant','n','status'])+''.join('<td>'+('—' if r.get(k) is None else f'{r[k]:.2f}')+'</td>' for k in ['peak_mib','anon_mib','wall_s'])+'</tr>' for r in rows)
page='''<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>Colocad memory study · 2026-11-11</title><link rel="icon" href="data:,"><style>body{font:16px system-ui;max-width:1200px;margin:auto;padding:24px;color:#172433;background:#f5f7fa}section{overflow:auto;background:white;padding:24px;margin:20px 0;border-radius:14px}svg{width:100%;height:auto}table{border-collapse:collapse;width:100%;font-size:13px}td,th{padding:7px;text-align:left;border-bottom:1px solid #ddd}input{font:inherit;padding:8px}code{background:#eef1f5}small{color:#526579}</style><h1>Memory limits are a pipeline problem</h1><p>Colocad alpha.8 · CadQuery 2.8.0 · build123d 0.13.0 — native Linux arm64, 1 vCPU, no swap.</p><section><h2>Read the comparisons correctly</h2><p>Common runs deliver exact STEP and validate analytic geometry. Policy refusal is not a low-memory success. A cgroup peak includes charged file pages; anonymous memory is reported separately. Three-run ranges are descriptive, not confidence intervals.</p><p>The indexed history test uses synthetic metadata, not hundreds of thousands of actual CAD edits. Chunking preserves geometry in an indexed project but does not produce one merged assembly STEP. No claim of O(log N) total resident exact geometry is made.</p></section>'''
page+='<section><h2>What the experiments establish</h2><p>The same 2,048 distinct-part task completes at a 4 GiB cap with 2,289 MiB (CadQuery) / 2,312 MiB (build123d) peak, but both exceed a 2 GiB cap during STEP export. Colocad is efficient for shared instances; it uses more memory on the tested 64 distinct parts and 1,024-hole solid.</p><p>In the history plots, <b>sdk</b> is the current Node JSON-ledger lookup, <b>indexed-node</b> is the same Node runtime and SDK import with SQLite, and <b>indexed</b> is an auxiliary Python/SQLite control. Compare sdk with indexed-node to isolate the indexing change. A runtime_oom is V8 heap exhaustion, distinct from a cgroup oom.</p><p>194 executed observations; 6 larger cases unrun after a boundary. Agent, harness, inference and browser are outside these measurements. Wall times are descriptive on a shared host. Alpha.9 is planned; the indexed and chunked prototypes are not released SDK functionality.</p></section>'
page+=''.join('<section><svg'+s+'</section>' for s in svgs)
page+='<section><h2>Every scheduled observation</h2><input id="filter" aria-label="Filter observations" placeholder="Filter backend, case, outcome"><table id="runs"><thead><tr>'+''.join('<th>'+x+'</th>' for x in ['Campaign','Backend','Family','Variant','N','Outcome','Peak MiB','Anon MiB','Seconds'])+'</tr></thead><tbody>'+table+'</tbody></table></section><script>document.getElementById("filter").addEventListener("input",e=>{for(const r of document.querySelectorAll("tbody tr"))r.hidden=!r.textContent.toLowerCase().includes(e.target.value.toLowerCase())})</script><small>All plots and data are embedded. No network, CDN, fonts or external JavaScript required.</small></html>'
(a.dest/'memory-study.html').write_text(page)
print(json.dumps({'scheduled':len(rows),'executed':evidence['executed'],'output':str(a.dest)}))
