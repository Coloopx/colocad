import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,readFile,writeFile,rm,access} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {build} from '../src/index.mjs';
import {readMesh} from '../src/hybrid-mesh.mjs';
import {NodeIO} from '@gltf-transform/core';

const root=fileURLToPath(new URL('../examples/hybrid-enclosure/',import.meta.url));
const spec=()=>readFile(join(root,'model.json'),'utf8').then(JSON.parse);
async function workspace(t){const dir=await mkdtemp(join(tmpdir(),'colocad-hybrid-'));t.after(()=>rm(dir,{recursive:true,force:true}));return dir;}
test('real loft wall, cubic tool, shell opening, closed offset, articulated clearances and a penetrating control',async t=>{
  const dir=await workspace(t), model=await spec();
  model.features.push({id:'closed',op:'hollow',input:'outer',thickness:1.6});
  for(const degrees of [0,15,30]) {
    const jaw=model.instances.find(x=>x.id==='jaw');
    jaw.rotate.degrees=degrees;
    const r=await build(model,join(dir,'pose'+degrees),{assetRoot:root});
    assert.equal(r.ok,true);assert.deepEqual(r.checks.map(c=>c.overlapMm3),[0,0]);
    assert.ok(r.features.find(x=>x.id==='closed').volumeMm3>r.features.find(x=>x.id==='housing').volumeMm3);
    assert.ok(Math.abs(r.inspections[0].spansMm[0]-1.6)<.005);assert.equal(r.inspections[1].contours,2);
    assert.equal(r.assemblyStep,'assembly.step');
  }
  model.instances.find(x=>x.id==='lens').translate=[0,0,19.5];
  const r=await build(model,join(dir,'interference'),{assetRoot:root});
  assert.equal(r.ok,false);assert.ok(r.checks[1].overlapMm3>17);
  const before=await readFile(join(dir,'interference/report.json'),'utf8');
  assert.equal(JSON.parse(before).ok,false);
});
test('STL and multicolour GLB round trip, mesh CSG, exact import and faceted B-rep conversion',async t=>{
  const dir=await workspace(t),out=join(dir,'base');
  await build(await spec(),out,{assetRoot:root});
  const glb=await readFile(join(out,'assembly.glb')), doc=await new NodeIO().readBinary(glb);
  assert.equal(doc.getRoot().listMeshes().length,3);
  const material=doc.getRoot().listMeshes()[2].listPrimitives()[0].getMaterial();assert.equal(material.getBaseColorFactor()[3],.8);
  const chunks=await readMesh(glb,'glb');
  assert.equal(chunks[2].color,'#339e76');
  const z=chunks[2].positions.filter((v,i)=>i%3===2);assert.ok(Math.abs(Math.min(...z)-5)<1e-4);
  const s={format:'colocad/hybrid/1',units:'mm',features:[
    {id:'stl',op:'import_mesh',path:'lens.stl',units:'mm'},
    {id:'cutter',op:'box',min:[0,-5,0],max:[5,5,12]},
    {id:'half',op:'mesh_cut',input:'stl',tool:'cutter'},
    {id:'facets',op:'faceted_brep',input:'half'},
    {id:'copy',op:'import_step',path:'exact.step'},
    {id:'all',op:'import_mesh',path:'assembly.glb'}
  ],instances:[{id:'half',input:'facets'}]};
  const r=await build(s,join(dir,'roundtrip'),{assetRoot:out});
  assert.equal(r.ok,true);
  for(const id of ['half','facets']) assert.ok(Math.abs(r.features.find(x=>x.id===id).volumeMm3-24)<1e-5);
  assert.ok(r.features.find(x=>x.id==='copy').volumeMm3>2500);
});
test('open surfaces, missing units, escaping paths and unmatched shell selectors cannot pass',async t=>{
  const dir=await workspace(t),out=join(dir,'rejected');
  await writeFile(join(dir,'open.obj'),'v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n');
  const model={format:'colocad/hybrid/1',units:'mm',features:[{op:'import_mesh',id:'open',path:'open.obj',units:'mm'}],instances:[{id:'open',input:'open'}]};
  await assert.rejects(build(model,out,{assetRoot:dir}),/Feature open/);
  delete model.features[0].units;await assert.rejects(build(model,out,{assetRoot:dir}),/units/);
  model.features[0].path='../';await assert.rejects(build(model,out,{assetRoot:dir}),/outside project/);
  const shell=await spec();shell.features[1].openFace.at=100;await assert.rejects(build(shell,out,{assetRoot:root}),/matched 0/);
  await assert.rejects(access(out));
});
