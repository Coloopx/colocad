"""Collect the fresh retrofit, angled flange and geometry-history checks into the README dataset."""
import hashlib,json,sys
from pathlib import Path
run=Path(sys.argv[1]);dest=Path(sys.argv[2]);field=run/'field'
def read(p):return json.loads(p.read_text())
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
sessions={};engineering={}
for backend in ('alpha10','colocad','cadquery','build123d'):
    directory=field/backend;doc=read(directory/'result.json');checks=read(field/(backend+'-checked.json'))
    assert doc['ok'] and len(checks['edits'])==256 and len(checks['setup'])==4 and len(checks['rejections'])==1
    series=[{'edits':s['edits'],**{k+'_mib':s[k]/2**20 for k in ('current','peak','file','anon','kernel')}} for s in doc['checkpoints']]
    by_edit={r['edits']:r for r in series}
    assert series[-1]['edits']==256 and int(checks['events']['events']['oom_kill'])==0
    trial={'checkpoints':series,'last_128_growth_mib':by_edit[256]['current_mib']-by_edit[128]['current_mib'],
           'peak_mib':series[-1]['peak_mib'],'events':checks['events'],'edits':checks['edits'],'preview_hashes':checks['preview_hashes'],
           'preview_bounds':checks['preview_bounds'],'reopened_pitches':[1,32,64,128,192,256]}
    if backend in ('colocad','alpha10'):trial['archive_audit']=read(field/(backend+'-archive.json'))
    sessions[backend]=trial
    if backend!='alpha10':
        measured=read(run/'flange'/backend/'occt-readback.json')
        assert len(measured['verified'])==3 and len(measured['expected_rejections'])==2
        engineering[backend]={'adapter':{'accepted':checks['setup'],'rejected':checks['rejections']},
                              'flange':{'accepted':measured['verified'],'rejected':measured['expected_rejections']}}
        if backend=='colocad':engineering[backend]['saddle_recognition']=doc['diagnostics']
assert [r['sha256'] for r in sessions['alpha10']['edits']]==[r['sha256'] for r in sessions['colocad']['edits']]
evidence={'campaign':read(run/'campaign.json'),'sessions':sessions,'engineering':engineering,
          'input_sha256':{f.name:sha(f) for f in (field/'inputs').glob('*.step')},
          'method':{'storage':'fresh Docker-managed native Linux volumes, no network/swap, 512 MiB and 1 vCPU',
                    'common_deliverables':'Persist and reopen the editable recipe, source STEP, exact assembly STEP, mesh preview, indexed version and receipt; rebuild 256 distinct parameter sets. Each output checked in separate OCCT.',
                    'scope':'One uninterrupted session per implementation, four setup deliveries. Colocad also stores native snapshots, full previews and checkpoints. Python controls use SQLite FULL synchronization, fsync, cache advice and bounded live shapes. Current cgroup sampled after commit; peak read before independent verification.',
                    'flexibility':'Two deterministic engineering workflows; three backends each accepted seven deliveries and rejected three incompatible replacements.'}}
(dest/'engineering.json').write_text(json.dumps(evidence,indent=2)+'\n')
