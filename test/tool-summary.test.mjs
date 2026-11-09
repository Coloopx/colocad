import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,rm,readFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {ModelingSession} from '../src/session.mjs';
import {createCompactSessionTools,compactReceipt} from '../examples/integrations/compact-tools.mjs';

test('compact tools keep executable schemas, exact measurements and recovery state',async t=>{
 const root=await mkdtemp(join(tmpdir(),'compact-cad-'));t.after(()=>rm(root,{recursive:true,force:true}));
 const session=await ModelingSession.create(root);const tools=await createCompactSessionTools(session);const call=(name,args)=>tools.find(t=>t.name===name).execute(args);
 assert.equal(tools.length,5);assert.ok(tools.find(x=>x.name==='cad_build').inputSchema.$defs);
 const model={version:1,units:'mm',parameters:{length:20},features:[{id:'stock',op:'box',origin:[0,0,0],size:[{param:'length'},10,6]}],output:'stock'};
 const built=await call('cad_build',{model});assert.equal(built.ok,true);assert.equal(built.data.geometry.volume_mm3,1200);assert.equal(built.session_step,'s0001');assert.ok(built.data.artifacts['model.step']);assert.equal(built.data.feature_summary.count,1);assert.equal(built.data.features,undefined);
 const failed=await call('cad_edit',{parameters:{unknown:1}});assert.equal(failed.ok,false);assert.equal(failed.error.code,'UNKNOWN_PARAMETER');assert.equal(failed.session_step,'s0002');
 const edited=await call('cad_edit',{parameters:{length:25}});assert.equal(edited.data.geometry.volume_mm3,1500);assert.equal((await session.source()).parameters.length,25);
 const ledger=await session.read();const full=JSON.parse(await readFile(join(root,ledger.steps[0].receiptFile)));assert.ok(Array.isArray(full.data.features));
});
test('response compaction never erases degradation, errors or capability limits',()=>{
 const receipt={ok:true,data:{features:[{id:'round',elapsed_ms:10,cache_hit:false,degradation:{error:{code:'KERNEL_ERROR'},omitted:['fillet'],affects:['appearance'],reason:'explicitly optional'}}],geometry:{solids:1},artifact_directory:'/trusted/result',artifact_sha256:{'model.step':'abc'},timing_ms:{total:20}}};
 const result=compactReceipt(receipt);assert.equal(result.data.feature_summary.degradations[0].id,'round');assert.deepEqual(result.data.feature_summary.degradations[0].omitted,['fillet']);assert.equal(result.data.feature_summary.degradations[0].error.code,'KERNEL_ERROR');assert.ok(receipt.data.features);
 const capabilities={ok:true,data:{limits:{features:128},unsupported:['example']}};assert.deepEqual(compactReceipt(capabilities),capabilities);
 const error={ok:false,error:{code:'BAD',feature_id:'hole',message:'outside stock'}};assert.deepEqual(compactReceipt(error),error);
});

test('guided workflow performs real calls, quantitative edits, failure recovery and CLI export',async t=>{
 const {runGuidedWorkflow}=await import('../examples/integrations/guided-workflow.mjs');
 const root=await mkdtemp(join(tmpdir(),'guided-cad-'));t.after(()=>rm(root,{recursive:true,force:true}));
 const result=await runGuidedWorkflow(root);assert.equal(result.passed,true);assert.equal(result.steps,5);assert.equal(result.measurements.finalLengthMm,100);
 assert.equal(result.calls.filter(c=>!c.result.ok).length,1);assert.match(await readFile(result.html,'utf8'),/colocad-viewer/);
});


test('compact build retains query revision and execution policy needed by a subsequent geometry query',()=>{
 const r=compactReceipt({ok:true,data:{features:[],artifact_directory:'/trusted/part',query_revision_id:'sha256:revision',execution_policy:'sha256:policy',geometry:{solids:1}}});
 assert.equal(r.data.query_revision_id,'sha256:revision');assert.equal(r.data.execution_policy,'sha256:policy');
});
