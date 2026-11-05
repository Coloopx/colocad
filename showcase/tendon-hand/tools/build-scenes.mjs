// Replay named SDK features as separate exact STEP assets for the editorial film.
import { pathToFileURL } from 'node:url';
import { readFile, writeFile, mkdir, copyFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { hand, stages } from '../model.mjs';
const out=resolve(process.argv[2]),sdk=process.argv[3];
const {CadClient}=await import(sdk?pathToFileURL(resolve(sdk)).href:'@coloopx/colocad/kernel');
await mkdir(out,{recursive:false});
const receipts={};
async function exportScene(name,model,preview=false){
  const folder=join(out,name);await mkdir(folder);
  const client=new CadClient({outputRoot:folder,wallMs:120000});
  if(preview)model.preview={width:640,height:480};
  const r=await client.build(model);if(!r.ok)throw Error(JSON.stringify(r.error));
  const dir=r.data.artifact_directory,rep=JSON.parse(await readFile(join(dir,'report.json')));
  await copyFile(join(dir,'model.step'),join(out,name+'.step'));
  receipts[name]={solids:rep.geometry.solids,faces:rep.geometry.faces,volume_mm3:rep.geometry.volume_mm3,
    target_ms:rep.features.find(f=>f.id===model.output)?.elapsed_ms,model_hash:rep.model_hash,step_sha256:rep.artifact_sha256['model.step'],timing_ms:rep.timing_ms};
  if(preview){await copyFile(join(dir,'model.glb'),join(out,name+'.glb'));await copyFile(join(dir,'preview.mesh.json'),join(out,name+'.mesh.json'));
    receipts[name].preview=Object.fromEntries(['triangles','tessellation_calls','instance_mesh_hits','revision_mesh_hits'].map(k=>[k,rep.preview[k]]));}
}
function single(id,diam){
 const {model}=hand('drive',{guideDiameter:diam});delete model.assembly;
 const keep=new Set([id]);for(const f of [...model.features].reverse())if(keep.has(f.id))for(const k of ['left','right','input'])if(f[k])keep.add(f[k]);
 model.features=model.features.filter(x=>keep.has(x.id));model.output=id;return model;
}
for(const stage of stages)await exportScene(stage,hand(stage).model,stage==='drive');
for(const f of ['ProximalBlank','ProximalTangPos','ProximalPinRoot','ProximalFork','ProximalPinEnd','ProximalCableGuideTool','ProximalCableGuide'])await exportScene(f,single(f,2.9),f==='ProximalCableGuide');
await exportScene('guide-edit',single('ProximalCableGuide',3.3));
await writeFile(join(out,'scenes.json'),JSON.stringify(receipts,null,2)+'\n');
