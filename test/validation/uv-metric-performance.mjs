/**
 * [INPUT]: 三份明确指定的release原生程序、公开钻孔块及公开原生CLI。
 * [OUTPUT]: 每次新进程的实际回执/STEP、预热后中位耗时和Level-1分项。
 * [POS]: PR3评审性能对照；无checkpoint/preview，不以耗时代替几何验收。
 * [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
 */
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {readFile,writeFile,mkdir,mkdtemp,readdir} from 'node:fs/promises';
import {join} from 'node:path';
import {tmpdir} from 'node:os';
import {performance} from 'node:perf_hooks';
import {attachStep} from '@coloopx/colocad/kernel';

const output=process.env.UV_METRIC_PERF_OUT??await mkdtemp(join(tmpdir(),'uv-metric-perf-'));
await mkdir(output,{recursive:true});
const digest=bytes=>createHash('sha256').update(bytes).digest('hex');
const save=(name,value)=>writeFile(join(output,name),JSON.stringify(value,null,2)+'\n',{flag:'wx'});
const source=await readFile(new URL('../../crates/agent-cad/tests/fixtures/occt-drilled-block.step',import.meta.url),'utf8');
const comment='/*'+ 'p'.repeat(1024*1024)+'*/\n';
const padded=source.replace('ISO-10303-21;','ISO-10303-21;\n'+comment);
assert.notEqual(padded,source);
const base={version:1,units:'mm',parameters:{},features:[],assembly:[]};

function imports(count,text){
 const model=attachStep(structuredClone(base),'source',text);
 for(let i=0;i<count;i++){
  model.features.push({id:'block-'+i,op:'import_step',asset:'source',definition:true,body_index:0});
  model.assembly.push({id:'part-'+i,input:'block-'+i,translation:[32*i,0,0]});
 }
 return model;
}
function ordinaryAssembly(){
 const model=imports(5,source);
 for(let i=5;i<10;i++){
  model.features.push({id:'pin-'+i,op:'cylinder',origin:[0,0,0],radius:5,height:10});
  model.assembly.push({id:'part-'+i,input:'pin-'+i,translation:[32*i+5,8,0]});
 }
 return model;
}
const cases=[{id:'drilled-32',model:imports(32,padded),solids:32,volume:32*(3200-40*Math.PI),
 bbox:[[0,0,0],[1012,16,10]],cache:{parse_calls:1,hits:31}},
 {id:'ordinary-10',model:ordinaryAssembly(),solids:10,volume:16000+1050*Math.PI,
 bbox:[[0,0,0],[298,16,10]],cache:{parse_calls:1,hits:4}}];
const variants=[['off',process.env.UV_METRIC_OFF_BINARY],['before',process.env.UV_METRIC_BEFORE_BINARY],
 ['after',process.env.UV_METRIC_AFTER_BINARY]];
for(const [name,binary]of variants)assert(binary,`missing ${name} binary`);
const records=[];

function verify(result,test){
 assert.equal(result.ok,true,JSON.stringify(result));
 assert.equal(result.data.validation.valid,true);assert.equal(result.data.geometry.solids,test.solids);
 const geometry=result.data.geometry;
 assert(Math.abs(geometry.volume_mm3-test.volume)<test.volume*1e-8);
 assert(geometry.bbox_mm.flat().every((v,i)=>Math.abs(v-test.bbox.flat()[i])<1e-6));
 assert.equal(result.data.step_source_cache.parse_calls,test.cache.parse_calls);
 assert.equal(result.data.step_source_cache.hits,test.cache.hits);
 assert.equal(result.data.assembly.instances.length,test.solids);
}
async function sample(test,variant,binary,index){
 const name=`${test.id}-${variant}-${index}`,root=join(output,name);
 // ---- SDK build默认写checkpoint；此对照显式使用不带缓存/预览标志的公开CLI ----
 const argv=['build','--out',root],start=performance.now();
 const child=spawnSync(binary,argv,{input:JSON.stringify(test.model),encoding:'utf8',timeout:120000,maxBuffer:8*1024*1024});
 const wall=performance.now()-start;
 await save(name+'-process.json',{argv,exit:child.status,signal:child.signal,stderr:child.stderr,error:child.error?.message});
 await writeFile(join(output,name+'-stdout.json'),child.stdout??'',{flag:'wx'});
 assert.equal(child.status,0,child.stderr||child.stdout||String(child.error));
 const result=JSON.parse(child.stdout);await save(name+'.json',result);verify(result,test);
 const files=await readdir(root);
 assert(!files.some(n=>/checkpoint|snapshot|preview|cache/i.test(n)),JSON.stringify(files));
 const step=await readFile(join(result.data.artifact_directory,'model.step'));
 assert.equal(digest(step),result.data.artifact_sha256['model.step']);
 return {case:test.id,variant,index,warmup:index===0,wall_ms:wall,timing_ms:result.data.timing_ms,
  solves:result.data.validation_property_solves,cache:result.data.step_source_cache,argv,files,
  step_sha256:digest(step),STEP_path:join(result.data.artifact_directory,'model.step')};
}
for(const test of cases){
 await save(test.id+'-input.json',test.model);
 // 交错顺序，减少固定先后次序与宿主漂移对比值的影响。
 for(let index=0;index<4;index++){
  const sequence=index%2?[...variants].reverse():variants;
  for(const [variant,binary]of sequence){
   records.push(await sample(test,variant,binary,index));
   await writeFile(join(output,'progress.json'),JSON.stringify(records,null,2)+'\n');
  }
 }
}
const median=values=>[...values].sort((a,b)=>a-b)[Math.floor(values.length/2)];
const summaries=[];
for(const test of cases)for(const [variant,binary]of variants){
 const rows=records.filter(r=>r.case===test.id&&r.variant===variant&&!r.warmup);
 summaries.push({case:test.id,variant,samples:rows.length,binary_sha256:digest(await readFile(binary)),
  wall_ms:median(rows.map(r=>r.wall_ms)),
  total_ms:median(rows.map(r=>r.timing_ms.total)),
  level1_and_manifold_ms:median(rows.map(r=>r.timing_ms.level1_and_manifold)),
  geometry_ms:median(rows.map(r=>r.timing_ms.geometry)),
  certified:rows.map(r=>r.solves.certified_regular_curves),
  geometry_checks:'valid/census/literal bbox/volume/actual STEP SHA; not independent equivalence'});
}
await save('summary.json',{schema:'uv-metric-performance.v1',platform:process.platform,arch:process.arch,
 samples:records,summaries,model_calls:0,HTTP_calls:0,checkpoint:false,preview:false,
 geometry_fixture_sha256:digest(Buffer.from(source)),padded_fixture_sha256:digest(Buffer.from(padded))});
console.log(JSON.stringify(summaries));
