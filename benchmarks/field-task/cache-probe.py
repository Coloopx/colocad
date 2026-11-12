"""Post-run diagnostic confined to a fresh experiment. Never touch live DB, head, shared or unknown files."""
import json,os,sqlite3,subprocess,sys,time,hashlib
from pathlib import Path
subprocess.run(sys.argv[1:],check=True)
out=Path('/out');s=out/'session';result=json.loads((out/'result.json').read_text())
assert result['historyEdits']==64
db,=s.glob('*.sqlite')
with sqlite3.connect(db.resolve().as_uri()+'?immutable=1',uri=True) as con:
    entries=[json.loads(raw) for raw, in con.execute('select entry from steps order by seq')]
assert len(entries)==68 and all(e['status']=='succeeded' and e['operation'] in ('edit','build') for e in entries)
head=entries[-1];protected={head['artifactDirectory'],str(Path(head['inputFile']).parent)}
files=set()
for e in entries[:-1]:
    for name in [e['artifactDirectory'],str(Path(e['inputFile']).parent)]:
        if name in protected:continue
        directory=(s/name).resolve();assert directory.is_relative_to(s.resolve()) and directory!=s.resolve()
        for f in directory.rglob('*'):
            assert not f.is_symlink()
            if f.is_file(): files.add(f)
def cg():
    root=Path('/sys/fs/cgroup')
    st=dict(i.split() for i in (root/'memory.stat').read_text().splitlines())
    return dict(current=int((root/'memory.current').read_text()),peak=int((root/'memory.peak').read_text()),**{k:int(st[k]) for k in ('anon','file','kernel','file_dirty','file_writeback')})
def visit(advise):
    for f in sorted(files):
        fd=os.open(f,os.O_RDONLY|os.O_NOFOLLOW)
        try:
            if advise:os.posix_fadvise(fd,0,0,os.POSIX_FADV_DONTNEED)
        finally:os.close(fd)
visit(False);time.sleep(.5);before=cg();time.sleep(.5);noop=cg()
visit(True);time.sleep(.5);after=cg()
# Reopen every historical STEP by its original digest after the AFTER sample (readback can re-warm pages).
for e in entries:
    receipt=json.loads((s/e['receiptFile']).read_text())
    step=s/e['artifactDirectory']/'model.step'
    assert hashlib.sha256(step.read_bytes()).hexdigest()==receipt['data']['artifact_sha256']['model.step']
(out/'cache-probe.json').write_text(json.dumps(dict(before=before,noop=noop,after=after,
    files=len(files),bytes=sum(p.stat().st_size for p in files),hashes_reopened=len(entries),events=(Path('/sys/fs/cgroup/memory.events').read_text()),
    scope='Fresh private session; producer exited; closed files of older accepted geometry revisions only, no active head, SQLite, global drop or changed artifact bytes. One diagnostic, not a runtime fix.')))
