"""Proof: same unique parts, bounded batch, process exit, durable indexed chunks.
Deliverable is a chunked project, NOT one merged assembly STEP. No geometry omission.
"""
import argparse,json,os,subprocess,sys,time
from pathlib import Path
from catalog import Catalog,digest
p=argparse.ArgumentParser();p.add_argument('backend');p.add_argument('n',type=int);p.add_argument('--batch',type=int,default=32);a=p.parse_args()
out=Path('/out');(out/'chunks').mkdir();catalog=Catalog(out/'project.sqlite');t=time.perf_counter();total_bytes=0;volume=0;count=0
python={'colocad':'python','cadquery':'/opt/cq/bin/python','build123d':'/opt/bd/bin/python'}[a.backend]
for start in range(0,a.n,a.batch):
    n=min(a.batch,a.n-start);pending=out/('pending-'+str(start));pending.mkdir()
    with (pending/'backend.stdout').open('w') as stdout,(pending/'backend.stderr').open('w') as stderr:
        r=subprocess.run([python,'/bench/workload.py',a.backend,'unique',str(n),'--start',str(start),'--out',str(pending)],stdout=stdout,stderr=stderr)
    if r.returncode:raise RuntimeError('chunk failed at '+str(start))
    receipt=json.loads((pending/'result.json').read_text());assert receipt['ok']
    if a.backend=='colocad':
        step=Path(receipt['data']['artifact_directory'])/'model.step';v=receipt['data']['geometry']['volume_mm3']
    else:step=pending/'model.step';v=receipt['volume_mm3']
    with step.open('rb') as f:os.fsync(f.fileno())
    sha=digest(step);size=step.stat().st_size;rel=step.relative_to(pending)
    final=out/'chunks'/str(start);pending.rename(final)
    directory=os.open(final.parent,os.O_DIRECTORY);os.fsync(directory);os.close(directory)
    record={'start':start,'count':n,'backend':a.backend,'family':'unique','path':str((final/rel).relative_to(out)),'sha256':sha,'step_bytes':size,'volume_mm3':v,'recipe':{'holes_per_part':16,'thickness_mm':6,'index_start':start,'count':n}}
    catalog.append_chunk(start,n,record);total_bytes+=size;volume+=v;count+=1
    # Advisory release of this project's own completed files; no global drop_caches.
    for f in final.rglob('*'):
        if f.is_file():
            try:
                with f.open('rb') as h:os.fsync(h.fileno());os.posix_fadvise(h.fileno(),0,0,os.POSIX_FADV_DONTNEED)
            except OSError:pass
    with (out/'events.jsonl').open('a') as f:f.write(json.dumps({'phase':'chunk_committed','elapsed_s':time.perf_counter()-t,'completed_parts':start+n})+'\n')
for index in [0,a.n//2,a.n-1]:
    start,n,record=catalog.locate(index);assert start<=index<start+n;assert digest(out/record['path'])==record['sha256']
catalog.close()
(out/'result.json').write_text(json.dumps({'ok':True,'parts':a.n,'chunks':count,'batch':a.batch,'volume_mm3':volume,'step_bytes':total_bytes,'deliverable':'indexed sharded project; not a single assembly STEP','catalog':'project.sqlite'})+'\n')
