"""Build README series only from complete, independently verified 256-edit sessions."""
import json
from statistics import median
from math import ceil

def read(path):
    return json.loads(path.read_bytes())

def checked_session(folder, backend):
    doc=read(folder/backend/'result.json')
    checks=read(folder/(backend+'-checked.json'))
    assert doc['ok'] and len(checks['edits'])==256 and len(checks['setup'])==4 and len(checks['rejections'])==1
    series=[{'edits':s['edits'],**{k+'_mib':s[k]/2**20 for k in ('current','peak','file','anon','kernel')}} for s in doc['checkpoints']]
    by_edit={r['edits']:r for r in series}
    assert list(by_edit)==[0,1,*range(4,257,4)] and int(checks['events']['events']['oom_kill'])==0
    trial={'checkpoints':series,'last_128_growth_mib':by_edit[256]['current_mib']-by_edit[128]['current_mib'],
           'peak_mib':series[-1]['peak_mib'],'events':checks['events'],'edits':checks['edits'],'preview_hashes':checks['preview_hashes'],
           'preview_bounds':checks['preview_bounds'],'reopened_pitches':[1,32,64,128,192,256]}
    if all('slab' in point for point in doc['checkpoints']):
        for sample,point in zip(series,doc['checkpoints']):
            sample['slab_mib']=point['slab']/2**20
        trial['last_128_slab_mib']=by_edit[256]['slab_mib']-by_edit[128]['slab_mib']
    if all('commitMs' in e for e in doc['editSteps']):
        times=sorted(e['commitMs'] for e in doc['editSteps'])
        assert len(times)==256 and times[0]>0
        trial['latency_ms']={'p50':median(times),'p95':times[ceil(.95*len(times))-1],'total_s':sum(times)/1000}
    if backend in ('colocad','alpha10'):trial['archive_audit']=read(folder/(backend+'-archive.json'))
    return trial

def summarize_rounds(rounds):
    assert len(rounds)==3
    series=[]
    for samples in zip(*(trial['checkpoints'] for trial in rounds)):
        count=samples[0]['edits']
        assert all(row['edits']==count for row in samples)
        values={k:median(s[k] for s in samples) for k in samples[0] if k!='edits'}
        series.append({'edits':count,**values,'current_min_mib':min(s['current_mib'] for s in samples),
                       'current_max_mib':max(s['current_mib'] for s in samples)})
    return {
        'checkpoints':series,'peak_mib':series[-1]['peak_mib'],
        'last_128_growth_mib':median(trial['last_128_growth_mib'] for trial in rounds),'rounds':rounds}
