import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp,rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { kk60, stages, specification } from '../examples/hiwin-kk60/model.mjs';
import { CadClient } from '../src/index.mjs';
test('K1/K2: catalog actuator is a complete product with named native parts', async t=>{
 const root=await mkdtemp(join(tmpdir(),'kk60-'));t.after(()=>rm(root,{recursive:true,force:true}));
 const cad=new CadClient({outputRoot:root,wallMs:120000});const m=kk60();
 assert.ok(m.features.length<=128);assert.ok(m.assembly.length>=60&&m.assembly.length<=128);
 for(const name of ['rail','carriage','ballscrew','bearing','coupling','seal','bolt'])assert.ok(m.assembly.some(x=>x.id.includes(name)),name);
 for(const s of stages){const r=await cad.validate(kk60(s.id));assert.equal(r.ok,true,s.id+JSON.stringify(r));}
 assert.equal(specification.stroke,210);assert.equal(specification.length,370);
 const r=await cad.build(kk60('rail-holes'));assert.equal(r.ok,true,JSON.stringify(r));
 assert.equal(r.data.geometry.solids,1);const [lo,hi]=r.data.geometry.bbox_mm;
 assert.ok(Math.abs(hi[0]-lo[0]-60)<1e-6);assert.ok(Math.abs(hi[2]-lo[2]-300)<1e-6);
});
test('K4: stroke endpoints remain editable and within the catalog guide envelope',async t=>{
 const root=await mkdtemp(join(tmpdir(),'kk60-motion-'));t.after(()=>rm(root,{recursive:true,force:true}));
 const cad=new CadClient({outputRoot:root,wallMs:120000});
 for(const travel of [-105,105]){const r=await cad.build(kk60('carriage-installed',{travel}));assert.equal(r.ok,true,JSON.stringify(r));const part=r.data.geometry;assert.equal(part.solids,3);assert.ok(Math.abs(part.bbox_mm[0][2]-(157+travel-41))<1e-6);assert.ok(Math.abs(part.bbox_mm[1][1]-33)<1e-6);}
 assert.throws(()=>kk60('complete',{travel:106}),/travel/);
});
