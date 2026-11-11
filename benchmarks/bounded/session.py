"""Alpha.9's unmodified 10k append workload and monitor, with per-version packages."""
import argparse,json,subprocess
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('package',type=Path);p.add_argument('output',type=Path);p.add_argument('project',type=Path)
p.add_argument('--kernel',type=Path);p.add_argument('--catalog',type=Path)
a=p.parse_args();root=Path(__file__).resolve().parents[2];dest=a.output.resolve();dest.mkdir(parents=True,exist_ok=False);data=a.project.resolve();data.mkdir(parents=True,exist_ok=False)
cmd=['docker','run','--rm','--cpus','1','--network','none','--memory','512m','--memory-swap','512m','-e','CASE_TIMEOUT=1800','-v',f'{a.package.resolve()}:/pkg:ro','-v',f'{root}/benchmarks:/pkg/benchmarks:ro','-v',f'{data}:/data','-v',f'{dest}:/out']
if a.kernel:cmd+=['-v',f'{a.kernel.resolve()}:/candidate:ro','-e','AGENT_CAD_BINARY=/candidate']
if a.catalog:cmd+=['-v',f'{a.catalog.resolve()}:/pkg/kernel/native/linux-arm64/colocad-catalog:ro']
subprocess.run(cmd+['colocad-memory:20261111','python','/pkg/benchmarks/memory/monitor.py','node','/pkg/benchmarks/session-catalog/workload.mjs','append','10000','/data/edits'],check=True)
m=json.loads((dest/'measurement.json').read_text());assert m['status']=='ok' and m['result']['verifiedInOrder']==10001
