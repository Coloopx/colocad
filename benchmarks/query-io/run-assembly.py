"""Measure one installed package on assemblies of 16 and 64 unique drilled plates."""
import argparse
import json
import subprocess
import uuid
from pathlib import Path

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('package',type=Path)
p.add_argument('output',type=Path)
p.add_argument('--repeats',type=int,default=3)
p.add_argument('--counts',type=int,nargs='+',choices=[16,64],default=[16,64])
p.add_argument('--limit-mib',type=int,default=512)
a=p.parse_args()
repo=Path(__file__).resolve().parents[2]
image='colocad-memory:20261111'
cases=[(n,rep+1) for n in a.counts for rep in range(a.repeats if n==64 else 1)]
for count,rep in cases:
    out=(a.output / f'{count}-{rep}').resolve()
    out.mkdir(parents=True,exist_ok=False)
    volume='colocad-assembly-'+uuid.uuid4().hex
    subprocess.run(['docker','volume','create',volume],stdout=subprocess.DEVNULL,check=True)
    docker=['docker','run','--rm','--network','none','--cpus','1','--memory',f'{a.limit_mib}m',
            '--memory-swap',f'{a.limit_mib}m','-v',volume+':/out','-v',f'{a.package.resolve()}:/pkg:ro',
            '-v',f'{repo}/benchmarks/memory:/bench:ro']
    try:
        subprocess.run([*docker,image,'python','/bench/monitor.py','node','/bench/workload.mjs',
                        'unique',str(count),'0','/out'],check=True)
        copy='cp /out/measurement.json /out/memory.jsonl /out/result.json /out/stderr.log /keep/'
        subprocess.run([*docker,'-v',f'{out}:/keep',image,'sh','-ec',copy],check=True)
        if json.loads((out/'measurement.json').read_text())['status']!='ok':
            raise RuntimeError(f'build failed: {out}')
        collect='sha256sum /out/job-*/revision/model.step > /keep/step.sha256'
        if rep==1 and count==64:
            collect+=' && cp /out/job-*/revision/model.step /keep/model.step'
        subprocess.run([*docker,'-v',f'{out}:/keep',image,'sh','-ec',collect],check=True)
    finally:
        subprocess.run(['docker','volume','rm',volume],stdout=subprocess.DEVNULL,check=True)
