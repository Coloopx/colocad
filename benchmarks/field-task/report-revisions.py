"""Build the public comparison from accepted deliveries and kernel accounting."""
import argparse
import json
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument('root', type=Path, help='baseline/candidate trials and their audits')
p.add_argument('dest', type=Path)
args = p.parse_args()
root = args.root
gate = json.loads(Path(__file__).with_name('revision-gate.json').read_text())
mib = 2**20

def read(path):
    return json.loads(path.read_text())

def summarize(name):
    path = root/name
    result = read(path/'result.json')
    audit = read(root/'audits'/(name.replace('/','-')+'.json'))
    assert result['ok'] and result['historyEdits'] == audit['distinctEditedSteps'] == 256
    assert len(audit['oldSnapshots']) == 6 and audit['accepted'] == 260
    samples = result['checkpoints']
    lookup = {r['edits']:r for r in samples}
    last = lookup[256]
    previous = lookup[128]
    late = (last['current'] - previous['current'])/mib
    stats = ['current','peak','anon','file','kernel']
    events = read(path/'cgroup-events.json') if (path/'cgroup-events.json').exists() else None
    return {
        'checkpoints': [{'edits':r['edits'], **{k+'_mib':r[k]/mib for k in stats}} for r in samples],
        'late_growth_mib':late,
        'peak_mib':last['peak']/mib,
        'passed_memory_thresholds':late <= gate['resource_gate_per_candidate_run']['max_late_current_growth_mib']
            and last['peak']/mib <= gate['resource_gate_per_candidate_run']['max_cgroup_peak_mib'],
        'audit':audit, 'cgroup_events':events,
        'outer_wall_s':read(path/'duration.json')['outer_wall_s'] if (path/'duration.json').exists() else None
    }

oracle = read(root/'readbacks.json')
assert len(oracle) == len({r['sha256'] for r in oracle}) == 256
data = {'gate':gate, 'verified_geometries':oracle, 'trials':{}}
if (root/'environment.json').exists():data['environment']=read(root/'environment.json')
for mount in ('ext4','bind'):
    for version in ('baseline','candidate'):
        name=f'final-{mount}-{version}'
        data['trials'][name]=summarize(name)
        if version == 'candidate':
            assert data['trials'][name]['passed_memory_thresholds'],name

for name in ['final-ext4-confirm', 'runner/history']:
    if not (root/name/'cgroup-events.json').exists():
        continue
    series=summarize(name)
    counters=series['cgroup_events']
    assert counters['limit']==512*mib and counters['swap']==0
    assert int(counters['events']['oom_kill'])==0
    data[name]=series
if (root/'retired-ext4/result.json').exists():
    data['retirement_only_trial'] = summarize('retired-ext4')

args.dest.write_text(json.dumps(data,indent=2)+'\n')
