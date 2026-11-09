import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,rm,readFile,writeFile,cp} from 'node:fs/promises';
import {join} from 'node:path';
import {tmpdir} from 'node:os';
import {spawnSync} from 'node:child_process';
import {CadClient,createCadTools,attachStep} from '../kernel/dist/index.js';
import {createHash} from 'node:crypto';
import {ModelingSession,readSessionBundle} from '../src/session.mjs';
const model={version:1,units:'mm',parameters:{height:30},features:[{id:'stock',op:'box',origin:[0,0,0],size:[10,20,{param:'height'}]}],output:'stock'};
const request=revision=>({schema:'colocad.query/1',query_revision_id:revision,target:{definition_id:'stock'},space:'definition',tolerance:{linear_mm:1e-5,angular_rad:1e-7},queries:[{id:'top',op:'faces',where:{planar_normal_aligned_with:[0,0,1]},limit:64}]});
async function setup(t){const root=await mkdtemp(join(tmpdir(),'colocad-query-sdk-'));t.after(()=>rm(root,{recursive:true,force:true}));return root;}

test('independent OCCT STEP preserves analytic top face and five inner-circle centers',async t=>{
 const root=await setup(t),cad=new CadClient({outputRoot:root});
 const base=new URL('../specs/experiments/0004/fixtures/',import.meta.url);
 const step=await readFile(new URL('occt-flange.step',base),'utf8');
 const oracle=JSON.parse(await readFile(new URL('occt-flange.json',base),'utf8'));
 assert.equal(createHash('sha256').update(step).digest('hex'),oracle.sha256);
 const source=attachStep({version:1,units:'mm',parameters:{},features:[{id:'stock',op:'import_step',asset:'foreign',body_index:0}],output:'stock'},'foreign',step);
 const built=await cad.build(source,{queryable:true});assert.equal(built.ok,true,JSON.stringify(built));
 assert.ok(Math.abs(built.data.geometry.volume_mm3-oracle.expected_volume_mm3)<1e-5);
 const q=request(built.data.query_revision_id);
 const top={kind:'expect',count:1,input:{kind:'filter',input:{kind:'entities',entity:'face'},where:{planar_normal_aligned_with:[0,0,1]}}};
 const circles={kind:'filter',where:{curve_type:'circle'},input:{kind:'children',entity:'edge',input:{kind:'filter',where:{loop_role:'inner'},input:{kind:'children',entity:'loop',input:top}}}};
 q.queries.push({id:'centers',op:'datums',selection:{kind:'expect',count:5,input:circles},derive:{kind:'point'}});
 const receipt=await cad.query(built.data.artifact_directory,q);assert.equal(receipt.ok,true,JSON.stringify(receipt));
 assert.ok(Math.abs(receipt.data.results[0].items[0].area_mm2-oracle.expected_top_area_mm2)<1e-5);
 const actual=receipt.data.results[1].items.map(x=>x.geometry.point_mm);
 for(const [x,y]of oracle.holes_xy_radius)assert.ok(actual.some(p=>Math.hypot(p[0]-x,p[1]-y,p[2]-8)<1e-5));
 assert.equal(receipt.data.execution.geometry_build_calls,0);assert.equal(receipt.data.execution.step_export_calls,0);
});

test('SDK reopens exact snapshot, exposes schema/tool, preserves refs and confines host paths',async t=>{
 const root=await setup(t),cad=new CadClient({outputRoot:join(root,'jobs')});
 const built=await cad.build(model,{queryable:true});assert.equal(built.ok,true,JSON.stringify(built));
 const reopened=new CadClient({outputRoot:root});const q=request(built.data.query_revision_id);
 const r=await reopened.query(built.data.artifact_directory,q);assert.equal(r.ok,true,JSON.stringify(r));
 assert.equal(r.data.results[0].total_count,1);assert.equal(r.data.results[0].items[0].center_mm[2],30);
 assert.deepEqual(r.data.results[0].items[0].normal,[0,0,1]);assert.equal(r.data.execution.geometry_build_calls,0);
 const tools=await createCadTools(reopened);assert.ok(tools.find(t=>t.name==='cad_query')?.inputSchema.$defs);
 const called=await tools.find(t=>t.name==='cad_query').execute({artifact_directory:built.data.artifact_directory,request:q});assert.deepEqual(called.data.results,r.data.results);
 const moved=join(root,'moved');await cp(built.data.artifact_directory,moved,{recursive:true});assert.deepEqual((await reopened.query(moved,q)).data.results,r.data.results);
 const restricted=new CadClient({outputRoot:built.data.artifact_directory});assert.equal((await restricted.query(moved,q)).error.code,'QUERY_ARTIFACT_OUTSIDE_ROOT');
 const edited=await cad.edit(model,{height:40});assert.equal(edited.ok,true);assert.ok(edited.data.query_revision_id);
 const stale=await cad.query(edited.data.artifact_directory,q);assert.equal(stale.error.code,'QUERY_REVISION_MISMATCH');
 const fresh=request(edited.data.query_revision_id);fresh.queries=[{id:'stale_ref',op:'describe',refs:[r.data.results[0].items[0].ref]}];
 const bad=await cad.query(edited.data.artifact_directory,fresh);assert.equal(bad.error.code,'QUERY_REVISION_MISMATCH');assert.equal(bad.error.query_id,'stale_ref');
});

test('query receipts persist without replacing accepted source and edits retain queryability',async t=>{
 const root=await setup(t);let session=await ModelingSession.create(join(root,'session'));
 const built=await session.build(model,{queryable:true});assert.equal(built.ok,true,JSON.stringify(built));
 const original=await session.source(),q=request(built.data.query_revision_id);
 assert.equal((await session.query(q)).ok,true);assert.deepEqual(await session.source(),original);
 session=await ModelingSession.open(session.root);assert.equal((await session.query(q)).ok,true);
 const edited=await session.edit({height:40});assert.equal(edited.ok,true);assert.ok(edited.data.query_revision_id);
 const stale=await session.query(q);assert.equal(stale.ok,false);assert.equal(stale.error.code,'QUERY_REVISION_MISMATCH');
 assert.equal((await session.source()).parameters.height,40);
 const ledger=await session.read();assert.equal(ledger.steps.filter(s=>s.operation==='query').length,3);assert.equal(ledger.steps.at(-1).status,'failed');
 const bundle=await readSessionBundle(session.root);assert.ok(bundle.steps[1].queryRequest);assert.ok(bundle.steps[1].scene);assert.ok(bundle.steps[3].stepFile);assert.equal(bundle.steps[1].stepFile,undefined);
});

test('CLI query emits a receipt, refuses overwrite and leaves source artifacts unchanged',async t=>{
 const root=await setup(t),cad=new CadClient({outputRoot:root});const built=await cad.build(model,{queryable:true});assert.equal(built.ok,true);
 const input=join(root,'request.json'),out=join(root,'receipt.json');await writeFile(input,JSON.stringify(request(built.data.query_revision_id)));
 const manifest=join(built.data.artifact_directory,'query-manifest.json'),before=await readFile(manifest,'utf8');
 const args=['bin/colocad.mjs','query',built.data.artifact_directory,input,'--out',out];
 const good=spawnSync(process.execPath,args,{encoding:'utf8'});assert.equal(good.status,0,good.stderr);assert.equal(JSON.parse(good.stdout).ok,true);assert.equal(JSON.parse(await readFile(out,'utf8')).ok,true);
 const duplicate=spawnSync(process.execPath,args,{encoding:'utf8'});assert.notEqual(duplicate.status,0);assert.equal(JSON.parse(duplicate.stderr).ok,false);
 assert.equal(await readFile(manifest,'utf8'),before);
});

test('named datums auto-enable exact artifacts, survive reopen and transform per occurrence',async t=>{
 const root=await setup(t),cad=new CadClient({outputRoot:root});
 const m=structuredClone(model);delete m.output;
 m.assembly=[{id:'placed',input:'stock',translation:[12,3,4],rotation_deg:[90,0,0]}];
 m.datums=[{id:'top',definition:'stock',tolerance:{linear_mm:1e-5,angular_rad:1e-7},derive:{kind:'selected',derive:{kind:'plane'},selection:{kind:'filter',input:{kind:'entities',entity:'face'},where:{planar_normal_aligned_with:[0,0,1]}}}}];
 const disabled=await cad.build(m,{queryable:false});assert.equal(disabled.error.code,'QUERY_SNAPSHOT_REQUIRED');
 const b=await cad.build(m);assert.equal(b.ok,true,JSON.stringify(b));assert.ok(b.data.query_revision_id);
 const fresh=new CadClient({outputRoot:root});const q=request(b.data.query_revision_id);q.target={instance_path:['placed']};q.space='assembly';q.queries=[{id:'datum',op:'named_datums',ids:['top']}];
 const r=await fresh.query(b.data.artifact_directory,q);assert.equal(r.ok,true,JSON.stringify(r));
 const datum=r.data.results[0].items[0];assert.equal(datum.kind,'plane');assert.equal(datum.space,'assembly');
 assert.ok(Math.abs(datum.geometry.point_mm[1]+27)<1e-5);assert.ok(Math.abs(datum.geometry.normal[1]+1)<1e-7);
 assert.deepEqual(datum.source_refs[0].instance_path,['placed']);
 q.queries[0].ids=['missing'];const bad=await fresh.query(b.data.artifact_directory,q);assert.equal(bad.error.code,'DATUM_REFERENCE');assert.equal(bad.error.query_id,'datum');
 const session=await ModelingSession.create(join(root,'session'));const initial=await session.build(m);assert.equal(initial.ok,true);assert.equal((await session.read()).steps[0].queryable,true);
 const edited=await session.edit({height:40});assert.equal(edited.ok,true);assert.notEqual(initial.data.query_revision_id,edited.data.query_revision_id);
});
