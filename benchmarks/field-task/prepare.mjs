// Prepare neutral inputs outside the measured task. No feature graph is given to the backends.
import assert from 'node:assert/strict';
import {readFile, mkdir, copyFile} from 'node:fs/promises';
import {resolve,join} from 'node:path';
import {createHash} from 'node:crypto';
import {kk60} from '../../examples/hiwin-kk60/model.mjs';
import {CadClient} from '../../kernel/dist/index.js';
const dir=resolve(process.argv[2]);await mkdir(dir,{recursive:true});
const cad=new CadClient({outputRoot:join(dir,'source-builds')});
const evidence=JSON.parse(await readFile(new URL('../../docs/benchmarks/alpha10/kk60-readback.json',import.meta.url)));
for(const name of ['nominal','relocated','incomplete']){
  const m=kk60('carriage-holes');
  if(name==='incomplete')m.features.find(f=>f.id==='carriageMounting').positions.pop();
  if(name==='relocated'){
    m.features.push({id:'relocated',op:'place',input:m.output,translation:[7,0,11],rotation_deg:[0,0,0]});m.output='relocated';
  }
  const built=await cad.build(m);assert.ok(built.ok,JSON.stringify(built.error));
  const step=join(built.data.artifact_directory,'model.step');
  if(name==='nominal')assert.equal(createHash('sha256').update(await readFile(step)).digest('hex'),evidence.results.find(x=>x.id==='s0006').step_sha256);
  await copyFile(step,join(dir,name+'.step'));
}
