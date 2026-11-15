/**
 * [INPUT]: 公开SDK、明确宿主预算和公开小STEP的纯注释扩展。
 * [OUTPUT]: 默认拒绝/显式预算接受、UTF-8/聚合/原生回执及摘要守卫回归。
 * [POS]: 测试字节合同，不以填充样本替代真实复杂几何验收。
 * [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
 */
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdtemp,rm} from 'node:fs/promises';
import {join} from 'node:path';
import {tmpdir} from 'node:os';
import {createHash} from 'node:crypto';
import {CadClient,attachStep,readModelFile,writeModelFile,DEFAULT_RESOURCE_LIMITS,RESOURCE_LIMIT_CEILINGS} from '@coloopx/colocad/kernel';

const MiB=1024*1024,base={version:1,units:'mm',parameters:{},features:[]};
const fixture=await readFile(new URL('../crates/agent-cad/tests/fixtures/occt-drilled-block.step',import.meta.url),'utf8');
const padded=fixture.replace('ISO-10303-21;','ISO-10303-21;\n/*'+'x'.repeat(33*MiB)+'*/');
const limits={inputBytes:64*MiB,stepBytes:40*MiB,totalStepBytes:48*MiB};
async function client(t,resourceLimits){
 const root=await mkdtemp(join(tmpdir(),'colocad-byte-limits-'));t.after(()=>rm(root,{recursive:true,force:true}));
 return new CadClient({outputRoot:root,wallMs:120000,resourceLimits});
}

test('STEP registration supports an explicit bounded override while preserving the default',()=>{
 assert.throws(()=>attachStep(base,'source',padded),/24 MiB/);
 assert.doesNotThrow(()=>attachStep(base,'source',padded,{maxBytes:40*MiB}));
});

test('registration uses UTF-8 bytes at the exact configured boundary',()=>{
 assert.doesNotThrow(()=>attachStep(base,'source','日'.repeat(4),{maxBytes:12}));
 assert.throws(()=>attachStep(base,'source','日'.repeat(5),{maxBytes:12}),e=>e.code==='RESOURCE_LIMIT'&&e.details.limit_bytes===12&&e.details.observed_bytes===15);
});

test('unsafe and incoherent host limits are rejected before spawning',()=>{
 for(const resourceLimits of [{inputBytes:Infinity},{stepBytes:0},{inputBytes:1.5},{inputBytes:513*MiB},
  {inputBytes:64*MiB,stepBytes:48*MiB,totalStepBytes:24*MiB}])
  assert.throws(()=>new CadClient({outputRoot:'/unused',resourceLimits}),e=>e.code==='INVALID_RESOURCE_LIMITS');
});

test('trusted file codecs preserve native aggregate-limit details',async t=>{
 const root=await mkdtemp(join(tmpdir(),'colocad-codec-error-'));t.after(()=>rm(root,{recursive:true,force:true}));
 const model={...base,features:[{id:'box',op:'box',origin:[0,0,0],size:[1,1,1]}],output:'box',assets:{}};
 for(const [id,step] of [['a','ab'],['b','cd']])model.assets[id]={step,sha256:createHash('sha256').update(step).digest('hex')};
 const file=join(root,'model.json');await writeFile(file,JSON.stringify(model));
 const cad=await client(t,{inputBytes:4096,stepBytes:3,totalStepBytes:3});
 await assert.rejects(()=>readModelFile(cad,file),e=>e.code==='RESOURCE_LIMIT'&&e.details?.resource==='totalStepBytes'
  &&e.details.limit_bytes===3&&e.details.observed_bytes===4&&!e.details.observed_is_lower_bound);
});

test('public SDK and actual native defaults and ceilings remain consistent',async t=>{
 const report=await (await client(t)).capabilities();assert.equal(report.ok,true,JSON.stringify(report));
 assert.deepEqual(report.data.resource_limits,DEFAULT_RESOURCE_LIMITS);
 assert.deepEqual(report.data.resource_limit_ceilings,RESOURCE_LIMIT_CEILINGS);
});

test('the same valid STEP is refused by default native inspection and accepted with host limits',async t=>{
 const model={...base,assets:{source:{step:padded,sha256:createHash('sha256').update(padded).digest('hex')}}};
 const ordinary=await client(t);
 const before=await ordinary.inspectStep(model,'source');assert.equal(before.ok,false);assert.equal(before.error.code,'RESOURCE_LIMIT');
 assert.equal(before.error.details.resource,'inputBytes');
 const cad=await client(t,limits),after=await cad.inspectStep(model,'source');
 assert.equal(after.ok,true,JSON.stringify(after));assert.equal(after.data.instances.length,1);
 assert.deepEqual(after.data.resource_limits,limits);
 assert.equal(after.data.resource_usage.input_bytes,Buffer.byteLength(JSON.stringify(model.assets.source)));
 assert.equal(after.data.resource_usage.step_bytes,Buffer.byteLength(padded));
 const built=await cad.build({...after.data.model_template,assets:model.assets},{queryable:false});
 assert.equal(built.ok,true,JSON.stringify(built));assert.equal(built.data.validation.valid,true);
 assert.deepEqual(built.data.resource_limits,limits);
 const strict=await client(t,{...limits,stepBytes:1024});
 const replay=await strict.build({...after.data.model_template,assets:model.assets},{queryable:false,previousRevision:built.data.artifact_directory});
 assert.equal(replay.ok,false);assert.equal(replay.error.code,'RESOURCE_LIMIT');assert.equal(replay.error.details.resource,'stepBytes');
});

test('the host configuration accessor is read once for validation and native forwarding',async t=>{
 const root=await mkdtemp(join(tmpdir(),'colocad-host-accessor-'));t.after(()=>rm(root,{recursive:true,force:true}));
 const expected={inputBytes:4096,stepBytes:1,totalStepBytes:3};let reads=0;
 const cad=new CadClient({outputRoot:root,get resourceLimits(){return ++reads===1?expected:undefined;}});
 assert.equal(reads,1,'validation and explicit forwarding must use the same configuration snapshot');
 const report=await cad.capabilities();assert.equal(report.ok,true,JSON.stringify(report));
 assert.deepEqual(report.data.resource_limits,expected);
});

test('active host limits are snapshotted independently of later caller mutation',async t=>{
 const config={...limits},cad=await client(t,config);config.stepBytes=1;
 const report=await cad.capabilities();assert.equal(report.ok,true,JSON.stringify(report));
 assert.deepEqual(report.data.resource_limits,limits);
});

test('trusted STEP and model file codecs use the same explicit client policy',async t=>{
 const root=await mkdtemp(join(tmpdir(),'colocad-large-files-'));t.after(()=>rm(root,{recursive:true,force:true}));
 const file=join(root,'foreign.step');await writeFile(file,padded);
 const cad=await client(t,limits),model=await readModelFile(cad,file);
 assert.equal(model.assets.source_part.sha256,createHash('sha256').update(padded).digest('hex'));
 const saved=join(root,'model.json');await writeModelFile(cad,model,saved);
 const opened=await readModelFile(cad,saved);
 assert.equal(opened.assets.source_part.sha256,model.assets.source_part.sha256);
 await assert.rejects(()=>readModelFile(new CadClient({outputRoot:join(root,'default')}),file),
  e=>e.code==='RESOURCE_LIMIT'&&e.details.resource==='inputBytes');
});

test('aggregate and escaped JSON bytes remain separate enforced limits',async t=>{
 const first='x'.repeat(14*MiB),second='y'.repeat(14*MiB);
 const model={...base,features:[{id:'box',op:'box',origin:[0,0,0],size:[1,1,1]}],output:'box',assets:{}};
 for(const [id,step] of [['a',first],['b',second]])model.assets[id]={step,sha256:createHash('sha256').update(step).digest('hex')};
 const aggregate=await (await client(t)).validate(model);
 assert.equal(aggregate.ok,false);assert.equal(aggregate.error.code,'RESOURCE_LIMIT');assert.equal(aggregate.error.details.resource,'totalStepBytes');
 const escaped='\n'.repeat(17*MiB),encoded={...model,assets:{a:{step:escaped,sha256:createHash('sha256').update(escaped).digest('hex')}}};
 const transport=await (await client(t)).validate(encoded);
 assert.equal(transport.ok,false);assert.equal(transport.error.code,'RESOURCE_LIMIT');assert.equal(transport.error.details.resource,'inputBytes');
});

test('larger asset permission never disables the original SHA check',async t=>{
 const model={...base,features:[{id:'box',op:'box',origin:[0,0,0],size:[1,1,1]}],output:'box',
  assets:{source:{step:padded,sha256:'0'.repeat(64)}}};
 const r=await (await client(t,limits)).validate(model);
 assert.equal(r.ok,false);assert.equal(r.error.code,'ASSET_DIGEST_MISMATCH');
});
