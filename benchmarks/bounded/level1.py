"""Compare exact-kernel phase timers on the same host; alpha.9 probe changes timers only."""
import argparse,hashlib,json,re,subprocess,tempfile
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('alpha9_probe',type=Path);p.add_argument('candidate',type=Path);p.add_argument('input',type=Path);p.add_argument('out',type=Path);p.add_argument('--repeats',type=int,default=3);a=p.parse_args()
data=a.input.read_bytes();rows=[]
for rep in range(a.repeats):
    for label in (['before','after'] if rep%2==0 else ['after','before']):
        binary=(a.alpha9_probe if label=='before' else a.candidate).resolve()
        with tempfile.TemporaryDirectory() as tmp:
            run=subprocess.run([str(binary),'build','--out',tmp+'/job'],input=data,capture_output=True,check=True)
            d=json.loads(run.stdout)['data'];v=d['timing_ms']
            ms=float(re.search(rb'PHASE level1 ([0-9.]+)',run.stderr)[1])*1000 if label=='before' else v['level1']
            rows.append(dict(label=label,rep=rep,level1_ms=ms,validation_ms=v['validation'],total_ms=v['total'],step_sha256=d['artifact_sha256']['model.step'],input_sha256=hashlib.sha256(data).hexdigest()))
            assert rows[0]['step_sha256']==rows[-1]['step_sha256']
            print(label,rep,round(ms),flush=True);a.out.write_text(json.dumps(rows,indent=2)+'\n')
