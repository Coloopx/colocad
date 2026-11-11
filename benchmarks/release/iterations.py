"""Join the full refresh and the follow-up assembly experiment with explicit revision IDs."""
import argparse,hashlib,json,statistics
from pathlib import Path
p=argparse.ArgumentParser()
for name in ('initial','final','paired_history','diagnosis','dest'):p.add_argument(name,type=Path)
a=p.parse_args()
read=lambda p:json.loads(p.read_text())
rows=[]
for file in sorted((a.final/'targeted').glob('*/comparison.json')):
    r=read(file);m=read(file.parent/'measurement.json')
    assert m['status']=='ok' and m['limit_bytes']=='536870912' and m['swap_max']=='0'
    r.pop('path');r['measurement_sha256']=hashlib.sha256((file.parent/'measurement.json').read_bytes()).hexdigest()
    r['solves']=m['result']['data'].get('validation_property_solves')
    r['stock_volume_oracles']=m['result']['data'].get('terminal_stock_volume_checks')
    rows.append(r)
expected={(v,n,i) for v in ['before','after'] for n in [1,16,64] for i in range(3)}
assert {(r['variant'],r['n'],r['rep']) for r in rows}==expected and len(rows)==18
accepted=read(a.final/'main-verified.json')
for n in (1,16,64):
    group=[r for r in rows if r['n']==n]
    assert all(len({json.dumps(r[key],sort_keys=True) for r in group})==1 for key in ('input_sha256','sha256','geometry','validation'))
    assert any(r['id']==f'colocad-unique-{n}-r0' and r['sha256']==group[0]['sha256'] and r['passed'] for r in accepted)
for row in rows:
    if row['n']==64 and row['variant']=='after':
        assert row['validation']['measured_instances']==64 and row['stock_volume_oracles']==64
        assert row['solves']['certified_regular_curves']==2*16*64
history=read(a.paired_history)
session=read(a.final/'session-write/measurement.json')
audit=read(a.final/'session-audit.json')
assert session['status']=='ok' and audit['count']==session['result']['verifiedInOrder']==10001
final_session={'peak_mib':session['peak_bytes']/2**20,'wall_s':session['wall_s'],'readback':audit,
               'files':sum(1 for p in (a.final/'session-project/edits').rglob('*') if p.is_file()),
               'sampled_peaks_mib':{k:v/2**20 for k,v in session['sampled_max'].items()}}
initial=read(a.initial/'campaign.json');final=read(a.final/'campaign.json')
diag=read(a.diagnosis/'final-result.json')['data']
assert diag['artifact_sha256']['model.step']==next(r for r in rows if r['n']==64)['sha256']
groups={}
for line in (a.diagnosis/'curves-profile.txt').read_text().splitlines():
    time,entry=line.split(' ms ',1);count,key=entry.split(' ',1);key=key.split(' first ',1)[0]
    item=groups.setdefault(key,{'curves':0,'elapsed_ms':0.})
    item['curves']+=int(count);item['elapsed_ms']+=float(time)
original=read(a.initial/'main/runs.json')['runs']
breakdown={}
for backend in ('colocad','cadquery','build123d'):
    samples=[]
    for r in original:
        if r['backend']!=backend or r['family']!='unique' or r['n']!=64:continue
        if backend=='colocad':
            item={k:s/1000 for k,s in r['result']['data']['timing_ms'].items()}
        else:
            item={};last=0.
            for event in (a.initial/'main'/r['id']/'events.jsonl').read_text().splitlines():
                ev=json.loads(event);item[ev['phase']]=ev['elapsed_s']-last;last=ev['elapsed_s']
        item['wall_s']=r['wall_s'];samples.append(item)
    breakdown[backend]={k:statistics.median(sample[k] for sample in samples) for k in samples[0]}
e={'alpha9_pair':history,'final_session':final_session,'unique64':{'before':initial,'after':final,'trials':rows,'initial_linux_phase_medians_s':breakdown,
'profile_macos_one_local_plate':groups,'profile_scope':'Standalone native curve checks in one saved 16-bore definition with 32 upper/lower trim rings, before the optimization.',
'final_macos_native_64':{'timing_ms':diag['timing_ms'],'curve_checks':diag['validation_property_solves'],'stock_checks':diag['terminal_stock_volume_checks']}}}
(a.dest/'iterations.json').write_text(json.dumps(e,indent=2)+'\n')
