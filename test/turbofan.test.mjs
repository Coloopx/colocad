import test from 'node:test';
import assert from 'node:assert/strict';
import { turbofan, stages } from '../examples/turbofan/model.mjs';
import { CadClient } from '../src/index.mjs';
import { mkdtemp,rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

test('E1/E2: real geometry milestones lead to a shared-definition turbofan assembly', async t=>{
  const root=await mkdtemp(join(tmpdir(),'turbofan-'));t.after(()=>rm(root,{recursive:true,force:true}));
  const cad=new CadClient({outputRoot:root,wallMs:120000});
  assert.ok(stages.length>=12);
  const model=turbofan();
  assert.ok(model.assembly.length>=80 && model.assembly.length<=128);
  assert.ok(new Set(model.assembly.map(x=>x.input)).size<model.assembly.length/2);
  for(const part of ['fan','compressor','combustor','turbine','shaft','nacelle','nozzle'])assert.ok(model.assembly.some(x=>x.id.includes(part)),part);
  for(const stage of stages){const r=await cad.validate(turbofan(stage.id));assert.equal(r.ok,true,stage.id+JSON.stringify(r));}
  const ops=new Set(model.features.map(x=>x.op));for(const op of ['revolve','profile_loft','cut'])assert.ok(ops.has(op));
});

test('E3: editable tip radius changes exact engine bounds', async t=>{
  const root=await mkdtemp(join(tmpdir(),'turbofan-edit-'));t.after(()=>rm(root,{recursive:true,force:true}));
  const cad=new CadClient({outputRoot:root,wallMs:120000});
  const a=await cad.build(turbofan('fan-row',{fanRadius:140}));
  const b=await cad.build(turbofan('fan-row',{fanRadius:150}));
  assert.equal(a.ok,true,JSON.stringify(a));assert.equal(b.ok,true,JSON.stringify(b));
  assert.equal(a.data.geometry.solids,b.data.geometry.solids);
  assert.ok(b.data.geometry.bbox_mm[1][0]-a.data.geometry.bbox_mm[1][0]>8);
});
