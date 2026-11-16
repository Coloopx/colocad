/**
 * [INPUT]: 公开小盒体、SDK/CLI/session入口及公开STEP合法注释填充。
 * [OUTPUT]: 输出预算传播、能力一致、文件摘要/读取、readback与HTML权限回归。
 * [POS]: PR6输出预算回归；不更改生产代码，不执行私有CAD或海量几何。
 * [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
 */
import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,readFile,writeFile,rm,stat} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {createHash,randomUUID} from 'node:crypto';
import * as sdk from '../kernel/dist/index.js';
import {ModelingSession,readSessionBundle,exportSessionHtml} from '../src/session.mjs';

const MiB=1024*1024,defaults={inputBytes:32*MiB,stepBytes:24*MiB,totalStepBytes:24*MiB,outputStepBytes:32*MiB};
const box=JSON.parse(await readFile(new URL('../crates/agent-cad/tests/fixtures/output-budget-box.json',import.meta.url),'utf8'));
const digest=bytes=>createHash('sha256').update(bytes).digest('hex');
const accepted=r=>{assert.equal(r.ok,true,JSON.stringify(r));return r.data;};
const denied=limit=>error=>{
  assert.equal(error.code,'RESOURCE_LIMIT');
  assert.equal(error.details.resource,'outputStepBytes');
  assert.equal(error.details.limit_bytes,limit);
  assert.ok(error.details.observed_bytes>limit);
  assert.equal(typeof error.details.observed_is_lower_bound,'boolean');
  return true;
};
async function root(t){
  const path=await mkdtemp(join(tmpdir(),'colocad-output-budget-'));
  t.after(()=>rm(path,{recursive:true,force:true}));return path;
}
function client(path,resourceLimits){return new sdk.CadClient({outputRoot:path,resourceLimits,wallMs:120000});}
function model(assembly=false){
  const m=structuredClone(box);
  if(assembly){delete m.output;m.assembly=[{id:'first',input:'box',translation:[0,0,0]},{id:'second',input:'box',translation:[10,0,0]}];}
  return m;
}
async function reader(cad,directory){
  assert.equal(typeof sdk.readStepArtifact,'function','SDK must expose a checked, host-budgeted output artifact reader');
  return sdk.readStepArtifact(cad,directory);
}

test('output defaults and ceilings match native capabilities without changing input policy',async t=>{
  assert.equal(sdk.DEFAULT_RESOURCE_LIMITS.outputStepBytes,32*MiB);
  assert.equal(sdk.RESOURCE_LIMIT_CEILINGS.outputStepBytes,512*MiB);
  const path=await root(t),ordinary=client(path),report=accepted(await ordinary.capabilities());
  assert.deepEqual(report.resource_limits,defaults);
  assert.equal(report.limits.output_step_bytes,32*MiB);
  assert.deepEqual(report.resource_limit_ceilings,sdk.RESOURCE_LIMIT_CEILINGS);
  const raised=client(path,{outputStepBytes:64*MiB}),active=accepted(await raised.capabilities());
  assert.deepEqual(active.resource_limits,{...defaults,outputStepBytes:64*MiB});
  assert.equal(active.limits.output_step_bytes,64*MiB);
  assert.equal(raised.resourceLimits.inputBytes,32*MiB);
  assert.equal(Object.isFrozen(raised.resourceLimits),true);
});

test('output host values are bounded and snapshotted once',async t=>{
  const path=await root(t);
  for(const outputStepBytes of [0,-1,1.5,NaN,Infinity,512*MiB+1,'65536'])
    assert.throws(()=>client(path,{outputStepBytes}),e=>e.code==='INVALID_RESOURCE_LIMITS');
  const limits={outputStepBytes:65536};let reads=0;
  const cad=new sdk.CadClient({outputRoot:path,get resourceLimits(){reads++;return limits;}});
  limits.outputStepBytes=1;
  assert.equal(reads,1);
  assert.equal(accepted(await cad.capabilities()).resource_limits.outputStepBytes,65536);
});

for(const assembly of [false,true])test(`native ${assembly?'assembly':'single'} enforces exact small output boundary and stricter replay`,async t=>{
  const path=await root(t),source=model(assembly),normal=client(path);
  const first=accepted(await normal.build(source));
  const raw=await readFile(join(first.artifact_directory,'model.step'));
  assert.ok(raw.length>1024&&raw.length<MiB);
  assert.equal(first.validation.valid,true);assert.equal(first.geometry.solids,assembly?2:1);
  const strict=client(path,{outputStepBytes:raw.length-1});
  const result=await strict.build(source,{previousRevision:first.artifact_directory});
  assert.equal(result.ok,false);denied(raw.length-1)(result.error);
  const exact=client(path,{outputStepBytes:raw.length}),built=accepted(await exact.build(source));
  assert.equal(built.resource_usage.output_step_bytes,raw.length);
  assert.equal(built.resource_usage.total_step_bytes,0);
  assert.equal(built.resource_limits.outputStepBytes,raw.length);
  const artifact=await reader(exact,built.artifact_directory);
  assert.deepEqual(artifact,raw);assert.equal(digest(artifact),built.artifact_sha256['model.step']);
  // Genuine geometry readback under a separately sufficient input policy.
  const readback=await sdk.readModelFile(normal,join(built.artifact_directory,'model.step'));
  const reopened=accepted(await normal.build(readback));
  assert.equal(reopened.geometry.solids,assembly?2:1);
  assert.ok(Math.abs(reopened.geometry.volume_mm3-(assembly?48:24))<1e-6);
});

test('IR cannot grant output permission and output permission cannot grant input admission',async t=>{
  const path=await root(t),cad=client(path,{outputStepBytes:64*MiB});
  for(const field of ['resourceLimits','outputStepBytes']){
    const m=model();m[field]=field==='resourceLimits'?{outputStepBytes:64*MiB}:64*MiB;
    const rejected=await cad.validate(m);assert.equal(rejected.ok,false);
  }
  const strict=client(path,{stepBytes:1,totalStepBytes:1,outputStepBytes:64*MiB});
  const m=model();m.assets={source:{step:'xx',sha256:digest('xx')}};
  const rejected=await strict.validate(m);
  assert.equal(rejected.ok,false);assert.equal(rejected.error.details.resource,'stepBytes');
});

test('file export uses output budget and preserves genuine STEP readback',async t=>{
  const path=await root(t),cad=client(join(path,'jobs'),{outputStepBytes:65536});
  const file=join(path,'part.step'),saved=await sdk.writeModelFile(cad,model(),file);
  assert.ok(saved.bytes<65536);assert.equal(digest(await readFile(file)),saved.sha256);
  const readback=await sdk.readModelFile(cad,file);
  assert.equal(accepted(await cad.build(readback)).geometry.solids,1);
  const deniedFile=join(path,'must-not-exist.step');
  await assert.rejects(sdk.writeModelFile(client(join(path,'strict'),{outputStepBytes:1}),model(),deniedFile),denied(1));
  assert.equal(await stat(deniedFile).then(()=>true,e=>{assert.equal(e.code,'ENOENT');return false;}),false);
});

test('SDK reads >32MiB public artifact only with current explicit output permission and SHA intact',async t=>{
  const path=await root(t),normal=client(join(path,'jobs')),built=accepted(await normal.build(model()));
  const file=join(built.artifact_directory,'model.step');
  const small=await readFile(file,'utf8');
  // Legal STEP comment changes only bytes, never topology, entity counts or vendor data.
  const padded=Buffer.from(small.replace('ISO-10303-21;','ISO-10303-21;\n/*'+'x'.repeat(32*MiB)+'*/'));
  await writeFile(file,padded);
  const reportFile=join(built.artifact_directory,'report.json');
  const report=JSON.parse(await readFile(reportFile,'utf8'));
  report.artifact_sha256['model.step']=digest(padded);report.step_bytes=padded.length;
  // Producer policy must never authorize a default reader.
  report.resource_limits={...defaults,outputStepBytes:64*MiB};
  await writeFile(reportFile,JSON.stringify(report));
  // 固定摘要旁文件与合法注释扩展同步；它不继承producer预算。
  await writeFile(join(built.artifact_directory,'model.step.sha256'),digest(padded)+'\n');
  await assert.rejects(reader(normal,built.artifact_directory),denied(32*MiB));
  const raised=client(path,{outputStepBytes:64*MiB});
  assert.deepEqual(await reader(raised,built.artifact_directory),padded);
  // Isolate the public file codec's existing 32MiB check without huge native geometry.
  // Only its producer is a test double; the codec, policy, file I/O and SHA are real.
  raised.build=async()=>({ok:true,data:{...report,artifact_directory:built.artifact_directory}});
  const exported=join(path,'large-public.step');
  const saved=await sdk.writeModelFile(raised,model(),exported);
  assert.equal(saved.bytes,padded.length);assert.equal(saved.sha256,digest(padded));
  assert.deepEqual(await readFile(exported),padded);
  // Embedded editable containers must satisfy both output and input/container limits.
  const embedded=join(path,'too-large-container.step');
  await assert.rejects(sdk.writeModelFile(raised,model(),embedded,{embedModel:true}),
    e=>e.code==='RESOURCE_LIMIT'&&e.details.resource==='inputBytes');
  assert.equal(await stat(embedded).then(()=>true,e=>{assert.equal(e.code,'ENOENT');return false;}),false);
  // Reading output is distinct from reimporting foreign text as a model input.
  await assert.rejects(sdk.readModelFile(raised,file),e=>e.code==='RESOURCE_LIMIT'&&e.details.resource==='inputBytes');
  padded[padded.length-1]^=1;await writeFile(file,padded);
  await assert.rejects(reader(raised,built.artifact_directory),/digest|integrity|sha256/i);
});

async function sessionFixture(t){
  const path=await root(t),session=await ModelingSession.create(path,{storage:'json',resourceLimits:{outputStepBytes:65536}});
  const m=model();m.preview={chord_mm:0.5,width:64,height:64};
  accepted(await session.build(m));return {path,session};
}
async function read(kind,path,options){
  if(kind==='bundle')return readSessionBundle(path,options);
  const destination=join(path,'history-'+randomUUID()+'.html');
  await exportSessionHtml(path,destination,options);
  const html=await readFile(destination,'utf8');
  const match=html.match(/<script id="colocad-session" type="application\/json">([\s\S]*?)<\/script>/);
  assert.ok(match);return JSON.parse(match[1]);
}
for(const kind of ['bundle','html']){
  test(`${kind} uses current output permission rather than producer receipt and freezes options before await`,async t=>{
    const {path}=await sessionFixture(t);
    const options={resourceLimits:{outputStepBytes:1}},running=read(kind,path,options);
    options.resourceLimits.outputStepBytes=65536;
    await assert.rejects(running,denied(1));
    const bundle=await read(kind,path,{resourceLimits:{outputStepBytes:65536}});
    const step=bundle.steps.find(s=>s.status==='succeeded');
    const bytes=Buffer.from(step.stepFile,'base64');
    assert.equal(digest(bytes),step.receipt.data.artifact_sha256['model.step']);
    assert.equal(step.scene.instances.length,1);
    assert.equal(step.receipt.data.geometry.solids,1);
  });
}
test('denied HTML leaves no file and malformed output policy is rejected',async t=>{
  const {path}=await sessionFixture(t),destination=join(path,'denied.html');
  await assert.rejects(exportSessionHtml(path,destination,{resourceLimits:{outputStepBytes:1}}),denied(1));
  assert.equal(await stat(destination).then(()=>true,e=>{assert.equal(e.code,'ENOENT');return false;}),false);
  await assert.rejects(readSessionBundle(path,{resourceLimits:{outputStepBytes:0}}),e=>e.code==='INVALID_RESOURCE_LIMITS');
});

test('session output permission never disables the accepted STEP digest',async t=>{
  const {path,session}=await sessionFixture(t),ledger=await session.read();
  const file=join(path,ledger.steps[0].artifactDirectory,'model.step');
  const bytes=await readFile(file);bytes[bytes.length-1]^=1;await writeFile(file,bytes);
  await assert.rejects(readSessionBundle(path,{resourceLimits:{outputStepBytes:65536}}),/SESSION_INTEGRITY|digest|integrity|sha256/i);
});

test('artifact reader does not load an assembly report above the old 2MiB stdout boundary',async t=>{
  const path=await root(t),cad=client(path),built=accepted(await cad.build(model()));
  const original=await readFile(join(built.artifact_directory,'model.step'));
  const report=JSON.parse(await readFile(join(built.artifact_directory,'report.json'),'utf8'));
  report.public_test_padding='x'.repeat(2*MiB+1);
  await writeFile(join(built.artifact_directory,'report.json'),JSON.stringify(report));
  assert.deepEqual(await reader(cad,built.artifact_directory),original);
  // 删除整个报告仍能校验固定摘要，证明reader不依赖报告大小或内容。
  await rm(join(built.artifact_directory,'report.json'));
  assert.deepEqual(await reader(cad,built.artifact_directory),original);
});

test('fixed STEP digest sidecar refuses overlong or corrupt metadata',async t=>{
  const path=await root(t),cad=client(path),built=accepted(await cad.build(model()));
  const sidecar=join(built.artifact_directory,'model.step.sha256');
  const canonical=await readFile(sidecar,'utf8');
  assert.match(canonical,/^[a-f0-9]{64}\n$/);
  await writeFile(sidecar,canonical+'x');
  await assert.rejects(reader(cad,built.artifact_directory),e=>e.code==='RESOURCE_LIMIT'&&e.details.resource==='outputBytes'&&e.details.limit_bytes===65);
  await writeFile(sidecar,'0'.repeat(64)+'\n');
  await assert.rejects(reader(cad,built.artifact_directory),/digest|integrity|sha256/i);
});

test('trusted receipt SHA reads legacy output without a report or sidecar and keeps the output budget',async t=>{
  const path=await root(t),cad=client(path),built=accepted(await cad.build(model()));
  const bytes=await readFile(join(built.artifact_directory,'model.step'));
  await rm(join(built.artifact_directory,'report.json'));
  await rm(join(built.artifact_directory,'model.step.sha256'),{force:true});
  assert.equal(typeof sdk.readStepArtifact,'function');
  assert.deepEqual(await sdk.readStepArtifact(cad,built.artifact_directory,{sha256:digest(bytes)}),bytes);
  await assert.rejects(sdk.readStepArtifact(cad,built.artifact_directory,{sha256:'0'.repeat(64)}),/digest|integrity|sha256/i);
  const strict=client(path,{outputStepBytes:1});
  await assert.rejects(sdk.readStepArtifact(strict,built.artifact_directory,{sha256:digest(bytes)}),denied(1));
});
