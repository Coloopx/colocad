import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {CadClient} from '../kernel/dist/index.js';

test('SDK solves opposed shaft/support mates, retains rotation and rejects overlap',async t=>{
 const root=await mkdtemp(join(tmpdir(),'colocad-mates-'));
 t.after(()=>rm(root,{recursive:true,force:true}));
 const cad=new CadClient({outputRoot:root});
 const capabilities=await cad.capabilities();assert.equal(capabilities.ok,true);
 assert.deepEqual(capabilities.data.assembly.mate_direction.axis,['aligned','opposed','unoriented']);
 const model=JSON.parse(await readFile(new URL('../specs/experiments/0004/shaft-support.json',import.meta.url),'utf8'));
 const result=await cad.build(model);assert.equal(result.ok,true,JSON.stringify(result));
 const solution=result.data.assembly.mate_solution;
 assert.equal(solution.degrees_of_freedom,1);assert.ok(solution.iterations>0);
 for(const row of solution.constraints){assert.ok(row.physical_residual.linear_mm<1e-6);assert.ok(row.physical_residual.angular_rad<1e-6);}
 const origin=solution.placements.shaft.origin_mm;
 assert.ok(Math.abs(origin[0])<1e-6);assert.ok(Math.abs(origin[1])<1e-6);assert.ok(Math.abs(origin[2]+8)<1e-6);
 assert.match(await readFile(join(result.data.artifact_directory,'model.step'),'utf8'),/NEXT_ASSEMBLY_USAGE_OCCURRENCE/);
 const oversized=await cad.edit(model,{shaft_radius:3.2});assert.equal(oversized.ok,false);
 assert.equal(oversized.error.code,'ASSEMBLY_COLLISION');
});
