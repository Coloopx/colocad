/**
 * [INPUT]: Public SDK/native host, generated exact box STEP and independent mapped/product occurrence fixtures.
 * [OUTPUT]: External STEP count budgets, local-definition isolation, structural diagnostics and replay regressions.
 * [POS]: STEP host-policy acceptance; real CAD stays separate from read-only private-source inspection.
 * [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
 */
import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,readFile} from 'node:fs/promises';
import {join} from 'node:path';
import {tmpdir} from 'node:os';
import {CadClient,attachStep} from '../kernel/dist/index.js';
const base={version:1,units:'mm',parameters:{},features:[]};
const stock={...base,features:[{id:'stock',op:'box',origin:[0,0,0],size:[2,2,2]}],output:'stock'};
const client=async limits=>new CadClient({outputRoot:await mkdtemp(join(tmpdir(),'step-policy-')),stepImportLimits:limits});
const asset=step=>attachStep(base,'source',step);
function limited(receipt,resource,actual,allowed){
 assert.equal(receipt.ok,false,JSON.stringify(receipt));assert.equal(receipt.error.code,'RESOURCE_LIMIT');
 const detail=receipt.error.details.resource_limit;
 assert.equal(detail.resource,resource);assert.equal(detail.actual,actual);assert.equal(detail.allowed,allowed);
 assert.equal(typeof detail.stage,'string');
}
let fixtures;
async function source(){
 if(fixtures)return fixtures;
 const cad=await client();const built=await cad.build({...stock,output:undefined,assembly:[{id:'part',input:'stock'}]});
 assert.equal(built.ok,true,JSON.stringify(built));
 const step=await readFile(join(built.data.artifact_directory,'model.step'),'utf8');
 fixtures={step,cad};return fixtures;
}
function mapped(step,count){
 const rep=step.match(/#(\d+)=ADVANCED_BREP_SHAPE_REPRESENTATION\([^\n]+,#(\d+)\);/);
 assert.ok(rep);const repId=rep[1],context=rep[2];
 let next=1000000,rows=[],ids=[];
 const add=s=>{const id=next++;rows.push('#'+id+'='+s+';');return id;};
 const origin=add("CARTESIAN_POINT('',(0.,0.,0.))"),axis=add("AXIS2_PLACEMENT_3D('',#"+origin+',$,$)');
 const map=add('REPRESENTATION_MAP(#'+axis+',#'+repId+')');
 for(let i=0;i<count;i++){
  const point=add("CARTESIAN_POINT('',("+(i*3)+'.,0.,0.))');
  const transform=add("CARTESIAN_TRANSFORMATION_OPERATOR_3D('',$,$,#"+point+',1.,$)');
  ids.push(add("MAPPED_ITEM('',#"+map+',#'+transform+')'));
 }
 const representation=add("SHAPE_REPRESENTATION('',("+ids.map(i=>'#'+i).join(',')+'),#'+context+')');
 // Replace the solid product's representation; the existing outer occurrence remains identity.
 step=step.replace(new RegExp('(SHAPE_DEFINITION_REPRESENTATION\\(#\\d+,)#'+repId+'(\\);)'), '$1#'+representation+'$2');
 return step.replace('ENDSEC;\nEND-ISO-10303-21;',rows.join('\n')+'\nENDSEC;\nEND-ISO-10303-21;');
}
function products(step,count){
 const records=[...step.matchAll(/^#(\d+)=(.+);$/gm)],tail=records.slice(-9);
 assert.ok(tail.some(r=>r[2].startsWith('NEXT_ASSEMBLY_USAGE_OCCURRENCE(')));
 const old=new Set(tail.map(r=>Number(r[1]))),last=Number(records.at(-1)[1]);
 const rows=[];
 for(let i=1;i<count;i++)for(const r of tail){
  const shift=i*9;
  rows.push('#'+(Number(r[1])+shift)+'='+r[2].replace(/#(\d+)/g,(_s,id)=>'#'+(old.has(Number(id))?Number(id)+shift:id))+';');
 }
 assert.equal(last,Number(tail.at(-1)[1]));
 return step.replace('ENDSEC;\nEND-ISO-10303-21;',rows.join('\n')+'\nENDSEC;\nEND-ISO-10303-21;');
}

test('host entity budget applies to inspect and every import mode at N/N+1',async()=>{
 const {step}=await source(),n=[...step.matchAll(/^#\d+=/gm)].length;
 const exact=await client({maxEntities:n});assert.equal((await exact.inspectStep(asset(step),'source')).ok,true);
 const strict=await client({maxEntities:n-1});limited(await strict.inspectStep(asset(step),'source'),'step_entities',n,n-1);
 for(const mode of [{definition:true},{assembly:true},{}]){
  const model=asset(step);model.features=[{id:'imported',op:'import_step',asset:'source',...mode}];model.output='imported';
  limited(await strict.build(model),'step_entities',n,n-1);
 }
});

test('host STEP policy rejects unknown, fractional, zero and over-ceiling options before execution',()=>{
 for(const stepImportLimits of [{maxEntities:0},{maxEntities:1.5},{maxEntities:2000001},{maxOccurrences:4097},{maxDefinitions:1025},{maxGroups:8193},{surprise:1},{constructor:1},{toString:1},JSON.parse('{"__proto__":1}')])
  assert.throws(()=>new CadClient({outputRoot:'/unused',stepImportLimits}),/stepImportLimits/);
});
for(const [name,make] of [['mapped',mapped],['global product',products]])test(name+' occurrences preserve 127/128 and require explicit 129',async()=>{
 const {step,cad}=await source();
 for(const count of [127,128]){
  const inspected=await cad.inspectStep(asset(make(step,count)),'source');
  assert.equal(inspected.ok,true,JSON.stringify(inspected));assert.equal(inspected.data.instances.length,count);
 }
 const text=make(step,129);
 limited(await cad.inspectStep(asset(text),'source'),'step_occurrences',129,128);
 const extended=await client({maxOccurrences:129}),model=asset(text);
 const inspected=await extended.inspectStep(model,'source');assert.equal(inspected.ok,true,JSON.stringify(inspected));
 assert.equal(inspected.data.instances.length,129);assert.equal(inspected.data.definitions,1);
 if(name==='mapped')for(const [i,instance]of inspected.data.instances.entries())
  assert.ok(Math.abs(instance.world_placement.origin_mm[0]-i*3)<1e-5);
 else for(const instance of inspected.data.instances)assert.deepEqual(instance.world_placement.origin_mm,[0,0,0]);
 const replay={...inspected.data.model_template,assets:model.assets};
 const built=await extended.build(replay,{queryable:false});assert.equal(built.ok,true,JSON.stringify(built));
 assert.equal(built.data.geometry.solids,129);assert.ok(Math.abs(built.data.geometry.volume_mm3-1032)<1e-5);
 assert.equal(built.data.assembly.instances.length,129);
 const output=await extended.inspectStep(asset(await readFile(join(built.data.artifact_directory,'model.step'),'utf8')),'source');
 assert.equal(output.ok,true,JSON.stringify(output));assert.equal(output.data.instances.length,129);assert.equal(output.data.definitions,1);
 const coordinates=list=>list.map(i=>i.world_placement.origin_mm.map(v=>Math.round(v*1e5)).join(',')).sort();
 assert.deepEqual(coordinates(output.data.instances),coordinates(inspected.data.instances));
 limited(await extended.build(replay,{queryable:true}),'query_occurrences',129,128);
 limited(await extended.build({...replay,preview:{chord_mm:0.4}}),'preview_occurrences',129,128);
 const reopened=await cad.inspectStep(asset(await readFile(join(built.data.artifact_directory,'model.step'),'utf8')),'source');
 limited(reopened,'step_occurrences',129,128);
});

test('definitions/groups and import-only feature capacity have explicit unchanged defaults',async()=>{
 const {step}=await source();
 const oneGroup=await client({maxGroups:1});limited(await oneGroup.inspectStep(asset(step),'source'),'step_groups',2,1);
 assert.equal((await (await client({maxGroups:2})).inspectStep(asset(step),'source')).ok,true);
 const model=asset(step);model.features=Array.from({length:129},(_,i)=>({id:'d'+i,op:'import_step',asset:'source',body_index:0,definition:true}));
 model.assembly=model.features.map((f,i)=>({id:'one'+i,input:f.id}));
 limited(await (await client()).validate(model),'model_features',129,128);
 const raised=await client({maxDefinitions:129,maxOccurrences:129});const valid=await raised.validate(model);assert.equal(valid.ok,true,JSON.stringify(valid));
 limited(await raised.validate({...model,checks:[{kind:'collision',parts:model.assembly.map(i=>i.id),max_overlap_mm3:0}]}),'checked_pairs',8256,8128);
 limited(await raised.build(model,{queryable:true}),'query_occurrences',129,128);
 limited(await raised.build({...model,preview:{chord_mm:0.4}}),'preview_occurrences',129,128);
 const ordinary={...stock,features:Array.from({length:129},(_,i)=>({id:'b'+i,op:'box',origin:[0,0,0],size:[1,1,1]})),output:'b128'};
 limited(await raised.validate(ordinary),'model_features',129,128);
 const two=await (await client()).build({...base,features:[{id:'a',op:'box',origin:[0,0,0],size:[1,1,1]},
  {id:'b',op:'box',origin:[5,0,0],size:[2,2,2]}],assembly:[{id:'a',input:'a'},{id:'b',input:'b'}]});
 assert.equal(two.ok,true,JSON.stringify(two));
 const text=await readFile(join(two.data.artifact_directory,'model.step'),'utf8');
 limited(await (await client({maxDefinitions:1})).inspectStep(asset(text),'source'),'step_definitions',2,1);
 assert.equal((await (await client({maxDefinitions:2})).inspectStep(asset(text),'source')).ok,true);
});

test('a lower host policy cannot recover an import from a higher-budget checkpoint',async()=>{
 const {step}=await source(),n=[...step.matchAll(/^#\d+=/gm)].length;
 const model=asset(step);model.features=[{id:'part',op:'import_step',asset:'source',definition:true}];model.output='part';
 const first=await (await client({maxEntities:n})).build(model);assert.equal(first.ok,true,JSON.stringify(first));
 const lower=await client({maxEntities:n-1});
 limited(await lower.build(model,{previousRevision:first.data.artifact_directory}),'step_entities',n,n-1);
});

test('agent model cannot grant itself a STEP policy and constructor config is frozen',async()=>{
 const {step}=await source();
 const config={maxEntities:1};const cad=await client(config);config.maxEntities=2000000;
 limited(await cad.inspectStep(asset(step),'source'),'step_entities',2,1);
 const injected=await (await client()).validate({...stock,stepImportLimits:{maxEntities:2000000}});
 assert.equal(injected.ok,false);assert.equal(injected.error.code,'SCHEMA_ERROR');
});


for(const shape of ['prototype toJSON','non-enumerable toJSON','getter'])
 test('review I1: host policy snapshot ignores '+shape,async()=>{
  const {step}=await source();let reads=0,jsonCalls=0;
  class Policy { maxEntities=1;toJSON(){jsonCalls++;return {maxEntities:2000000};} }
  let policy=new Policy();
  if(shape==='non-enumerable toJSON'){
   policy={maxEntities:1};Object.defineProperty(policy,'toJSON',{value(){jsonCalls++;return {maxEntities:2000000};}});
  }
  if(shape==='getter'){
   policy={};Object.defineProperty(policy,'maxEntities',{enumerable:true,get(){return ++reads===1?1:2000000;}});
  }
  const cad=await client(policy),capabilities=await cad.capabilities();assert.equal(capabilities.ok,true,JSON.stringify(capabilities));
  assert.equal(capabilities.data.step_import.active_limits.maxEntities,1,'native must receive the validated value');
  limited(await cad.inspectStep(asset(step),'source'),'step_entities',2,1);
  assert.equal(jsonCalls,0,'custom serialization must never be invoked');
  if(shape==='getter')assert.equal(reads,1,'read policy accessors once, before validating the snapshot');
 });

// -- 单定义不展开无关位姿；同一原件仍约束整装配。 --
for(const [name,make] of [['mapped',mapped],['product',products]])test('review10: '+name+' local definition and same-source assembly controls',async t=>{
 const {step}=await source(),text=make(step,129),model=asset(text);
 const single={...model,features:[{id:'part',op:'import_step',asset:'source',definition:true,body_index:0}],output:'part'};
 const whole={...model,features:[{id:'part',op:'import_step',asset:'source',assembly:true}],output:'part'};
 await t.test('default single definition succeeds in local coordinates',async()=>{
  const built=await (await client()).build(single);
  assert.equal(built.ok,true,JSON.stringify(built));assert.equal(built.data.geometry.solids,1);
  assert.ok(Math.abs(built.data.geometry.volume_mm3-8)<1e-5);
  assert.deepEqual(built.data.geometry.bbox_mm,[[0,0,0],[2,2,2]]);
 });
 await t.test('default whole assembly refuses 129',async()=>{
  const cad=await client();
  limited(await cad.build(whole),'step_occurrences',129,128);
  limited(await cad.inspectStep(model,'source'),'step_occurrences',129,128);
 });
 await t.test('explicit 129 imports all solids',async()=>{
  const cad=await client({maxOccurrences:129}),inspected=await cad.inspectStep(model,'source');
  assert.equal(inspected.ok,true,JSON.stringify(inspected));
  assert.equal(inspected.data.definitions,1);assert.equal(inspected.data.instances.length,129);
  const built=await cad.build({...inspected.data.model_template,assets:model.assets},{queryable:false});
  assert.equal(built.ok,true,JSON.stringify(built));assert.equal(built.data.geometry.solids,129);
  assert.ok(Math.abs(built.data.geometry.volume_mm3-1032)<1e-5);
  const expected=name==='mapped'?[[0,0,0],[386,2,2]]:[[0,0,0],[2,2,2]];
  assert.deepEqual(built.data.geometry.bbox_mm,expected);
 });
 await t.test('single definition does not consume expanded group or occurrence budgets',async()=>{
  const built=await (await client({maxGroups:1,maxOccurrences:1})).build(single);
  assert.equal(built.ok,true,JSON.stringify(built));assert.equal(built.data.geometry.solids,1);
 });
 await t.test('invalid index and non-definition mode still refuse',async()=>{
  const bad=structuredClone(single);bad.features[0].body_index=1;
  assert.equal((await (await client()).build(bad)).error.code,'INVALID_REFERENCE');
  const ordinary=structuredClone(single);delete ordinary.features[0].definition;
  const rejected=await (await client({maxOccurrences:129})).build(ordinary);
  assert.equal(rejected.ok,false);assert.equal(rejected.error.code,'IMPORT_INCOMPLETE');
 });
});

for(const [name,make] of [['mapped',mapped],['product',products]])test('review10 cache: '+name+' adjacent local definitions admit once',async()=>{
 const {step}=await source(),text=make(step,129),model=attachStep(asset(text),'alias',text);
 model.features=[{id:'a',op:'import_step',asset:'source',definition:true,body_index:0},
  {id:'b',op:'import_step',asset:'alias',definition:true,body_index:0}];
 model.assembly=[{id:'one',input:'a'},{id:'two',input:'b',translation:[10,0,0]}];
 const cad=await client(),built=await cad.build(model);
 assert.equal(built.ok,true,JSON.stringify(built));assert.equal(built.data.geometry.solids,2);
 assert.ok(Math.abs(built.data.geometry.volume_mm3-16)<1e-5);
 assert.equal(built.data.step_source_cache.parse_calls,1);assert.equal(built.data.step_source_cache.hits,1);
 const bad=structuredClone(model);bad.features[1].body_index=1;
 assert.equal((await (await client()).build(bad)).error.code,'INVALID_REFERENCE');
 const ordinary=structuredClone(model);ordinary.features[1].definition=false;
 limited(await (await client()).build(ordinary),'step_occurrences',129,128);
});

test('review10 washer: public equivalent129 same-file local/cache/whole controls',async t=>{
 const text=await readFile(new URL('../crates/agent-cad/tests/fixtures/washer129-colocad.step',import.meta.url),'utf8');
 const model=asset(text),volume=Math.PI*(4.5**2-2.2**2)*1.5;
 const single={...model,features:[{id:'washer',op:'import_step',asset:'source',definition:true,body_index:0}],output:'washer'};
 await t.test('default local definition and adjacent cache preserve72.61791mm3',async()=>{
  const one=await (await client()).build(single);assert.equal(one.ok,true,JSON.stringify(one));
  assert.equal(one.data.geometry.solids,1);assert.ok(Math.abs(one.data.geometry.volume_mm3-volume)<1e-5);
  const two=structuredClone(single);two.features.push({...two.features[0],id:'second'});delete two.output;
  two.assembly=[{id:'a',input:'washer'},{id:'b',input:'second',translation:[12,0,0]}];
  const built=await (await client()).build(two);assert.equal(built.ok,true,JSON.stringify(built));
  assert.equal(built.data.geometry.solids,2);assert.ok(Math.abs(built.data.geometry.volume_mm3-2*volume)<1e-5);
  assert.equal(built.data.step_source_cache.parse_calls,1);assert.equal(built.data.step_source_cache.hits,1);
 });
 await t.test('default whole source refuses129',async()=>{
  const whole={...model,features:[{id:'washer',op:'import_step',asset:'source',assembly:true}],output:'washer'};
  limited(await (await client()).build(whole),'step_occurrences',129,128);
 });
 await t.test('explicit129 preserves complete washer tray',async()=>{
  const cad=await client({maxOccurrences:129}),inspected=await cad.inspectStep(model,'source');
  assert.equal(inspected.ok,true,JSON.stringify(inspected));assert.equal(inspected.data.instances.length,129);
  const all=await cad.build({...inspected.data.model_template,assets:model.assets},{queryable:false});
  assert.equal(all.ok,true,JSON.stringify(all));assert.equal(all.data.geometry.solids,129);
  assert.ok(Math.abs(all.data.geometry.volume_mm3-129*volume)<1e-4);
 });
});
