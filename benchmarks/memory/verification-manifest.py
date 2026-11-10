import argparse,hashlib,json,sqlite3
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('root',type=Path);a=p.parse_args();root=a.root.resolve();manifest=[]
def rows(c):return json.loads((root/c/'runs.json').read_text())['runs']
def add(c,r):
    if r['status']!='ok':return
    d=root/c/r['id'];res=r['result'];data=res.get('data',{})
    path=d/(data['artifact_directory'].removeprefix('/out/')+'/model.step') if data.get('artifact_directory') else d/'model.step'
    if c=='native-control':path=d/'native/model.step'
    assert path.is_file(),path
    manifest.append({'id':c+'/'+r['id'],'path':'/data/'+str(path.relative_to(root)),'family':r['family'],'n':r['n']})
for r in rows('main'):
    if r.get('rep')==0 and r['n']=={'part':1024,'unique':64,'repeated':128,'chain':64}[r['family']]:add('main',r)
for c in ['boundary','compositional','native-control']:
    for r in rows(c):add(c,r)
for b in ['cadquery','build123d']:
    for family in ['part','unique','repeated','chain']:
        selected=[r for r in rows('extreme') if r['backend']==b and r['family']==family and r['status']=='ok']
        if selected:add('extreme',max(selected,key=lambda r:r['n']))
catalog_checks=[]
for r in rows('sharded'):
    assert r['status']=='ok',r['id']
    d=root/'sharded'/r['id'];wal=d/'project.sqlite-wal'
    # These are finished, closed artifacts, not a concurrently written catalog.
    # Never use immutable mode while an uncheckpointed WAL may contain commits.
    assert not wal.exists() or wal.stat().st_size==0
    db=sqlite3.connect('file:'+str(d/'project.sqlite')+'?mode=ro&immutable=1',uri=True);count=0;total=0;recorded_volume=0
    records=[]
    for start,n,raw in db.execute('SELECT start,count,record FROM chunks ORDER BY start'):
        assert start==count;record=json.loads(raw);f=d/record['path'];h=hashlib.sha256()
        with f.open('rb') as stream:
            for b in iter(lambda:stream.read(1024*1024),b''):h.update(b)
        assert h.hexdigest()==record['sha256'];assert f.stat().st_size==record['step_bytes'];count+=n;total+=f.stat().st_size;recorded_volume+=record['volume_mm3'];records.append((start,n,record))
    assert count==r['n'];assert total==r['result']['step_bytes'];assert abs(recorded_volume-r['result']['volume_mm3'])<1e-5
    assert db.execute('PRAGMA integrity_check').fetchone()[0]=='ok';db.close()
    catalog_checks.append({'id':r['id'],'passed':True,'parts':count,'chunks':len(records),'step_bytes':total,'all_chunk_hashes_verified':True,'contiguous_coverage':True,'sqlite_integrity':'ok'})
    if r['n']==512:
        for index in [0,len(records)//2,len(records)-1]:
            start,n,record=records[index];path=d/record['path'];manifest.append({'id':'sharded/'+r['id']+'/chunk-'+str(start),'path':'/data/'+str(path.relative_to(root)),'family':'unique','n':n,'start':start})
(root/'verification-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
(root/'catalog-verification.json').write_text(json.dumps(catalog_checks,indent=2)+'\n')
print(json.dumps({'step_deliveries':len(manifest),'catalogs':len(catalog_checks),'all_catalog_hashes_and_ranges':'passed'}))
