"""Prepare a sequential, byte-exact replay from 256 accepted geometry edits."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import sqlite3

parser=argparse.ArgumentParser()
parser.add_argument('session',type=Path)
parser.add_argument('output',type=Path)
args=parser.parse_args()
root=args.session.resolve()
out=args.output
out.mkdir(parents=True,exist_ok=False)
db,=root.glob('*.sqlite')
con=sqlite3.connect(db.as_uri()+'?immutable=1',uri=True)
rows=con.execute('SELECT seq,entry FROM steps WHERE seq>4 ORDER BY seq')
occurrences=Counter()
size_by_hash={}
count=0
with (out/'payload').open('xb') as stream, (out/'members.jsonl').open('x') as manifest:
    for count,(seq,entry_json) in enumerate(rows,1):
        entry=json.loads(entry_json)
        assert seq==count+4 and entry['status']=='succeeded' and entry['operation']=='edit'
        art=root/entry['artifactDirectory']
        files=[root/entry['inputFile'],root/entry['receiptFile'],*sorted(art.iterdir())]
        receipt=json.loads(files[1].read_bytes())['data']
        want={str(art/name):sha for name,sha in receipt['artifact_sha256'].items()}
        want.update({str(files[0]):entry['inputSha256'],str(files[1]):entry['receiptSha256']})
        assert set(want)<=set(map(str,files))
        members=[]
        for file in files:
            assert file.is_file() and not file.is_symlink() and root in file.resolve().parents
            name=file.relative_to(root).as_posix()
            data=file.read_bytes()
            sha=hashlib.sha256(data).hexdigest()
            if str(file) in want:assert want[str(file)]==sha
            members.append([name,len(data),sha])
            size_by_hash[sha]=len(data)
            occurrences[sha]+=1
            stream.write(data)
        manifest.write(json.dumps(members,separators=(',',':'))+'\n')
assert count==256
con.close()
sizes=list(size_by_hash.values())
evidence=dict(revisions=count,files=occurrences.total() if hasattr(occurrences,'total') else sum(occurrences.values()),
              distinct_hashes=len(sizes),bytes=(out/'payload').stat().st_size,unique_bytes=sum(sizes),
              sqlite_bytes=db.stat().st_size,manifest_sha256=hashlib.sha256((out/'members.jsonl').read_bytes()).hexdigest())
(out/'capture.json').write_text(json.dumps(evidence,indent=2)+'\n')
print(evidence)
