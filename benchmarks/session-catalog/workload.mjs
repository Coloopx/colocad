import assert from 'node:assert/strict';
import { copyFile, mkdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { ModelingSession } from '../../src/session.mjs';

const [operation, countText, directory] = process.argv.slice(2);
const count = Number(countText);

async function appendHistory() {
  const session = await ModelingSession.create(directory);
  const model = {version:1,units:'mm',parameters:{height:10},features:[
    {id:'stock',op:'box',origin:[0,0,0],size:[10,20,{param:'height'}]},
  ],output:'stock',preview:{width:160,height:120,chord_mm:1}};
  let geometry = await session.build(model,{queryable:true});
  assert.equal(geometry.ok,true,JSON.stringify(geometry.error));
  let edits = 0, queries = 0;
  for (let i=1; i<=count; i++) {
    let receipt;
    if(i%100===0) {
      const height = 10+(i/100)%7;
      geometry = await session.edit({height});
      receipt = geometry;
      assert.equal(receipt.ok,true,JSON.stringify(receipt.error));
      assert.ok(Math.abs(receipt.data.geometry.volume_mm3-200*height)<1e-6);
      edits++;
    } else {
      receipt = await session.query({
        schema:'colocad.query/1',query_revision_id:geometry.data.query_revision_id,
        target:{definition_id:'stock'},space:'definition',tolerance:{linear_mm:1e-5,angular_rad:1e-7},
        queries:[{id:'faces',op:'faces',where:{planar_normal_aligned_with:[0,0,1]},limit:16}],
      });
      assert.equal(receipt.ok,true,JSON.stringify(receipt.error));
      assert.ok(receipt.data.results[0].items.length>0);
      queries++;
    }
    assert.equal(receipt.session_step,'s'+String(i+1).padStart(4,'0'));
    if(i%1000===0) console.log(i, new Date().toISOString());
  }
  const reopened = await ModelingSession.open(directory);
  assert.equal((await reopened.latestGeometry()).id, geometry.session_step);
  let after=0, seen=0;
  do {
    const page=await reopened.history({after,limit:128});
    for(const entry of page.steps) {
      assert.equal(entry.id,'s'+String(++seen).padStart(4,'0'));
      assert.equal(entry.status,'succeeded');
    }
    after=page.nextCursor;
  } while(after!==null);
  assert.equal(seen,count+1);
  return {ok:true,edits,queries,initialBuilds:1,verifiedInOrder:seen,head:geometry.session_step};
}

async function run() {
  if(operation==='append') return appendHistory();
  if(operation==='prepare') {
    await mkdir(directory,{recursive:true});
    await copyFile(join('/fixtures',countText,'session.json'),join(directory,'session.json'));
    return {ok:true,synthetic:true,count};
  }
  if(operation==='migrate') {
    const result=await ModelingSession.migrate(directory);
    assert.equal(result.steps,count);
    return {ok:true,synthetic:true,count};
  }
  if(operation==='head') {
    const session=new ModelingSession(directory);
    const head=await session.latestGeometry();
    assert.equal(head.id,'s'+String(count).padStart(7,'0'));
    return {ok:true,synthetic:true,count,head:head.id};
  }
  throw Error('unknown operation');
}
await writeFile('/out/result.json',JSON.stringify(await run())+'\n');
