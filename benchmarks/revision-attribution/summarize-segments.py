"""Summarize only complete, independently verified matched KK60 pairs."""
import argparse
import hashlib
import json
import math
import statistics
from pathlib import Path

parser=argparse.ArgumentParser()
parser.add_argument('results',type=Path)
parser.add_argument('output',type=Path)
parser.add_argument('--first-pair',type=int,default=1)
parser.add_argument('--last-pair',type=int,default=10**6)
args=parser.parse_args()
def load(folder,name):
    return json.loads((folder/name).read_bytes())

rows=[]
for folder in sorted(args.results.iterdir()):
    cfg=load(folder,'run-config.json')
    if not args.first_pair<=cfg['pair']<=args.last_pair:continue
    result=load(folder,'result.json')
    archive=load(folder,'archive.json')
    checks=load(folder,'colocad-checked.json')
    assert len(checks['edits'])==256 and archive['accepted']==260
    assert len(archive['oldSnapshots'])==6 and int(checks['events']['events']['oom_kill'])==0
    samples=result['checkpoints']
    old,latest=[next(s for s in samples if s['edits']==n) for n in [128,256]]
    latencies=sorted(row['commitMs'] for row in result['editSteps'])
    keys=['current','slab','file','anon','heapUsed']
    rows.append(dict(**cfg,**load(folder,'layout.json'),
        lateMiB={k:(latest[k]-old[k])/2**20 for k in keys},
        peakMiB=latest['peak']/2**20,
        editP50Ms=statistics.median(latencies),editP95Ms=latencies[math.ceil(.95*len(latencies))-1],
        editTotalSeconds=sum(latencies)/1000,artifactHashes=archive['artifactHashesChecked'],
        oldSnapshotPitches=[s['pitch'] for s in archive['oldSnapshots']],
        occtEditedSteps=len(checks['edits']),oomKills=int(checks['events']['events']['oom_kill']),
        fullCheckSHA256=hashlib.sha256((folder/'colocad-checked.json').read_bytes()).hexdigest(),
        samples=[{k:s[k] for k in ['edits','current','peak','slab','file','anon','heapUsed']} for s in samples]))
modes=['baseline','candidate']
summary={}
for mode in modes:
    r=[x for x in rows if x['mode']==mode]
    summary[mode]={k:statistics.median(x[k] for x in r) for k in ['peakMiB','editP50Ms','editP95Ms','editTotalSeconds','files','directories']}
    summary[mode]['lateMiB']={k:statistics.median(x['lateMiB'][k] for x in r) for k in r[0]['lateMiB']}
pairs=[]
for pair in sorted({r['pair'] for r in rows}):
    old,new=[next(r for r in rows if r['pair']==pair and r['mode']==mode) for mode in modes]
    assert old['cpu']==new['cpu']
    pairs.append(dict(pair=pair,**{k+'ChangePercent':100*(new[k]/old[k]-1) for k in ['peakMiB','editP95Ms','editTotalSeconds']}))
summary['pairedMedianPercent']={k:statistics.median(p[k] for p in pairs) for k in pairs[0] if k!='pair'}
args.output.parent.mkdir(parents=True,exist_ok=True)
args.output.write_text(json.dumps(dict(workload='KK60 256 distinct camera adapter edits with independent OCP readback',
    baseline='@coloopx/colocad 0.1.1',baselineTarSha256='bc35c546414662422949c0108852ee3f0eb028adf0b80b9af7ce2f790e7beaf0',
    platform='linux/arm64',image='colocad-memory:20261111',limitMiB=512,swapMiB=0,
    summary=summary,pairs=pairs,runs=rows),separators=(',',':'))+'\n')
print(json.dumps(summary,indent=2))
