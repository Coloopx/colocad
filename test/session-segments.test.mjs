import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,rm,readFile,writeFile,rename,open,readdir,stat,mkdir} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {execFileSync,spawn} from 'node:child_process';
import {once} from 'node:events';
import {ModelingSession,readSessionBundle} from '../src/session.mjs';

const model={version:1,units:'mm',parameters:{width:12},features:[
  {id:'stock',op:'box',origin:[0,0,0],size:[{param:'width'},10,5]}
],output:'stock',preview:{width:80,height:80}};
async function fixture(t) {
  const dir=await mkdtemp(join(tmpdir(),'colocad-segments-'));
  t.after(()=>rm(dir,{force:true,recursive:true}));
  const session=await ModelingSession.create(join(dir,'session'),{storage:'segmented'});
  const first=await session.build(model,{queryable:true});
  assert.ok(first.ok,JSON.stringify(first));
  return {dir,session,first};
}
const request=id=>({schema:'colocad.query/1',query_revision_id:id,target:{definition_id:'stock'},
  space:'definition',tolerance:{linear_mm:1e-5,angular_rad:1e-7},queries:[{id:'faces',op:'faces',limit:16}]});

test('cold inputs, aliased inspection source, exact query and viewer survive segment rotation and relocation',async t=>{
  const {dir,session,first}=await fixture(t);
  const original=await readFile(join(first.data.artifact_directory,'model.step'));
  const q=request(first.data.query_revision_id);
  assert.ok((await session.query(q)).ok);
  assert.ok((await session.observe({view:'front'})).ok);
  const firstEntry=(await session.history()).steps[0];
  for(let width=13;width<32;width++)assert.ok((await session.edit({width})).ok);
  assert.equal((await readdir(join(session.root,'segments'))).length,2);
  await assert.rejects(stat(first.data.artifact_directory),{code:'ENOENT'});
  const moved=join(dir,'moved');await rename(session.root,moved);
  const reopened=await ModelingSession.open(moved,{queryRoots:[dir]});
  const dest=join(dir,'delivered');
  const paths=JSON.parse(execFileSync(process.execPath,['bin/colocad.mjs','checkout',moved,firstEntry.id,dest]));
  assert.deepEqual(await readFile(join(paths.artifactDirectory,'model.step')),original);
  assert.equal(JSON.parse(await readFile(paths.inputFile)).parameters.width,12);
  assert.deepEqual(JSON.parse(await readFile(paths.receiptFile)),await reopened.receipt(firstEntry));
  const query=await reopened.cad.query(paths.artifactDirectory,q);
  assert.ok(query.ok,JSON.stringify(query.error));
  assert.equal(query.data.execution.geometry_build_calls,0);
  assert.equal(query.data.results[0].items.length,6);
  await assert.rejects(reopened.checkout(firstEntry.id,dest),{code:'EEXIST'});
  const bundle=await readSessionBundle(moved);
  assert.equal(bundle.steps.length,22);
  assert.equal(bundle.steps[1].model.parameters.width,12);
  assert.ok(bundle.steps[2].scene.instances.length);
  assert.ok(bundle.steps.at(-1).stepFile);
});

test('an incomplete archive rolls back head and retry overwrites uncommitted bytes',async t=>{
  const {dir,session,first}=await fixture(t);
  const stable=await session.head(),folder=join(session.root,'segments');
  const path=join(folder,'history-1.pack');
  // Fail after archive_dir has appended bytes, before the head transaction can publish.
  const unexpected=join(session.root,'steps','s0001','unexpected-directory');
  await mkdir(unexpected);
  await assert.rejects(session.edit({width:21}),/invalid archive member/);
  assert.ok((await stat(path)).size>0);
  assert.equal((await session.head()).session_step,stable.session_step);
  assert.ok((await stat(join(first.data.artifact_directory,'model.step'))).isFile());
  await rm(unexpected,{recursive:true});
  const reopened=await ModelingSession.open(session.root,{queryRoots:[dir]});
  assert.equal((await reopened.history()).steps[1].status,'interrupted');
  const delivered=await reopened.edit({width:23});assert.ok(delivered.ok);
  const before=(await reopened.checkout('s0001',join(dir,'checkout'))).artifactDirectory;
  assert.equal((await reopened.cad.query(before,request(first.data.query_revision_id))).ok,true);
});

test('retained editable source is still SHA checked and cold member corruption rejects checkout',async t=>{
  const {dir,session}=await fixture(t);
  assert.ok((await session.edit({width:15})).ok);
  const latest=await session.latestGeometry(),path=join(session.root,latest.inputFile);
  const saved=await readFile(path);
  await writeFile(path,JSON.stringify({...JSON.parse(saved),parameters:{width:140}}));
  await assert.rejects(session.edit({width:17}),/SESSION_INTEGRITY/);
  await writeFile(path,saved);
  const pack=join(session.root,'segments/history-1.pack'),handle=await open(pack,'r+');
  try {
    const x=Buffer.alloc(1);await handle.read(x,0,1,0);x[0]^=1;await handle.write(x,0,1,0);
  } finally {await handle.close();}
  const dest=join(dir,'corrupt');
  await assert.rejects(session.checkout('s0001',dest),/SESSION_INTEGRITY/);
  await assert.rejects(stat(dest),{code:'ENOENT'});
  assert.equal((await session.head()).session_step,latest.id);
});

test('reader holding the accepted revision sees its complete loose files while another process retires it',async t=>{
  const {dir,session,first}=await fixture(t);
  let finish,release;
  const readStarted=new Promise(done=>{finish=done;});
  const hold=new Promise(done=>{release=done;});
  const reader=session.withStore(false,async store=>{
    const prior=await store.head();
    finish();
    await hold;
    const receipt=await session.receiptFrom(store,prior);
    assert.deepEqual(await readFile(join(first.data.artifact_directory,'model.step.sha256'),'utf8'),
      receipt.data.artifact_sha256['model.step']+'\n');
  });
  await readStarted;
  const writer=new ModelingSession(session.root);
  const editing=writer.edit({width:24});
  await new Promise(done=>setTimeout(done,100));
  release();await reader;
  assert.ok((await editing).ok);
  const paths=await writer.checkout('s0001',join(dir,'after-reader'));
  assert.ok((await stat(join(paths.artifactDirectory,'model.step'))).isFile());
});

test('SIGKILL during segment append keeps the acknowledged head, then a fresh writer recovers',async t=>{
  const {dir,session,first}=await fixture(t);
  const producer=`
    import {ModelingSession} from './src/session.mjs';
    const session=await ModelingSession.open(process.argv[1]);
    const original=session.withStore.bind(session);
    session.withStore=(write,callback)=>original(write,store=>callback({...store,finish:async row=>{
      const gate=new Promise(done=>process.once('message',done));
      process.send('ready');await gate;
      return store.finish(row);
    }}));
    await session.edit({width:35});
  `;
  const writer=spawn(process.execPath,['--input-type=module','-e',producer,session.root],
    {detached:true,stdio:['ignore','ignore','pipe','ipc']});
  const exited=once(writer,'exit');
  try {
    assert.deepEqual((await once(writer,'message')).slice(0,1),['ready']);
    await session.withStore(false,async store=>{
      assert.equal((await store.head()).id,first.session_step);
      writer.send('finish');
      const pack=join(session.root,'segments/history-1.pack');
      const deadline=Date.now()+4000;
      while(!(await stat(pack).catch(()=>null))?.size){
        assert.ok(Date.now()<deadline,'catalog reached the append boundary');
        await new Promise(done=>setTimeout(done,3));
      }
      process.kill(-writer.pid,'SIGKILL');await exited;
    });
    const recovered=await ModelingSession.open(session.root);
    assert.equal((await recovered.head()).session_step,first.session_step);
    assert.equal((await recovered.history()).steps[1].status,'interrupted');
    assert.ok((await recovered.edit({width:18})).ok);
    const cold=await recovered.checkout(first.session_step,join(dir,'restored'));
    assert.equal((await readFile(join(cold.artifactDirectory,'model.step.sha256'),'utf8')).trim(),first.data.artifact_sha256['model.step']);
  } finally {
    if(writer.exitCode===null&&writer.signalCode===null)process.kill(-writer.pid,'SIGKILL');
  }
});
