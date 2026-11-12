"""Measure the imported saddle in each backend. Optional durable revision history for the comparison."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import shutil
import sqlite3

parser = argparse.ArgumentParser()
parser.add_argument('backend', choices=('cadquery', 'build123d'))
parser.add_argument('source_dir', type=Path)
parser.add_argument('out', type=Path)
parser.add_argument('--edits', type=int, choices=(0, 256), default=0)
args = parser.parse_args()
out = args.out
if args.backend == 'cadquery':
    import cadquery as cq
    def load(path): return cq.importers.importStep(str(path)).val()
    def circles(shape, radius):
        return [(e.arcCenter().toTuple(), e.normal().toTuple()) for e in shape.Edges()
                if e.geomType()=='CIRCLE' and abs(e.radius()-radius)<.001 and e.IsClosed()]
    def box(x,y,z,dx,dy,dz): return cq.Solid.makeBox(dx,dy,dz,(x,y,z))
    def bore(x,y,z,r,h): return cq.Solid.makeCylinder(r,h,(x,y,z),(0,1,0))
    def compound(parts): return cq.Compound.makeCompound(parts)
    def export(shape,path): cq.exporters.export(shape,str(path),tolerance=.15,angularTolerance=.15)
else:
    import build123d as bd
    def load(path): return bd.import_step(str(path))
    def circles(shape, radius):
        return [(tuple(e.arc_center),tuple(e.normal())) for e in shape.edges()
                if e.geom_type==bd.GeomType.CIRCLE and abs(e.radius-radius)<.001 and e.is_closed]
    def box(x,y,z,dx,dy,dz): return bd.Solid.make_box(dx,dy,dz).translate((x,y,z))
    def bore(x,y,z,r,h): return bd.Solid.make_cylinder(r,h,bd.Plane(origin=(x,y,z),z_dir=(0,1,0)))
    def compound(parts): return bd.Compound(children=parts)
    def export(shape,path):
        if path.suffix=='.stl': assert bd.export_stl(shape,str(path),tolerance=.15,angular_tolerance=.15)
        else: assert bd.export_step(shape,str(path))

def digest(path):
    with path.open('rb') as source: return hashlib.file_digest(source,'sha256').hexdigest()

def sync(path):
    fd=os.open(path,os.O_RDONLY)
    try: os.fsync(fd)
    finally: os.close(fd)

def drop_clean_pages(path):
    with path.open('rb') as file:
        os.posix_fadvise(file.fileno(),0,0,os.POSIX_FADV_DONTNEED)

def make_adapter(order, mount):
    saddle,rings,(x,y,z)=mount
    g,l,p=order['gauge'],order['length'],order['cameraPitch']
    plate=box(x-22,y,z-l/2,44,g,l)
    cutters=[bore(u,y-1,v,2.75,g+2) for u,v in rings]
    cutters.extend(bore(x+s*p/2,y-1,z,1.7,g+2) for s in (-1,1))
    if order['window']: cutters.append(box(x-4,y-1,z+5,8,g+2,4))
    return compound([saddle,plate.cut(*cutters)])

def measure_mount(path):
    saddle=load(path)
    cc=[p for p,axis in circles(saddle,2.5) if abs(axis[1])>.9999]
    y=max(p[1] for p in cc)
    rings=[(a,c) for a,b,c in cc if abs(b-y)<.01]
    if len(rings)!=4: return None,len(rings)
    xs,zs=zip(*rings)
    assert abs(max(xs)-min(xs)-30)<.01 and abs(max(zs)-min(zs)-30)<.01
    return (saddle,rings,[sum(xs)/4,y,sum(zs)/4]),4

def checkpoint(edits):
    cg=Path('/sys/fs/cgroup')
    stats=dict(line.split() for line in (cg/'memory.stat').read_text().splitlines())
    return dict(edits=edits,current=int((cg/'memory.current').read_text()),
                peak=int((cg/'memory.peak').read_text()),
                **{k:int(stats[k]) for k in ('file','anon','kernel')})

orders=json.loads(Path(__file__).with_name('orders.json').read_text())
receipts=[]; edit_steps=[]; points=[]; seq=0; prior=None
if args.edits:
    source_dir=out/'sources';source_dir.mkdir()
    for path in args.source_dir.glob('*.step'):
        shutil.copyfile(path,source_dir/path.name);sync(source_dir/path.name)
    shutil.copyfile(__file__,source_dir/'python-backends.py')
    shutil.copyfile(Path(__file__).with_name('orders.json'),source_dir/'orders.json')
    sync(source_dir/'python-backends.py');sync(source_dir/'orders.json');sync(source_dir)
    db=sqlite3.connect(out/'history.sqlite')
    db.execute('PRAGMA synchronous=FULL')
    db.execute('CREATE TABLE revisions(seq INTEGER PRIMARY KEY, model TEXT NOT NULL, step TEXT NOT NULL, digest TEXT NOT NULL, mesh TEXT NOT NULL, mesh_digest TEXT NOT NULL)')
    db.commit();sync(out)
else: source_dir=args.source_dir

def save(shape,order,path,seq):
    export(shape,path)
    if not args.edits:return
    mesh=path.with_suffix('.stl');export(shape,mesh)
    sync(path);sync(mesh);sync(path.parent)
    step_hash=digest(path)
    with db:
        db.execute('INSERT INTO revisions VALUES(?,?,?,?,?,?)',
                   (seq,json.dumps(order),path.name,step_hash,mesh.name,digest(mesh)))
    for f in (path,mesh):drop_clean_pages(f)
    return step_hash

for order in orders:
    if order['source'] != prior:
        measured,count=measure_mount(source_dir/(order['source']+'.step'))
        if measured is None:
            receipts.append(dict(id=order['id'],reason='MOUNT_CARDINALITY',observed=count))
            continue
        mount=measured;prior=order['source']
    name=order['id']+'.step';seq+=1
    save(make_adapter(order,mount),order,out/name,seq)
    receipts.append(dict(id=order['id'],placed=mount[2],step=name))
assert [r.get('reason','STEP') for r in receipts]==['STEP','STEP','STEP','MOUNT_CARDINALITY','STEP']

if args.edits:
    points.append(checkpoint(0))
    for i in range(1,args.edits+1):
        order={**orders[-1], 'gauge':5+((i-1)%5)*.5,'cameraPitch':16+((i-1)//5)*.25}
        seq+=1;path=out/f'edit-{i:03}.step'
        sha=save(make_adapter(order,mount),order,path,seq)
        edit_steps.append(dict(i=i,step=path.name,gauge=order['gauge'],cameraPitch=order['cameraPitch'],sha256=sha))
        if i==1 or i%4==0:points.append(checkpoint(i))
    db.close()
    cg=Path('/sys/fs/cgroup')
    (out/'cgroup-events.json').write_text(json.dumps(dict(limit=int((cg/'memory.max').read_text()),swap=int((cg/'memory.swap.max').read_text()),events=dict(line.split() for line in (cg/'memory.events').read_text().splitlines()))))
    with sqlite3.connect(out/'history.sqlite') as audit:
        assert audit.execute('SELECT count(*) FROM revisions').fetchone()[0]==seq
        for i in [1,32,64,128,192,256]:
            stored,step,sha,mesh,mesh_sha=audit.execute('SELECT model,step,digest,mesh,mesh_digest FROM revisions WHERE seq=?',(i+4,)).fetchone()
            assert digest(out/step)==sha and digest(out/mesh)==mesh_sha
            xs=[pt[0] for pt,axis in circles(load(out/step),1.7) if abs(axis[1])>.9999]
            assert len(xs)==4 and math.isclose(max(xs)-min(xs),json.loads(stored)['cameraPitch'],abs_tol=.01)
(out/'result.json').write_text(json.dumps(dict(ok=True,receipts=receipts,historyEdits=args.edits,editSteps=edit_steps,checkpoints=points)))
