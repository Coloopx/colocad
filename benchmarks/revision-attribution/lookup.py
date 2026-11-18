"""Materialize only one requested revision, then run the released SDK and existing OCCT oracle."""
import argparse
import hashlib
import json
import re
from pathlib import Path
import struct
import subprocess
import sys
import tempfile

parser=argparse.ArgumentParser()
for key in ('store','runtime','inputs','output'):parser.add_argument(key,type=Path)
args=parser.parse_args()
here=Path(__file__).resolve().parent
sys.path.insert(0,str(here.parent/'field-task'))
from verify import verify

ledger=args.store/'index.jsonl'
offsets=args.store/'offsets'
# One-time construction for this experimental archive. A writer can append a
# single offset per commit in the same transaction as its member directory.
with ledger.open('rb') as idx, offsets.open('xb') as ptr:
    while True:
        offset=idx.tell()
        if not idx.readline():break
        ptr.write(struct.pack('<Q',offset))
assert offsets.stat().st_size==256*8
results=[]
with offsets.open('rb') as ptr,ledger.open('rb') as idx:
    for edit in (192,1,256,64,128,32):
        ptr.seek((edit-1)*8)
        idx.seek(struct.unpack('<Q',ptr.read(8))[0])
        entries=json.loads(idx.readline())
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp)
            for name,location,offset,size,sha in entries:
                relative=Path(name)
                if relative.is_absolute() or '..' in relative.parts or not re.fullmatch(r'segment-\d{4}\.bin',location):
                    raise ValueError('Unexpected archive member or segment')
                if not 0<=size<=16*1024**2 or offset<0:
                    raise ValueError('Member is outside this fixture budget')
                with (args.store/location).open('rb') as stream:
                    stream.seek(offset)
                    data=stream.read(size)
                assert hashlib.sha256(data).hexdigest()==sha
                target=root/name
                target.parent.mkdir(parents=True,exist_ok=True)
                target.write_bytes(data)
            step,=root.glob('artifacts/*/revision/model.step')
            receipt,=root.glob('steps/*/receipt.json')
            gauge=5+((edit-1)%5)*.5
            pitch=16+((edit-1)//5)*.25
            geometry=verify(step,args.inputs/'relocated.step',str(edit),7,11,54,gauge,pitch,True)
            cmd=['node',here/'lookup.mjs',args.runtime,step.parent,receipt,str(edit)]
            query=json.loads(subprocess.check_output(cmd,text=True))
            results.append(dict(edit=edit,member_bytes=sum(e[3] for e in entries),members=len(entries),
                                geometry=geometry,snapshot=query))
args.output.write_text(json.dumps(results,indent=2)+'\n')
