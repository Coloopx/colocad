"""Collect acceptance, per-trial traces and exclusive V8 allocation attribution."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
from statistics import mean
from math import sqrt

parser=argparse.ArgumentParser()
parser.add_argument('work',type=Path)
parser.add_argument('dest',type=Path)
args=parser.parse_args()
work=args.work
repo=Path(__file__).resolve().parents[2]
def read(p):return json.loads(p.read_bytes())
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def corr(a,b):
    aa=mean(a);bb=mean(b)
    return sum((x-aa)*(y-bb) for x,y in zip(a,b))/sqrt(sum((x-aa)**2 for x in a)*sum((y-bb)**2 for y in b))

def series(samples):
    keep=('current','peak','anon','file','kernel','slab','slab_reclaimable','slab_unreclaimable',
          'active_file','inactive_file','heapUsed','preGcHeap','file_dirty','file_writeback')
    checkpoints=[dict(edit=row.get('edits',row.get('edit')),**{k:row[k] for k in keep if k in row}) for row in samples]
    values={s['edit']:s for s in checkpoints}
    assert 128 in values and 256 in values
    deltas={k:(values[256][k]-values[128][k])/2**20 for k in keep if k in values[128]}
    output=dict(delta_mib_128_256=deltas,samples_bytes=checkpoints)
    if 'heapUsed' in values[128]:
        late=[s for s in checkpoints if s['edit']>=128]
        def diff(k):return [b[k]-a[k] for a,b in zip(late,late[1:])]
        output['late_stepwise_heap_current_pearson']=corr(diff('heapUsed'),diff('current'))
        output['late_stepwise_heap_anon_pearson']=corr(diff('heapUsed'),diff('anon'))
    return output

official=read(repo/'docs/benchmarks/stable-0.1.1/observations.json')['campaign']
record=dict(package=official['label'],registry_integrity=official['registry_integrity'],image_id=official['image_id'],
            native_sha256=sha(work/'package/kernel/native/linux-arm64/agent-cad'),input_sha256={p.name:sha(p) for p in sorted((work/'raw/trial/history/round-1/inputs').glob('*.step'))})
assert record['native_sha256']==official['linux_arm64']['agent-cad']
record['original']={}
prior=read(repo/'docs/benchmarks/stable-0.1.1/engineering.json')
for backend in ('colocad','cadquery','build123d'):
    runs=[]
    for p,reference in zip(sorted((work/'raw/trial/history').glob('round-*/'+backend+'/result.json')),prior['sessions'][backend]['rounds']):
        run=series(read(p)['checkpoints'])
        assert run['delta_mib_128_256']['current']==reference['last_128_growth_mib']
        runs.append(dict(raw_sha256=sha(p),**run))
    assert len(runs)==3
    record['original'][backend]=runs
record['live']={}
for p in sorted((work/'live').glob('*/result.json')):
    root=p.parent
    checked=read(root/'colocad-checked.json')
    archive=read(root/'archive.json')
    events=read(root/'cgroup-events.json')
    assert events['limit']==512*2**20 and events['swap']==0 and int(events['events']['oom_kill'])==0
    assert len(checked['edits'])==256 and archive['accepted']==260 and len(archive['oldSnapshots'])==6
    record['live'][root.name]=dict(**series(read(p)['checkpoints']),events=events,checked_edits=256,
                                  old_snapshot_count=6,verified_hashes=archive['artifactHashesChecked'])
assert len(record['live'])==4
serial=work/'serial/cpu0-natural'
page_census=read(serial/'cold-pages.json')
a=read(serial/'archive.json')
events=read(serial/'cgroup-events.json')
assert len(read(serial/'colocad-checked.json')['edits'])==256 and a['accepted']==260
assert len(a['oldSnapshots'])==6 and int(events['events']['oom_kill'])==0 and events['swap']==0 and events['limit']==512*2**20
record['serial']=dict(**series(read(serial/'result.json')['checkpoints']),census=page_census,checked_edits=256,
                      verified_hashes=a['artifactHashesChecked'],old_snapshot_count=6,events=events,advice_stat=read(serial/'advice-stat.json'))
record['capture']=read(work/'capture/capture.json')
record['layout']={}
for p in sorted((work/'layout').glob('*-write.json')):
    d=read(p)
    checks=read(p.with_name(p.name.replace('-write','-verify')))
    assert d['limits']==['100000 100000',str(512*2**20),'0'] and 'oom_kill 0' in d['events']
    assert checks['members_sha256_verified']==record['capture']['files'] and d['payload_bytes']==record['capture']['bytes']
    record['layout'][p.stem.removesuffix('-write')]=dict(**series(d['samples']),**checks,wall_s=d['wall_s'])
assert len(record['layout'])==6
record['sync_pairs']={}
for p in sorted((work/'sync').glob('*-write.json')):
    d=read(p);checks=read(p.with_name(p.name.replace('-write','-verify')))
    assert d['payload_bytes']==record['capture']['bytes'] and checks['members_sha256_verified']==record['capture']['files']
    assert d['limits']==['100000 100000',str(512*2**20),'0'] and 'oom_kill 0' in d['events']
    record['sync_pairs'][p.stem.removesuffix('-write')]=dict(wall_s=d['wall_s'],**checks,**series(d['samples']))
assert len(record['sync_pairs'])==6
record['cold_queries']=read(work/'cold-queries.json')
assert [r['edit'] for r in record['cold_queries']]==[192,1,256,64,128,32]
assert all(r['snapshot']['geometry_build_calls']==0 and r['geometry']['validSolids']==2 for r in record['cold_queries'])
profile=read(work/'v8-profile.json')
sites=Counter()
def walk(node,owner):
    frame=node['callFrame']
    url=frame['url']
    if url.startswith('file:///pkg/'):
        owner=(url.removeprefix('file:///pkg/')+':'+str(frame['lineNumber']+1)+' '+(frame['functionName'] or '(anonymous)'))
    sites[owner]+=node['selfSize']
    for child in node.get('children',[]):walk(child,owner)
walk(profile['head'],'runtime/other')
record['allocation_profile']=dict(edits=64,sampling_interval=32768,includes_collected=True,
                                 raw_sha256=sha(work/'v8-profile.json'),sample_count=len(profile['samples']),
                                 estimated_bytes=sum(sites.values()),estimated_bytes_by_nearest_package_frame=dict(sites.most_common()))
args.dest.mkdir(parents=True,exist_ok=True)
(args.dest/'evidence.json').write_text(json.dumps(record,indent=2)+'\n')
