import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,rm,readFile,writeFile,cp,rename,chmod} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {spawn} from 'node:child_process';
import {once} from 'node:events';
import {CadClient} from '../kernel/dist/index.js';
import {resolveBinary} from '../kernel/dist/binary.js';
import {ModelingSession} from '../src/session.mjs';
import {model as installation} from '../examples/engineering-interfaces/model.mjs';
const tolerance={linear_mm:1e-5,angular_rad:1e-7};
const request=(built,definition,queries)=>({schema:'colocad.query/1',query_revision_id:built.data.query_revision_id,target:{definition_id:definition},space:'definition',tolerance,queries});
async function temp(t){const p=await mkdtemp(join(tmpdir(),'colocad-lifecycle-'));t.after(()=>rm(p,{recursive:true,force:true}));return p;}
const box={version:1,units:'mm',parameters:{height:10},features:[{id:'stock',op:'box',origin:[0,0,0],size:[10,10,{param:'height'}]}],output:'stock'};
test('semantic named-face lifecycle: split, merge, deletion and old-ref rejection',async t=>{
 const root=await temp(t),cad=new CadClient({outputRoot:root});
 const base={...structuredClone(box),features:[...box.features,{id:'named',op:'name_topology',input:'stock',faces:{left:{axis:'x',value:0,tolerance:1e-5,expected_count:1}}}],output:'named'};
 const a=await cad.build(base,{queryable:true});assert.equal(a.ok,true,JSON.stringify(a));
 const selection={kind:'filter',input:{kind:'entities',entity:'face'},where:{name:'named/left'}};
 const observe=async(b,id)=>cad.query(b.data.artifact_directory,request(b,id,[{id:'named',op:'select',selection}]));
 const ar=await observe(a,'named');assert.equal(ar.ok,true,JSON.stringify(ar));assert.equal(ar.data.results[0].total_count,1);const old=ar.data.results[0].items[0].ref;
 const split={...base,features:[...base.features,{id:'tool',op:'box',origin:[-1,-1,4],size:[12,12,2]},{id:'split',op:'cut',left:'named',right:'tool'}],output:'split'};
 const b=await cad.build(split,{queryable:true});assert.equal(b.ok,true,JSON.stringify(b));const br=await observe(b,'split');assert.equal(br.data.results[0].total_count,2);
 const ambiguity=await cad.query(b.data.artifact_directory,request(b,'split',[{id:'unique',op:'select',selection:{kind:'expect',count:1,input:selection}}]));assert.equal(ambiguity.error.code,'SELECTION_CARDINALITY');
 const stale=await cad.query(b.data.artifact_directory,request(b,'split',[{id:'old',op:'describe',refs:[old]}]));assert.equal(stale.error.code,'QUERY_REVISION_MISMATCH');
 const merged={...split,features:[...split.features,{id:'fill',op:'box',origin:[0,0,4],size:[10,10,2]},{id:'merged',op:'union',left:'split',right:'fill'}],output:'merged'};
 const c=await cad.build(merged,{queryable:true});assert.equal(c.ok,true,JSON.stringify(c));const cr=await observe(c,'merged');assert.equal(cr.ok,true,JSON.stringify(cr));assert.equal(cr.data.results[0].total_count,1);
 const deleted={...base,features:[...base.features,{id:'tool',op:'box',origin:[-1,-1,-1],size:[6,12,12]},{id:'deleted',op:'cut',left:'named',right:'tool'}],output:'deleted'};
 const d=await cad.build(deleted,{queryable:true});assert.equal(d.ok,true,JSON.stringify(d));const dr=await observe(d,'deleted');assert.equal(dr.ok,true,JSON.stringify(dr));assert.equal(dr.data.results[0].total_count,0);
});
test('foreign assembly replacement and rejected replacement retain accepted session across relocation',async t=>{
 const root=await temp(t),fixtures=new URL('../specs/experiments/0004/fixtures/engineering/',import.meta.url),support=await readFile(new URL('support.step',fixtures),'utf8');
 let s=await ModelingSession.create(join(root,'session'));let previous;
 for(const id of ['motor','motor-housing','motor-rotated']){
  const m=installation(await readFile(new URL(id+'.step',fixtures),'utf8'),support);m.preview={width:160,height:120,chord_mm:1};const b=await s.build(m);assert.equal(b.ok,true,JSON.stringify(b));if(previous)assert.notEqual(b.data.query_revision_id,previous);previous=b.data.query_revision_id;
  const q=request(b,'motor',[{id:'mount',op:'interfaces',ids:['mount']}]);q.target={instance_path:['motorPart']};q.space='assembly';const r=await s.query(q);assert.equal(r.ok,true,JSON.stringify(r));assert.equal(r.data.results[0].items[0].orientation.source,'geometry_derived_named_datum');
 }
 const accepted=await s.source(),last=await s.latestGeometry();
 for(const [id,code] of [['motor-wrong-pitch','INTERFACE_INCOMPATIBLE'],['motor-missing-hole','INTERFACE_NOT_FOUND']]){
  const m=installation(await readFile(new URL(id+'.step',fixtures),'utf8'),support);m.preview={width:160,height:120,chord_mm:1};const r=await s.build(m);assert.equal(r.error.code,code);assert.deepEqual(await s.source(),accepted);assert.equal((await s.latestGeometry()).id,last.id);
 }
 await rename(s.root,join(root,'moved'));s=await ModelingSession.open(join(root,'moved'));assert.deepEqual(await s.source(),accepted);
 const q={schema:'colocad.query/1',query_revision_id:previous,target:{instance_path:['motorPart']},space:'assembly',tolerance,queries:[{id:'mount',op:'interfaces',ids:['mount']}]};assert.equal((await s.query(q)).ok,true);
});
test('checkpoint execution policy rejects an old or missing policy instead of reusing old feature semantics',async t=>{
 const root=await temp(t),cad=new CadClient({outputRoot:root});const b=await cad.build(box);assert.equal(b.ok,true);assert.equal(typeof b.data.execution_policy,'string');
 const same=await cad.build(box);assert.equal(same.ok,true);assert.equal(same.data.features[0].cache_hit,true);
 const reportPath=join(same.data.artifact_directory,'report.json'),report=JSON.parse(await readFile(reportPath));delete report.execution_policy;await writeFile(reportPath,JSON.stringify(report));
 const c=await cad.build(box);assert.equal(c.ok,true);assert.equal(c.data.features[0].cache_hit,false);
});
test('real terminated build leaves interrupted history, releases dead lock and resumes accepted geometry',async t=>{
 const root=await temp(t),s=await ModelingSession.create(join(root,'session'));const first=await s.build(box,{queryable:true});assert.equal(first.ok,true);
 const marker=join(root,'started'),wrapper=join(root,'native-probe.sh');await writeFile(wrapper,'#!/bin/sh\nprintf started > "$COLOCAD_PROBE_MARKER"\nexec "$COLOCAD_PROBE_NATIVE" "$@"\n');await chmod(wrapper,0o700);
 const module=new URL('../src/session.mjs',import.meta.url).href,model=new URL('../specs/experiments/0005/corpus-v1/product-kk60-accessory.json',import.meta.url).href;
 const program=`import {readFile} from 'node:fs/promises';import {ModelingSession} from ${JSON.stringify(module)};const s=await ModelingSession.open(process.env.COLOCAD_PROBE_SESSION,{binary:process.env.COLOCAD_PROBE_WRAPPER,wallMs:120000});const m=JSON.parse(await readFile(new URL(${JSON.stringify(model)}),'utf8'));m.preview={width:160,height:120,chord_mm:1};await s.build(m);`;
 const child=spawn(process.execPath,['--input-type=module','-e',program],{detached:true,stdio:['ignore','ignore','pipe'],env:{...process.env,COLOCAD_PROBE_SESSION:s.root,COLOCAD_PROBE_WRAPPER:wrapper,COLOCAD_PROBE_MARKER:marker,COLOCAD_PROBE_NATIVE:resolveBinary()}});let errors='';child.stderr.on('data',b=>errors+=b);
 t.after(()=>{try{process.kill(-child.pid,'SIGKILL');}catch{}});const exited=once(child,'exit');const deadline=Date.now()+10000;
 for(;;){try{await readFile(marker);break;}catch{}assert.equal(child.exitCode,null,errors);assert.ok(Date.now()<deadline,'native did not start');await new Promise(r=>setTimeout(r,20));}
 await new Promise(r=>setTimeout(r,100));assert.equal((await s.read()).steps.at(-1).status,'running');process.kill(-child.pid,'SIGKILL');await exited;
 const recovered=await ModelingSession.open(s.root);assert.equal((await recovered.read()).steps.at(-1).status,'interrupted');assert.equal((await recovered.source()).parameters.height,10);
 const edited=await recovered.edit({height:12});assert.equal(edited.ok,true,JSON.stringify(edited));assert.equal((await recovered.source()).parameters.height,12);assert.deepEqual((await recovered.read()).steps.map(x=>x.status),['succeeded','interrupted','succeeded']);
});
