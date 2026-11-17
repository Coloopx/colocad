"""Linux-only persistence replay. Both layouts store and sync identical members."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import time

parser=argparse.ArgumentParser()
parser.add_argument('operation',choices=['write','verify'])
parser.add_argument('layout',choices=['loose','segments','segments-commit'])
parser.add_argument('input',type=Path)
parser.add_argument('store',type=Path)
parser.add_argument('result',type=Path)
args=parser.parse_args()
manifest=args.input/'members.jsonl'
store=args.store
index=store/'index.jsonl'
cg=Path('/sys/fs/cgroup')

def syncdir(directory):
    fd=os.open(directory,os.O_RDONLY|os.O_DIRECTORY)
    try:os.fsync(fd)
    finally:os.close(fd)

def discard(file):
    os.posix_fadvise(file.fileno(),0,0,os.POSIX_FADV_DONTNEED)

def sample(edit):
    stat={k:int(v) for k,v in (line.split() for line in (cg/'memory.stat').read_text().splitlines())}
    return dict(edit=edit,current=int((cg/'memory.current').read_text()),
                peak=int((cg/'memory.peak').read_text()),
                **{k:stat[k] for k in ('anon','file','kernel','slab','slab_reclaimable','slab_unreclaimable')})

def write():
    checkpoints=[]
    segment=None
    count=0
    total=0
    start=time.monotonic()
    with manifest.open() as rows,(args.input/'payload').open('rb',buffering=0) as src,index.open('xb',buffering=0) as ledger:
        for edit,line in enumerate(rows,1):
            entries=[]
            dirs=set()
            if args.layout!='loose' and (edit-1)%16==0:
                if segment:segment.close()
                segment_name=f'segment-{(edit-1)//16:04d}.bin'
                segment=(store/segment_name).open('xb',buffering=0)
            for name,size,sha in json.loads(line):
                relative=Path(name)
                assert not relative.is_absolute() and '..' not in relative.parts
                data=src.read(size)
                assert len(data)==size
                if segment:
                    location=segment_name
                    offset=segment.tell()
                    assert segment.write(data)==size
                    if args.layout=='segments':os.fsync(segment.fileno())
                else:
                    location=name
                    offset=0
                    dest=store/relative
                    dest.parent.mkdir(exist_ok=True,parents=True)
                    with dest.open('xb',buffering=0) as member:
                        assert member.write(data)==size
                        os.fsync(member.fileno())
                        discard(member)
                    dirs.update(dest.relative_to(store).parents)
                count+=1
                total+=size
                entries.append([name,location,offset,size,sha])
            if segment:
                if args.layout=='segments-commit':os.fsync(segment.fileno())
                discard(segment)
            for directory in sorted(dirs,key=lambda p:-len(p.parts)):
                if directory!=Path('.'):syncdir(store/directory)
            syncdir(store)
            ledger.write(json.dumps(entries,separators=(',',':')).encode()+b'\n')
            os.fsync(ledger.fileno())
            discard(ledger)
            # Drop completed, page-aligned source bytes in both arms.
            src_end=src.tell()//4096*4096
            if src_end:os.posix_fadvise(src.fileno(),0,src_end,os.POSIX_FADV_DONTNEED)
            if edit==1 or edit%4==0:checkpoints.append(sample(edit))
        assert not src.read(1) and edit==256
    if segment:segment.close()
    return dict(layout=args.layout,edits=edit,members=count,payload_bytes=total,wall_s=time.monotonic()-start,
                samples=checkpoints,events=(cg/'memory.events').read_text(),
                limits=[(cg/k).read_text().strip() for k in ('cpu.max','memory.max','memory.swap.max')])

def verify():
    count=0
    with index.open() as ledger,manifest.open() as source:
        for revisions,line in enumerate(source,1):
            entries=json.loads(ledger.readline())
            assert [[name,size,sha] for name,_,__,size,sha in entries]==json.loads(line)
            for name,location,offset,size,sha in entries:
                with (store/location).open('rb') as f:
                    f.seek(offset)
                    assert hashlib.sha256(f.read(size)).hexdigest()==sha,name
                count+=1
        assert not ledger.readline()
    return dict(revisions=revisions,members_sha256_verified=count,files=sum(p.is_file() for p in store.rglob('*')),
                directories=sum(p.is_dir() for p in store.rglob('*')),index_bytes=index.stat().st_size)
args.result.write_text(json.dumps(write() if args.operation=='write' else verify(),indent=2)+'\n')
