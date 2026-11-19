/**
 * [INPUT]: 公开Colocad SDK、本轮原生内核和可重复生成的UV缩放STEP。
 * [OUTPUT]: 单体/刚体装配及官方模板STEP回读回归。
 * [POS]: 公共消费入口校验；几何与距离容差不变。
 * [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
 */
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile,mkdtemp,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {CadClient,attachStep} from '@coloopx/colocad/kernel';
const base={version:1,units:'mm',parameters:{},features:[]};
const fixture=new URL('../crates/agent-cad/tests/fixtures/uv-rescaled-plate.step',import.meta.url);
for(const assembly of [false,true])test(`surface metric survives public SDK ${assembly?'placed assembly':'part'} and STEP readback`,async t=>{
 const root=await mkdtemp(join(tmpdir(),'colocad-uv-metric-'));t.after(()=>rm(root,{recursive:true,force:true}));
 const cad=new CadClient({outputRoot:root,wallMs:120000});
 const model=attachStep({...base,features:[{id:'plate',op:'import_step',asset:'source',assembly:true}]},'source',await readFile(fixture,'utf8'));
 if(assembly)model.assembly=[{id:'mounted',input:'plate',translation:[5,-3,2]}];else model.output='plate';
 const built=await cad.build(model,{queryable:false});assert.equal(built.ok,true,JSON.stringify(built));
 assert.equal(built.data.validation.valid,true);assert.equal(built.data.geometry.solids,1);
 assert.equal(built.data.kernel.tolerances,'kernel_defaults');
 assert.ok(built.data.kernel.upstream_fixes.includes('uv_trim_checks_on_surface'));
 const expected=2400-6*Math.PI;
 assert.ok(Math.abs(built.data.geometry.volume_mm3-expected)<expected*1e-8);
 const expectedBounds=assembly?[[5,-3,2],[25,17,8]]:[[0,0,0],[20,20,6]];
 built.data.geometry.bbox_mm.flat().forEach((v,i)=>assert.ok(Math.abs(v-expectedBounds.flat()[i])<1e-5));
 const source=attachStep(base,'reopen',await readFile(join(built.data.artifact_directory,'model.step'),'utf8'));
 const inspection=await cad.inspectStep(source,'reopen');assert.equal(inspection.ok,true,JSON.stringify(inspection));
 const reopened=await cad.build({...inspection.data.model_template,assets:source.assets},{queryable:false});
 assert.equal(reopened.ok,true,JSON.stringify(reopened));assert.equal(reopened.data.validation.valid,true);
 assert.equal(reopened.data.geometry.solids,1);
 reopened.data.geometry.bbox_mm.flat().forEach((v,i)=>assert.ok(Math.abs(v-built.data.geometry.bbox_mm.flat()[i])<1e-5));
});

test('UV-rescaled terminal geometry survives a real checkpoint hit with an intermediate fallback',async t=>{
 const root=await mkdtemp(join(tmpdir(),'colocad-uv-cache-'));t.after(()=>rm(root,{recursive:true,force:true}));
 const cad=new CadClient({outputRoot:root,wallMs:120000});
 const model=attachStep({...base,features:[
  {id:'plate',op:'import_step',asset:'source',assembly:true},
  {id:'moved',op:'translate',input:'plate',offset:[3,0,0]}],output:'moved'},'source',await readFile(fixture,'utf8'));
 const first=await cad.build(model,{queryable:false});assert.equal(first.ok,true,JSON.stringify(first));
 const second=await cad.build(model,{queryable:false});assert.equal(second.ok,true,JSON.stringify(second));
 assert.equal(second.data.features[1].id,'moved');assert.equal(second.data.features[1].cache_hit,true);
 assert.equal(second.data.geometry.solids,first.data.geometry.solids);
 assert.ok(Math.abs(second.data.geometry.volume_mm3-first.data.geometry.volume_mm3)<1e-6);
 second.data.geometry.bbox_mm.flat().forEach((v,i)=>assert.ok(Math.abs(v-first.data.geometry.bbox_mm.flat()[i])<1e-7));
});

test('UV-rescaled terminal import itself is reused and validated after checkpoint restore',async t=>{
 const root=await mkdtemp(join(tmpdir(),'colocad-uv-terminal-cache-'));t.after(()=>rm(root,{recursive:true,force:true}));
 const cad=new CadClient({outputRoot:root,wallMs:120000});
 const model=attachStep({...base,features:[{id:'plate',op:'import_step',asset:'source',assembly:true}],output:'plate'},'source',await readFile(fixture,'utf8'));
 const first=await cad.build(model,{queryable:false});assert.equal(first.ok,true,JSON.stringify(first));
 const next=await cad.build(model,{queryable:false});assert.equal(next.ok,true,JSON.stringify(next));
 assert.equal(next.data.features[0].cache_hit,true);assert.equal(next.data.validation.valid,true);
 assert.equal(next.data.geometry.solids,first.data.geometry.solids);
 assert.ok(Math.abs(next.data.geometry.volume_mm3-first.data.geometry.volume_mm3)<1e-6);
});
