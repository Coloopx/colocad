"""Check M1 gates and archive the raw cgroup observations from run.py."""
import argparse
import hashlib
import json
import statistics
import subprocess
import tarfile
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument('runs', type=Path)
p.add_argument('output', type=Path)
p.add_argument('--image', default='colocad-memory:20261111')
p.add_argument('--storage', required=True, choices=['docker-volume','bind-mount'])
args = p.parse_args()
root = Path(__file__).resolve().parents[2]
args.output.mkdir(parents=True, exist_ok=True)
observations = {}
archive = args.output / 'observations.tar.gz'
def portable_member(member):
    member.uid = member.gid = member.mtime = 0
    member.uname = member.gname = ''
    return member


with tarfile.open(archive, 'w:gz') as tar:
    for file in sorted(args.runs.glob('*/measurement.json')):
        observation = json.loads(file.read_text())
        assert observation['status'] == 'ok' and observation['events']['oom_kill'] == 0, file.parent
        observations[file.parent.name] = observation
        for name in ('measurement.json','result.json','memory.jsonl','stdout.log','stderr.log'):
            tar.add(file.parent / name, arcname=f'{file.parent.name}/{name}', filter=portable_member)
assert len(observations) == 16
appends = observations['append-10000-0']['result']
assert appends['queries'] == 9900 and appends['edits'] == 100 and appends['verifiedInOrder'] == 10001
lookups = {}
for n in (10000,100000,500000):
    probes = [observations[f'head-{n}-{rep}'] for rep in range(3)]
    lookups[n] = {
        'total_peak_mib_median':statistics.median(p['peak_bytes'] / 2**20 for p in probes),
        'sampled_anon_peak_mib_median':statistics.median(p['sampled_max']['anon'] / 2**20 for p in probes),
    }
delta = lookups[500000]['sampled_anon_peak_mib_median'] - lookups[10000]['sampled_anon_peak_mib_median']
assert delta <= 16
files = ['Cargo.lock','src/session.mjs','src/session-catalog.mjs','crates/session-catalog/src/main.rs',
         'crates/session-catalog/src/migration.rs','benchmarks/session-catalog/workload.mjs',
         'benchmarks/memory/monitor.py','kernel/native/linux-arm64/colocad-catalog','kernel/native/linux-arm64/agent-cad']
image = json.loads(subprocess.check_output(['docker','image','inspect',args.image]))[0]
summary = {
    'image': {k:image[k] for k in ('Id','Architecture','Os')},
    'storage':args.storage, 'cpu':1, 'memory_mib':512, 'swap_mib':0, 'network':False,
    'sources_sha256':{f:hashlib.sha256((root/f).read_bytes()).hexdigest() for f in files},
    'archive_sha256':hashlib.sha256(archive.read_bytes()).hexdigest(),
    'lookups':lookups, 'anon_growth_10k_to_500k_mib':delta,
    'observations':observations,
}
(args.output/'measurements.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({'lookup_medians':lookups,'anon_growth_mib':delta,'append_peak_mib':observations['append-10000-0']['peak_bytes']/2**20}))
