"""Archive only the accounting evidence; do not recover the 4.63 GiB geometry archive in full."""
import argparse,datetime,hashlib,json,sqlite3,statistics
from pathlib import Path
here=Path(__file__).resolve().parent;repo=here.parents[1]
p=argparse.ArgumentParser()
for name in ['archive_extract','field','verified','live','edit_dir','verified_edits','probe','output']:p.add_argument(name,type=Path)
a=p.parse_args();read=lambda path:json.loads(path.read_text());MiB=2**20
iso=lambda t:datetime.datetime.fromisoformat(t.replace('Z','+00:00')).timestamp()
def sha(f):return hashlib.sha256(f.read_bytes()).hexdigest()
def historic(tag):
    path=a.archive_extract/f'session-{tag}';m=read(path/'measurement.json')
    assert m['status']=='ok' and m['result']['verifiedInOrder']==10001
    indexes=list((a.archive_extract/f'project-{tag}').rglob('*.sqlite'))
    if indexes:
        index,=indexes
        con=sqlite3.connect(index.resolve().as_uri()+'?immutable=1',uri=True)
        creation=json.loads(con.execute('SELECT header FROM metadata').fetchone()[0])['createdAt'];con.close()
        db_digest=sha(index)
    else:
        anchor=read(path/'anchor.json');creation=anchor['createdAt'];db_digest=anchor['db_sha256']
    created=iso(creation)
    bounds=[(int(i),iso(t)) for i,t in (line.split() for line in (path/'stdout.log').read_text().splitlines())]
    assert [n for n,_ in bounds]==list(range(1000,10001,1000))
    # session creation and last progress message both occur inside the measured child.
    # The unknown delay from monitor startup to session creation is in [0,delta].
    delta=m['wall_s']-(bounds[-1][1]-created);assert 0<=delta<2
    trace=[json.loads(s) for s in (path/'memory.jsonl').read_text().splitlines()]
    rows=[]
    for n,t in bounds:
        lo=t-created;hi=lo+delta
        # End before earliest possible completion; excludes the final history audit.
        recent=[s for s in trace if lo-5<=s['t']<=lo-1]
        assert len(recent)>30
        rows.append(dict(operations=n,entries=n+1,samples=len(recent),current_window_mib={
            k:statistics.median(s[k] for s in recent)/MiB for k in ['current','anon','file','kernel']},
            sampled_high_water_interval_mib=[max(s['current'] for s in trace if s['t']<=end)/MiB for end in (lo,hi)]))
    return dict(kernel_peak_mib=m['peak_bytes']/MiB,clock_offset_upper_s=delta,checkpoints=rows,
        sha256={f:sha(path/f) for f in ('measurement.json','memory.jsonl','stdout.log')},db_sha256=db_digest)
current=repo/'docs/benchmarks/alpha10'
controls=read(current/'controls.json')
heads=[dict(entries=n,peak_mib=statistics.median(r['peak_bytes']/MiB for r in controls['indexed_session'] if r['id'].startswith(f'head-{n}-'))) for n in (10000,100000,500000)]
summary=read(current/'observations.json')['summary']
geom={r['case']:r['backends']['colocad']['peak_mib']['median'] for r in summary if r['suite']=='main'}
verified=read(a.verified);field={}
for b,v in verified.items():
    m=read(a.field/b/'measurement.json')
    assert m['status']=='ok' and not m['events']['oom_kill'] and m['limit_bytes']=='536870912'
    field[b]={**v,'wall_s':m['wall_s'],'peak_mib':m['peak_bytes']/MiB,'memory_events':m['events'],'producer':m['result']}
live=read(a.live/'result.json');assert live['ok'] and len(set(r['revision'] for r in live['checkpoints']))==1
assert live['historyQueries']==4096 and live['checkpoints'][-1]['queries']==4096
live['session_disk_bytes']=sum(f.stat().st_size for f in (a.live/'session').rglob('*') if f.is_file())
live['session_sqlite_bytes']=next((a.live/'session').glob('*.sqlite')).stat().st_size
live['packs_bytes']=sum(f.stat().st_size for f in (a.live/'session/inspections').glob('receipts-*.gz'))
edit=read(a.edit_dir/'result.json');assert edit['historyEdits']==64
edit['readbacks']=read(a.verified_edits);assert len(edit['readbacks'])==64
edit['session_disk_bytes']=sum(f.stat().st_size for f in (a.edit_dir/'session').rglob('*') if f.is_file())
edit['session_sqlite_bytes']=next((a.edit_dir/'session').glob('*.sqlite')).stat().st_size
probe=read(a.probe);assert probe['hashes_reopened']==68
data=dict(schema='colocad.field-history/1',image=read(current/'acceptance.json')['campaign']['image_id'],old_writes={k:historic(v) for k,v in [('alpha.9','before'),('alpha.10','after')]},head_reads=heads,geometry_median_mib=geom,field=field,
    sources={p.name:sha(p) for p in (a.field/'inputs').glob('*.step')},fixed_geometry=live,rebuild=edit,cache_diagnostic=probe,
    notes=['Mixed write: per-series one archived session; checkpoint component medians in a 4 s window ending 1 to 2.1 s before the corresponding commit marker. Independent component statistics need not sum exactly. High-water interval is sampled, not memory.peak.',
    'Head read: synthetic metadata, three fresh runs/size. Geometry: three fresh runs/size. Fixed geometry: one fresh session with 4 accepted product revisions then 4096 identical durable exact queries; kernel stats at the SDK boundary; excludes final paged audit from checkpoints. No 20 ms trace to disk in this run.',
    'Field backends share geometric acceptance. Python exporters do not implement the Colocad revision ledger, preview or durable queries. Timings are single workflow receipts, not a speed ranking or a comparison of matched persistence implementations.'])
a.output.write_text(json.dumps(data,indent=2)+'\n')
