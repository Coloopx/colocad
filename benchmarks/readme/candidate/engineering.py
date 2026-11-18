"""Collect the fresh retrofit, angled flange and geometry-history checks into the README dataset."""
import hashlib,json,sys
from pathlib import Path
run=Path(sys.argv[1]);dest=Path(sys.argv[2]);field=run/'field'
def read(p):return json.loads(p.read_text())
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
sessions={};engineering={}
backends = ['colocad','cadquery','build123d']
if (field/'alpha10').is_dir():
    backends.append('alpha10')
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'stable'))
from revision_stats import checked_session, summarize_rounds

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
        evidence['sessions'][backend]=summarize_rounds(rounds)
    evidence['method']['scope']=('One separate image-startup session per backend is recorded under initial_sessions. '
        'The scored runs use three fresh sessions per backend, warmed runtime files and rotated backend order. '
        'Charts use checkpoint medians across scored sessions. Late growth is the median paired difference. '
        'Current is sampled after a commit at 0, 1 and every fourth edit. Peak is the kernel high-water mark.')
(dest/'engineering.json').write_text(json.dumps(evidence,indent=2)+'\n')
