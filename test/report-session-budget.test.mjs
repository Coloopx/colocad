/** [INPUT]: 真实session、文件/packed回执、当前宿主预算与可信SHA。
 * [OUTPUT]: session reader/observer不继承producer、gzip膨胀受限及结构化超限/摘要控制。
 * [POS]: reportBytes集成回归，仅Windows执行。
 * [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md */
import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,rm,readFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {createHash} from 'node:crypto';
import {CadClient} from '../kernel/dist/index.js';
import {ModelingSession,readSessionBundle} from '../src/session.mjs';
import {storeReceipt,readPacked} from '../src/session-payload.mjs';
const MiB=1024*1024,sha=b=>createHash('sha256').update(b).digest('hex');
async function root(t){const p=await mkdtemp(join(tmpdir(),'report-session-'));t.after(()=>rm(p,{recursive:true,force:true}));return p;}
const denied=e=>e.code==='RESOURCE_LIMIT'&&e.details.resource==='reportBytes'&&e.details.limit_bytes===1024;
test('file session receipt and bundle use current report budget rather than producer metadata',async t=>{
  const p=await root(t),s=await ModelingSession.create(p,{storage:'json'});
  const m={version:1,units:'mm',parameters:{},features:[{id:'box',op:'box',origin:[0,0,0],size:[2,3,4]}],output:'box',preview:{width:64,height:64,chord_mm:0.5}};
  const r=await s.build(m);assert.equal(r.ok,true,JSON.stringify(r));
  const [step]=(await s.read()).steps,raw=await readFile(join(p,step.receiptFile));assert.ok(raw.length>1024);
  const before=sha(raw),strict=await ModelingSession.open(p,{resourceLimits:{reportBytes:1024}});
  await assert.rejects(strict.receipt(step),denied);
  await assert.rejects(readSessionBundle(p,{resourceLimits:{reportBytes:1024}}),denied);
  assert.equal(sha(await readFile(join(p,step.receiptFile))),before);
});
test('packed report decoding uses current bounded inflated bytes and retains SHA rejection',async t=>{
  const p=await root(t),raw=JSON.stringify({ok:true,data:{public_note:'x'.repeat(2*MiB)}}),digest=sha(raw);
  const ref=await storeReceipt(p,'s0001',raw,8*MiB);assert.ok(ref);
  assert.equal((await readPacked(p,ref,digest,8*MiB)).data.public_note.length,2*MiB);
  await assert.rejects(readPacked(p,ref,digest,1024),denied);
  await assert.rejects(readPacked(p,ref,'0'.repeat(64),8*MiB),/checksum/);
});
test('observeArtifact preserves structured reportBytes refusal from a real session producer',async t=>{
  const p=await root(t),producer=await ModelingSession.create(join(p,'producer'),{storage:'json'});
  const model={version:1,units:'mm',parameters:{},features:[{id:'box',op:'box',origin:[0,0,0],size:[2,3,4]}],output:'box',preview:{width:64,height:64,chord_mm:0.5}};
  const built=await producer.build(model);assert.equal(built.ok,true,JSON.stringify(built));
  const directory=built.data.artifact_directory;
  assert.match(built.data.artifact_sha256['preview.mesh.json'],/^[a-f0-9]{64}$/);
  const file=join(directory,'report.json'),before=await readFile(file);
  assert.ok(before.length>1024,'real producer report must exceed strict reader budget');
  const strict=new CadClient({outputRoot:join(p,'observer'),resourceLimits:{reportBytes:1024}});
  const observed=await strict.observeArtifact(directory);
  assert.equal(observed.ok,false,JSON.stringify(observed));
  assert.equal(observed.error.code,'RESOURCE_LIMIT');
  assert.equal(observed.error.details.resource,'reportBytes');
  assert.equal(observed.error.details.limit_bytes,1024);
  assert.equal(observed.error.details.observed_bytes,before.length);
  assert.equal(observed.error.details.observed_is_lower_bound,false);
  assert.equal(sha(await readFile(file)),sha(before),'refused observer must preserve producer report');
});
