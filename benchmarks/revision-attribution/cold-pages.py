"""Observe resident file pages and advise only old immutable members of this disposable run."""
import ctypes
import json
import mmap
import os
from pathlib import Path
import sqlite3
import sys

if sys.argv[1]=='warm':raise SystemExit
root=Path(sys.argv[1]).resolve()
result=Path(sys.argv[2])
db,=root.glob('*.sqlite')
libc=ctypes.CDLL(None,use_errno=True)
libc.mincore.argtypes=[ctypes.c_void_p,ctypes.c_size_t,ctypes.POINTER(ctypes.c_ubyte)]
def resident(file):
    size=os.fstat(file.fileno()).st_size
    if not size:return 0
    pages=(size+mmap.PAGESIZE-1)//mmap.PAGESIZE
    vec=(ctypes.c_ubyte*pages)()
    with mmap.mmap(file.fileno(),0,access=mmap.ACCESS_COPY) as mapping:
        addr=ctypes.addressof(ctypes.c_char.from_buffer(mapping))
        if libc.mincore(addr,size,vec)!=0:raise OSError(ctypes.get_errno(),'mincore')
    return sum(p&1 for p in vec)*mmap.PAGESIZE

def census(files,advise=False):
    total=dict(files=0,resident_before=0,resident_after=0)
    for path in files:
        if path.is_symlink() or not path.is_file() or root not in path.resolve().parents:
            raise ValueError('Unexpected member path in the experiment')
        with path.open('rb',buffering=0) as f:
            total['files']+=1
            total['resident_before']+=resident(f)
            if advise:os.posix_fadvise(f.fileno(),0,0,os.POSIX_FADV_DONTNEED)
            total['resident_after']+=resident(f)
    return total
connection=sqlite3.connect(db.as_uri()+'?immutable=1',uri=True)
seq,=connection.execute('SELECT max(seq) FROM steps').fetchone()
assert seq==260
def members(head):
    for (raw,) in connection.execute('SELECT entry FROM steps WHERE seq '+('= ?' if head else '< ?')+' ORDER BY seq',(seq,)):
        entry=json.loads(raw)
        assert entry['status']=='succeeded' and entry['operation'] in ('build','edit')
        yield root/entry['inputFile']
        yield root/entry['receiptFile']
        yield from (root/entry['artifactDirectory']).iterdir()
out=dict(head=census(members(True)),catalog=census([db]),old=census(members(False),True))
connection.close()
result.write_text(json.dumps(out,indent=2)+'\n')
