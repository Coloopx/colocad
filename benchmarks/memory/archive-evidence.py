"""Archive raw measurements without multi-GB geometry or synthetic ledgers."""
import argparse,hashlib,json,shutil,tarfile
from pathlib import Path

p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('dest',type=Path);a=p.parse_args()
here=Path(__file__).resolve().parent;repo=here.parent.parent;a.dest.mkdir(parents=True,exist_ok=True)
campaigns=['main','extreme','history','history-node','sharded','boundary','compositional','native-control']
metadata=['environment.json','verification.json','verification-manifest.json','catalog-verification.json','verification-manifest-diagnostic.json']
allowed={'measurement.json','result.json','events.jsonl','memory.jsonl','stdout.log','stderr.log','container-state.json','container.log','native.stdout','native.stderr'}
entries={}
for name in metadata:
    src=a.root/name
    if src.exists():shutil.copyfile(src,a.dest/name);entries[name]=src
for campaign in campaigns:
    folder=a.root/campaign;entries[campaign+'/runs.json']=folder/'runs.json'
    for run in json.loads((folder/'runs.json').read_text())['runs']:
        for src in (folder/run['id']).glob('*'):
            if src.is_file() and src.name in allowed:entries[str(src.relative_to(a.root))]=src
for src in sorted(here.rglob('*')):
    if src.is_file() and '__pycache__' not in src.parts and (src.suffix in {'.py','.mjs','.json','.md'} or src.name=='Dockerfile'):
        entries['source/'+str(src.relative_to(here))]=src
for src in (a.root/'source-snapshots').glob('*.py'):
    assert hashlib.sha256(src.read_bytes()).hexdigest()==src.stem
    entries['runner-snapshots/'+src.name]=src
entries['source/native-release.json']=repo/'kernel/release.json'
hashes={name:hashlib.sha256(src.read_bytes()).hexdigest() for name,src in sorted(entries.items())}
with tarfile.open(a.dest/'raw-evidence.tar.gz','w:gz') as archive:
    for name,src in sorted(entries.items()):
        info=archive.gettarinfo(str(src),arcname=name);info.uid=info.gid=0;info.uname=info.gname='';info.mtime=0
        with src.open('rb') as f:archive.addfile(info,f)
hashes['raw-evidence.tar.gz']=hashlib.sha256((a.dest/'raw-evidence.tar.gz').read_bytes()).hexdigest()
(a.dest/'raw-manifest.json').write_text(json.dumps(hashes,indent=2)+'\n')
print(json.dumps({'archived_files':len(entries),'archive_bytes':(a.dest/'raw-evidence.tar.gz').stat().st_size,'archive_sha256':hashes['raw-evidence.tar.gz']}))
