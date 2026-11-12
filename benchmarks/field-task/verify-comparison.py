"""Independent checks of every warm-session STEP, persisted revision and sampled preview shape."""
import argparse
import hashlib,json,sqlite3
from pathlib import Path
import struct
import numpy as np
from verify import verify

p=argparse.ArgumentParser()
p.add_argument('out',type=Path);p.add_argument('backend',choices=('alpha10','colocad','cadquery','build123d'))
args=p.parse_args();name=args.backend;root=args.out/name
doc=json.loads((root/'result.json').read_text());inputs=args.out/'inputs'
assert doc['ok'] and len(doc['editSteps'])==doc['historyEdits']==256
bad=[v for v in doc['receipts'] if 'reason' in v]
assert bad==[dict(id='missing',reason='MOUNT_CARDINALITY',observed=3)] and not (root/'missing.step').exists()
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
setup=[]
for id,x,z,l,g,pitch,window in [('install',0,0,44,5,16,False),('resize',0,0,54,7,22,False),('service',0,0,54,7,22,True),('replace',7,11,54,7,22,True)]:
    setup.append(verify(root/(id+'.step'),inputs/('relocated.step' if id=='replace' else 'nominal.step'),id,x,z,l,g,pitch,window))
native=name in ('alpha10','colocad')
db_path=next((root/'session').glob('*.sqlite')) if native else root/'history.sqlite'
db=sqlite3.connect(db_path.resolve().as_uri()+'?immutable=1',uri=True)
assert db.execute('PRAGMA integrity_check').fetchone()[0]=='ok'
assert db.execute('SELECT count(*) FROM '+('steps' if native else 'revisions')).fetchone()[0]==260
edits=[]
for i,row in enumerate(doc['editSteps'],1):
    gauge=5+((i-1)%5)*.5; pitch=16+((i-1)//5)*.25
    assert row['gauge']==gauge and row['cameraPitch']==pitch
    if native:
        entry=json.loads(db.execute('SELECT entry FROM steps WHERE seq=?',(i+4,)).fetchone()[0])
        assert entry['id']==row['id'] and entry['status']=='succeeded'
        revision=root/'session'/entry['artifactDirectory']
        step=revision/'model.step';mesh=revision/'model.stl'
        receipt=json.loads((root/'session'/entry['receiptFile']).read_text())
        assert sha(root/'session'/entry['inputFile'])==entry['inputSha256']
        mesh_hash=receipt['data']['artifact_sha256']['model.stl']
    else:
        model,step_path,step_hash,mesh_path,mesh_hash=db.execute('SELECT model,step,digest,mesh,mesh_digest FROM revisions WHERE seq=?',(i+4,)).fetchone()
        model=json.loads(model);assert (model['gauge'],model['cameraPitch'],model['source'])==(gauge,pitch,'relocated')
        assert step_hash==row['sha256']
        step=root/step_path;mesh=root/mesh_path
    assert sha(mesh)==mesh_hash
    if i in (1,32,64,128,192,256):
        buf=mesh.read_bytes();n=struct.unpack_from('<I',buf,80)[0]
        assert n>0 and len(buf)==84+50*n
        tri=np.ndarray((n,3,3),dtype='<f4',buffer=buf,offset=96,strides=(50,12,4))
        assert np.isfinite(tri).all() and np.max(np.abs(tri.max(axis=(0,1))-[29,33+gauge,38]))<.05
    report=verify(step,inputs/'relocated.step',str(i),7,11,54,gauge,pitch,True)
    assert report['sha256']==row['sha256'];edits.append(report)
db.close()
events=json.loads((root/'cgroup-events.json').read_text())
assert int(events['events']['oom_kill'])==0 and int(events['limit'])==512*2**20 and int(events['swap'])==0
assert len({r['sha256'] for r in edits})==256
out=dict(backend=name,setup=setup,rejections=bad,edits=edits,preview_hashes=256,preview_bounds=6,events=events)
(args.out/(name+'-checked.json')).write_text(json.dumps(out,indent=2)+'\n')
print(name,len(edits),'distinct revised assemblies accepted',flush=True)
