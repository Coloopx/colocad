"""Collect the fresh retrofit, angled flange and geometry-history checks into the README dataset."""
import hashlib,json,sys
from pathlib import Path
from statistics import median
run=Path(sys.argv[1]);dest=Path(sys.argv[2]);field=run/'field'
def read(p):return json.loads(p.read_text())
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
sessions={};engineering={}
backends = ['colocad','cadquery','build123d']
if (field/'alpha10').is_dir():
    backends.append('alpha10')
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
    if backend in ('colocad','alpha10'):trial['archive_audit']=read(folder/(backend+'-archive.json'))
    return trial

for backend in backends:
    doc=read(field/backend/'result.json')
    checks=read(field/(backend+'-checked.json'))
    sessions[backend]=checked_session(field,backend)
    if backend!='alpha10':
        measured=read(run/'flange'/backend/'occt-readback.json')
        assert len(measured['verified'])==3 and len(measured['expected_rejections'])==2
        engineering[backend]={'adapter':{'accepted':checks['setup'],'rejected':checks['rejections']},
                              'flange':{'accepted':measured['verified'],'rejected':measured['expected_rejections']}}
        if backend=='colocad':engineering[backend]['saddle_recognition']=doc['diagnostics']
if 'alpha10' in sessions:
    assert [r['sha256'] for r in sessions['alpha10']['edits']]==[r['sha256'] for r in sessions['colocad']['edits']]
evidence={'campaign':read(run/'campaign.json'),'sessions':sessions,'engineering':engineering,
          'input_sha256':{f.name:sha(f) for f in (field/'inputs').glob('*.step')},
          'method':{'storage':'fresh Docker-managed native Linux volumes, no network/swap, 512 MiB and 1 vCPU',
                    'common_deliverables':'Persist and reopen the editable recipe, source STEP, exact assembly STEP, mesh preview, indexed version and receipt; rebuild 256 distinct parameter sets. Each output checked in separate OCCT.',
                    'scope':'One uninterrupted session per implementation, four setup deliveries. Colocad also stores native snapshots, full previews and checkpoints. Python controls use SQLite FULL synchronization, fsync, cache advice and bounded live shapes. Current cgroup sampled after commit; peak read before independent verification.',
                    'flexibility':'Two deterministic engineering workflows; three backends each accepted seven deliveries and rejected three incompatible replacements.'}}
if evidence['campaign']['runtime_kind']=='public npm release':
    evidence['initial_sessions']=sessions
    evidence['sessions']={}
    for backend in backends:
        rounds=[]
        for n in (1,2,3):
            folder=run/'history'/f'round-{n}'
            assert {f.name:sha(f) for f in (folder/'inputs').glob('*.step')}==evidence['input_sha256']
            rounds.append(checked_session(folder,backend))
        series=[]
        for samples in zip(*(trial['checkpoints'] for trial in rounds)):
            count=samples[0]['edits']
            assert all(row['edits']==count for row in samples)
            values={k:median(s[k] for s in samples) for k in samples[0] if k!='edits'}
            series.append({'edits':count,**values,'current_min_mib':min(s['current_mib'] for s in samples),
                           'current_max_mib':max(s['current_mib'] for s in samples)})
        evidence['sessions'][backend]={
            'checkpoints':series,'peak_mib':series[-1]['peak_mib'],
            'last_128_growth_mib':median(trial['last_128_growth_mib'] for trial in rounds),'rounds':rounds}
    evidence['method']['scope']=('One separate image-startup session per backend is recorded under initial_sessions. '
        'The scored runs use three fresh sessions per backend, warmed runtime files and rotated backend order. '
        'Charts use checkpoint medians across scored sessions. Late growth is the median paired difference. '
        'Current is sampled after a commit at 0, 1 and every fourth edit. Peak is the kernel high-water mark.')
(dest/'engineering.json').write_text(json.dumps(evidence,indent=2)+'\n')
