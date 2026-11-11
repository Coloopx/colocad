"""Portable records of every extended observation, including rejected and unrun jobs."""
import collections,hashlib,json,re,statistics,sys
from pathlib import Path
out,dest=map(Path,sys.argv[1:]);read=lambda p:json.loads(p.read_bytes())
def clean(row):
    return {k:row[k] for k in ['id','backend','family','mode','variant','n','rep','memory_mib','timeout_s','status','exit_code','wall_s','peak_bytes','sampled_max','events','retained_output_bytes','container_oom_killed'] if k in row}
record={'scope':'Supplementary controls: separate delivery contracts and budgets. Do not pool with the 207 primary STEP runs. Legacy JSON and SQLite synthetic history observations are not 500k executed edits. Disk shards are not one assembly STEP.','campaigns':{}}
for name in ('boundary','extreme','compositional','sharded','native-control','history','history-node'):
    data=read(out/name/'runs.json');rs=[]
    for row in data['runs']:
        r=clean(row);error=row.get('result',{}).get('error',{});error=error if isinstance(error,dict) else {};r['error_code']=error.get('code')
        stderr=out/name/r['id']/'stderr.log'
        if r['status']=='error' and stderr.is_file() and 'heap out of memory' in stderr.read_text().lower():r['classification']='runtime_heap_oom'
        if r['status']=='rejected':r['classification']=r['error_code']
        rs.append(r)
    plan=Path(__file__).resolve().parents[1]/'memory'/('plan-'+name+'.json');assert [r['id'] for r in rs]==[r['id'] for r in read(plan)]
    record['campaigns'][name]={'planned':len(rs),'outcomes':dict(collections.Counter(r.get('classification',r['status']) for r in rs)),'rows':rs}
queries={}
for lbl in ['before','after']:
    q=[read(f) for f in sorted((out/'queries').glob('query-'+lbl+'-*/measurement.json'))];assert len(q)==3 and all(r['status']=='ok' for r in q)
    queries[lbl]=[{**clean(r),'result':r['result']} for r in q]
assert len({r['result']['resultHash'] for q in queries.values() for r in q})==1
record['queries']={'baseline_commit':'e568748728cfef03ba346e397672537a95c08c55','native_held_fixed':True,'runs':queries}
allcatalog=[(p.parent.name,read(p)) for p in sorted((out/'catalog').glob('*/measurement.json'))];assert len(allcatalog)==16 and all(r['status']=='ok' for name,r in allcatalog)
record['indexed_session']=[{'id':name,**clean(r),'result':r.get('result')} for name,r in allcatalog]
record['preview']=read(out/'preview.log');assert len(record['preview'])==24
record['chunk_catalogs']=read(out/'catalog-verification.json');assert len(record['chunk_catalogs'])==15 and all(r['passed'] for r in record['chunk_catalogs'])
record['shard_sample_readbacks']=read(out/'shard-samples-verified.json');assert len(record['shard_sample_readbacks'])==9
ids={r['id'] for r in record['chunk_catalogs']}
record['monolithic_control_readbacks']=[read(p)[0] for p in sorted((out/'aux-readback').glob('*-verified.json'))]
expected={name+'/'+r['id'] for name in ('boundary','extreme','compositional','native-control') for r in record['campaigns'][name]['rows'] if r['status']=='ok'}
assert {r['id'] for r in record['monolithic_control_readbacks']}==expected
record['readbacks']={'scaling':read(out/'main-verified.json'),'products':read(out/'fixtures-verified.json'),'windows':read(out/'windows-verified.json')}
(dest/'controls.json').write_text(json.dumps(record,indent=2)+'\n')
print('controls verified',len(expected),'extra monolithic exports, 15 catalogs, 9 chunk samples, 16 session cases')
