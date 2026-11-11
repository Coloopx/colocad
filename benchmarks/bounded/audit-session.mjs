// Independent readback of every committed ledger pointer, outside the timed build.
import assert from 'node:assert/strict';
import {readFile,realpath} from 'node:fs/promises';
import {join} from 'node:path';
import {createHash} from 'node:crypto';
import {ModelingSession} from '../../src/session.mjs';
const project=await realpath(process.argv[2]),s=await ModelingSession.open(project);
let cursor=0,count=0,queries=0,geometry=0,last;
const checked=async(file,hash)=>{const bytes=await readFile(join(project,file));assert.equal(createHash('sha256').update(bytes).digest('hex'),hash);return bytes;};
do{
  const page=await s.history({after:cursor,limit:128});
  for(const e of page.steps){
    assert.equal(e.id,'s'+String(++count).padStart(4,'0'));assert.equal(e.status,'succeeded');
    const reply=await s.receipt(e);assert.equal(reply.ok,true);
    if(e.operation==='query'){
      const q=JSON.parse(await checked(e.queryRequestFile,e.queryRequestSha256));
      assert.equal(q.query_revision_id,last);assert.ok(reply.data.results[0].items.length>0);
      assert.equal(e.inputSha256,geometry.inputSha256);queries++;
    }else{
      const m=JSON.parse(await checked(e.inputFile,e.inputSha256));
      assert.ok(Math.abs(reply.data.geometry.volume_mm3-m.parameters.height*200)<1e-6);
      await checked(e.artifactDirectory+'/model.step',reply.data.artifact_sha256['model.step']);
      last=reply.data.query_revision_id;geometry=e;
    }
  }
  cursor=page.nextCursor;
}while(cursor!==null);
assert.equal(count,10001);assert.equal(queries,9900);assert.equal((await s.latestGeometry()).id,'s10001');
console.log(JSON.stringify({ok:true,count,queries,geometry:count-queries,head:geometry.id}));
