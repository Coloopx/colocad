import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, readFile, writeFile, rm, rename, stat, open } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { once } from 'node:events';
import { createHash } from 'node:crypto';
import { spawn, execFileSync } from 'node:child_process';
import { ModelingSession } from '../src/session.mjs';

const model={version:1,units:'mm',parameters:{z:10},features:[
  {id:'stock',op:'box',origin:[0,0,0],size:[10,20,{param:'z'}]},
],output:'stock',preview:{width:160,height:120,chord_mm:1}};
function faces(revision) {
  return {schema:'colocad.query/1',query_revision_id:revision,target:{definition_id:'stock'},
    space:'definition',tolerance:{linear_mm:1e-5,angular_rad:1e-7},queries:[{id:'all',op:'faces',limit:16}]};
}
async function directory(t) {
  const root=await mkdtemp(join(tmpdir(),'colocad-m1-'));
  t.after(()=>rm(root,{recursive:true,force:true}));
  return root;
}
function cli(...args) {
  return JSON.parse(execFileSync(process.execPath,['bin/colocad.mjs',...args],{encoding:'utf8'}));
}

test('CLI migration preserves ledger, source, exact query refs and pagination across relocation',async t=>{
  const root=await directory(t), original=join(root,'old');
  const old=await ModelingSession.create(original,{storage:'json',title:'Legacy'});
  const initial=await old.build(model,{queryable:true});
  const q=faces(initial.data.query_revision_id);
  const previous=await old.query(q);
  assert.equal(previous.ok,true,JSON.stringify(previous.error));
  assert.equal((await old.edit({typo:3})).ok,false);
  const ledger=await old.read(), bytes=await readFile(join(original,'session.json'));
  const migrated=cli('migrate',original);
  assert.equal(migrated.steps,3);
  assert.deepEqual(await readFile(join(original,migrated.backup)),bytes);
  assert.equal(cli('migrate',original).migrated,false);
  const moved=join(root,'moved');await rename(original,moved);
  const session=await ModelingSession.open(moved);
  assert.deepEqual(await session.read(),ledger);
  const first=cli('history',moved,'--limit','2');
  assert.deepEqual(first.steps,ledger.steps.slice(0,2));
  assert.deepEqual(await session.history({after:first.nextCursor}),{steps:[ledger.steps[2]],nextCursor:null});
  assert.deepEqual(cli('history',moved,'--all'),ledger);
  const existingRef=previous.data.results[0].items[0].ref;
  const describe={...q,queries:[{id:'ref',op:'describe',refs:[existingRef]}]};
  assert.equal((await session.query(describe)).ok,true);
  assert.equal((await session.edit({z:12})).ok,true);
  assert.equal((await session.query(describe)).error.code,'QUERY_REVISION_MISMATCH');
  assert.equal((await session.latestGeometry()).id,'s0005');
});

test('a malformed legacy tail cannot publish a partial catalog and a corrected retry succeeds',async t=>{
  const root=await directory(t);
  await ModelingSession.create(root,{storage:'json',title:'old ledger '.repeat(100)});
  const manifest=join(root,'session.json');
  const valid=await readFile(manifest,'utf8'), damaged=valid+'bad tail';
  await writeFile(manifest,damaged);
  await assert.rejects(ModelingSession.migrate(root));
  assert.equal(await readFile(manifest,'utf8'),damaged);
  await writeFile(manifest,valid);
  assert.equal((await ModelingSession.migrate(root)).migrated,true);
  assert.equal(await new ModelingSession(root).latestGeometry(),undefined);
});

test('indexed writer excludes a second writer while readers can inspect the running step',async t=>{
  const root=await directory(t),session=await ModelingSession.create(root);
  let release, started;
  const running=new Promise(r=>started=r), gate=new Promise(r=>release=r);
  session.cad.build=async()=>{started();await gate;return {ok:false,error:{code:'TEST_FAILURE',message:'test'}};};
  const first=session.build(model);
  try {
    await running;
    const reader=new ModelingSession(root);
    assert.equal((await reader.history()).steps[0].status,'running');
    await assert.rejects(reader.build(model),/SESSION_BUSY/);
    await assert.rejects(reader.history({limit:0}),/SESSION_PAGE/);
    assert.equal(await reader.latestGeometry(),undefined);
  } finally {release();await first;}
  assert.equal((await session.history()).steps[0].status,'failed');
});

test('killing a streaming migration preserves v1 and retry discards the partial database',async t=>{
  const root=await directory(t), source=join(root,'session.json');
  const file=await open(source,'wx');
  const n=50000, hash=createHash('sha256');
  async function write(chunk) {hash.update(chunk);await file.writeFile(chunk);}
  try {
    await write('{"title":"Interrupted migration","steps":[');
    for(let i=0;i<n;i+=1000) {
      const block=Array.from({length:1000},(_,j)=>JSON.stringify({
        id:'s'+String(i+j+1).padStart(4,'0'),status:'succeeded',operation:'build',label:'large legacy import',
      }));
      await write((i?',':'')+block.join(','));
    }
    await write('],"format":"colocad/session/1","id":"test"}');
  } finally {await file.close();}
  const child=spawn(process.execPath,['bin/colocad.mjs','migrate',root],{detached:true,stdio:'ignore'});
  const exit=once(child,'exit');
  try {
    const deadline=Date.now()+10000;
    for(;;) {
      const stage=await stat(join(root,'.colocad-import.tmp')).catch(()=>null);
      if(stage && stage.size>0) break;
      assert.ok(Date.now()<deadline && child.exitCode===null,'migration should be writing its private database');
      await new Promise(r=>setTimeout(r,2));
    }
    process.kill(-child.pid,'SIGKILL');
    await exit;
  } finally {try{process.kill(-child.pid,'SIGKILL');}catch{}}
  assert.equal(createHash('sha256').update(await readFile(source)).digest('hex'),hash.digest('hex'));
  const retry=await ModelingSession.migrate(root);
  assert.equal(retry.steps,n);
  assert.equal((await new ModelingSession(root).latestGeometry()).id,'s50000');
  await assert.rejects(stat(join(root,'.colocad-import.tmp')),{code:'ENOENT'});
});
