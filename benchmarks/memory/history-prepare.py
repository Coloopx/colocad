"""Synthetic metadata scaling, seeded from an authentic accepted session entry.
Does not claim N CAD builds or N distinct geometric revisions were executed.
"""
import argparse,json,sqlite3
from pathlib import Path
from catalog import Catalog
p=argparse.ArgumentParser();p.add_argument('seed',type=Path);p.add_argument('out',type=Path);p.add_argument('--counts',default='1000,10000,100000,500000');a=p.parse_args()
seed=json.loads(a.seed.read_text());entry=next(s for s in reversed(seed['steps']) if s['status']=='succeeded' and s['operation'] not in ('observe','query'))
for n in map(int,a.counts.split(',')):
    d=a.out/str(n);d.mkdir(parents=True,exist_ok=False);c=Catalog(d/'project.sqlite')
    with (d/'session.json').open('w') as f:
        head={k:v for k,v in seed.items() if k!='steps'};f.write(json.dumps(head)[:-1]+',"steps":[')
        with c.db:
            for i in range(n):
                row={**entry,'id':f's{i+1:07d}','label':'metadata scaling fixture '+str(i+1)};raw=json.dumps(row,separators=(',',':'))
                if i:f.write(',')
                f.write(raw);c.db.execute('INSERT INTO events VALUES(?,?)',(i+1,raw))
        f.write(']}\n')
    c.close();print(n,(d/'session.json').stat().st_size,flush=True)
