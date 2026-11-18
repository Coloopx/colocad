/** [INPUT]: 真实Node子进程与有界原生事件夹具。
 * [OUTPUT]: 有效进展续期、重放/日志不续期、取消与旧wall预算回归。
 * [POS]: SDK执行监督测试，不执行模型几何。
 * [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md */
import {mkdtemp,rm} from 'node:fs/promises';import {join} from 'node:path';import {tmpdir} from 'node:os';
import test from 'node:test';import assert from 'node:assert/strict';
import {execute,receipt} from '../kernel/dist/process.js';
import {progressReader} from '../kernel/dist/progress.js';
import {CadClient} from '../kernel/dist/client.js';
const line=n=>'COLOCAD_PROGRESS '+JSON.stringify({schema:'colocad.progress/1',seq:n,phase:'feature',state:'completed',feature_id:'box'})+'\n';
test('host can choose idle supervision and a wall deadline larger than the old 120 seconds',()=>{
 assert.doesNotThrow(()=>new CadClient({outputRoot:'/tmp',wallMs:120001}));
 for(const value of [0,-1,Infinity,NaN,2**31])assert.throws(()=>new CadClient({outputRoot:'/tmp',idleMs:value}));
});
test('parser binds a bounded monotonic event stream and ignores log noise, duplicates and malformed events',()=>{
 const seen=[],read=progressReader(row=>seen.push(row.seq));
 const raw=line(1);read(Buffer.from(raw.slice(0,10)));read(Buffer.from(raw.slice(10)));
 read(Buffer.from(line(1)+'busy\n'+'COLOCAD_PROGRESS '+JSON.stringify({schema:'colocad.progress/1',seq:2,phase:'heartbeat',state:'completed'})+'\n'+line(3)));
 read(Buffer.from('x'.repeat(10000)+'\n'+line(4)));assert.deepEqual(seen,[1,3,4]);
});
test('valid feature progress keeps a real child alive beyond an old total deadline',async()=>{
 const script=`let seq=0;const timer=setInterval(()=>{process.stderr.write('COLOCAD_PROGRESS '+JSON.stringify({schema:'colocad.progress/1',seq:++seq,phase:'feature',state:'completed',feature_id:'part'})+'\\n');if(seq===12){clearInterval(timer);console.log(JSON.stringify({ok:true,data:{done:true}}));}},50);`;
 const seen=[];const result=await execute(process.execPath,['-e',script],'',null,2048,{idleMs:500,onProgress:p=>seen.push(p.seq)});
 assert.equal(receipt(result).ok,true,JSON.stringify(result));assert.equal(seen.length,12);assert.equal(result.idleTimedOut,false);
});
test('ordinary stderr noise cannot renew inactivity and the child is reaped',async()=>{
 const result=await execute(process.execPath,['-e',"setInterval(()=>process.stderr.write('still busy\\n'),10)"],'',null,2048,{idleMs:200});
 assert.equal(receipt(result).error.code,'IDLE_TIMEOUT');assert.equal(result.code,null);
});
test('host cancellation is distinct from idle or wall expiry and observer failure does not change receipt',async()=>{
 const abort=new AbortController();const script=`process.stderr.write(${JSON.stringify(line(1))});setInterval(()=>{},1000);`;
 const result=await execute(process.execPath,['-e',script],'',null,2048,{idleMs:500,signal:abort.signal,onProgress:()=>abort.abort()});
 assert.equal(receipt(result).error.code,'CANCELLED');
 const ok=await execute(process.execPath,['-e',`process.stderr.write(${JSON.stringify(line(1))});console.log('{"ok":true,"data":{}}');`],'',null,2048,{idleMs:500,onProgress:()=>{throw Error('observer');}});
 assert.equal(receipt(ok).ok,true);
 const wall=await execute(process.execPath,['-e','setInterval(()=>{},1000)'],'',100,2048);assert.equal(receipt(wall).error.code,'TIMEOUT');
});

test('replayed protocol events do not create progress and async observer rejection is isolated',async()=>{
 const result=await execute(process.execPath,['-e',`setInterval(()=>process.stderr.write(${JSON.stringify(line(1))}),20)`],'',null,2048,{idleMs:250});
 assert.equal(receipt(result).error.code,'IDLE_TIMEOUT');assert.equal(receipt(result).error.feature_id,'box');
 const ok=await execute(process.execPath,['-e',`process.stderr.write(${JSON.stringify(line(1))});console.log('{"ok":true,"data":{}}');`],'',null,2048,{idleMs:1000,onProgress:async()=>{throw Error('async observer');}});
 assert.equal(receipt(ok).ok,true);
});

test('SDK forwards native feature identity and exact early-collision diagnostics without changing stdout receipts',async t=>{
 const root=await mkdtemp(join(tmpdir(),'colocad-progress-'));t.after(()=>rm(root,{recursive:true,force:true}));
 const progress=[],cad=new CadClient({outputRoot:root,idleMs:30000,onProgress:event=>progress.push(event)});
 const cap=await cad.capabilities();assert.equal(cap.ok,true,JSON.stringify(cap));assert.equal(cap.data.native_progress.schema,'colocad.progress/1');
 const m={version:1,units:'mm',parameters:{},features:[{id:'a',op:'box',origin:[0,0,0],size:[10,10,10]},{id:'b',op:'box',origin:[5,0,0],size:[10,10,10]}],assembly:[{id:'first',input:'a'},{id:'second',input:'b'}],checks:[{kind:'collision',parts:['first','second'],max_overlap_mm3:0.001}]};
 const failed=await cad.build(m);assert.equal(failed.ok,false);assert.equal(failed.error.code,'ASSEMBLY_COLLISION');assert.equal(failed.error.feature_id,'b');
 assert.deepEqual(failed.error.details.source_features,['a','b']);assert.ok(progress.some(p=>p.feature_id==='a'&&p.state==='completed'));
 m.checks[0].allowed_pairs=[['first','second']];const clean=await cad.build(m);assert.equal(clean.ok,true,JSON.stringify(clean));
 const replay=await cad.build(m);assert.equal(replay.ok,true,JSON.stringify(replay));
 assert.ok(progress.filter(p=>p.feature_id==='b'&&p.phase==='feature'&&p.state==='completed').length>=2);
 assert.equal(replay.data.assembly.checks[0].overlap_mm3,clean.data.assembly.checks[0].overlap_mm3);
});

test('public build CLI forwards host inactivity and preserves the native refusal', async t => {
  const {writeFile} = await import('node:fs/promises');
  const {execFile} = await import('node:child_process');
  const {promisify} = await import('node:util');
  const {fileURLToPath} = await import('node:url');
  const root = await mkdtemp(join(tmpdir(), 'colocad-cli-policy-'));
  t.after(() => rm(root, {recursive:true, force:true}));
  const binary = join(root, 'paced.cjs');
  await writeFile(binary, '#!/usr/bin/env node\n'+`
    process.stdin.resume();
    let seq = 0;
    const timer = setInterval(() => {
      if (!process.argv.includes('--progress')) return;
      const event = {schema:'colocad.progress/1',seq:++seq,phase:'feature',state:'completed',feature_id:'body'};
      process.stderr.write('COLOCAD_PROGRESS '+JSON.stringify(event)+'\\n');
      if (seq===12) {
        clearInterval(timer);
        console.log(JSON.stringify({ok:false,error:{code:'ASSEMBLY_COLLISION',message:'pair',details:{parts:['a','b']}}}));
        process.exitCode = 1;
      }
    },250);
  `, {mode:0o755});
  const cli = fileURLToPath(new URL('../bin/colocad.mjs',import.meta.url));
  const model = fileURLToPath(new URL('../examples/manifold.json',import.meta.url));
  const invoke = (wallMs) => promisify(execFile)(process.execPath, [cli,'build',model,join(root,'job')], {
    env:{...process.env,AGENT_CAD_BINARY:binary,COLOCAD_IDLE_MS:'2000',COLOCAD_PROGRESS:'0',...(wallMs?{COLOCAD_WALL_MS:wallMs}:{})},
    timeout:15000
  });
  await assert.rejects(invoke(), e => {
    const failure = JSON.parse(e.stderr).receipt?.error;
    assert.equal(failure?.code, 'ASSEMBLY_COLLISION', e.stderr);
    assert.deepEqual(failure.details.parts, ['a','b']);
    return true;
  });
  await assert.rejects(invoke('150'), e => JSON.parse(e.stderr).receipt?.error.code==='TIMEOUT');
});
