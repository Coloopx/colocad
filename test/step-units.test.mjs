/**
 * [INPUT]: 公开SDK、显式body上下文的合成STEP与宿主配置native。
 * [OUTPUT]: 多context inspect→官方template build的公开消费回归。
 * [POS]: B4 SDK入口验收，不读取任何私有硬件或请求模型。
 * [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
 */
import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,readFile,rm,mkdir,writeFile} from 'node:fs/promises';
import {join} from 'node:path';
import {tmpdir} from 'node:os';
import {CadClient,attachStep} from '../kernel/dist/index.js';

test('same-length distinct contexts replay through the public shared-definition template',async t=>{
 const root=await mkdtemp(join(tmpdir(),'colocad-unit-context-'));
 t.after(()=>rm(root,{recursive:true,force:true}));
 const cad=new CadClient({binary:process.env.AGENT_CAD_BINARY,outputRoot:root,wallMs:120000});
 const step=await readFile(new URL('../crates/agent-cad/tests/fixtures/unit-context-box.step',import.meta.url),'utf8');
 const model=attachStep({version:1,units:'mm',parameters:{},features:[]},'fixture',step);
 const inspected=await cad.inspectStep(model,'fixture');
 assert.equal(inspected.ok,true,JSON.stringify(inspected));
 assert.equal(inspected.data.definitions,1);assert.equal(inspected.data.instances.length,1);
 const built=await cad.build({...inspected.data.model_template,assets:model.assets},{queryable:false});
 assert.equal(built.ok,true,JSON.stringify(built));
 assert.equal(built.data.geometry.solids,1);
 assert.ok(Math.abs(built.data.geometry.volume_mm3-24)<1e-6);
 const actual=built.data.geometry.bbox_mm,want=[[0,0,0],[2,3,4]];
 for(let i=0;i<2;i++)for(let j=0;j<3;j++)assert.ok(Math.abs(actual[i][j]-want[i][j])<1e-6);
});

// ---- 维护者公开夹具：同一几何，仅有三条角度元数据记录之差 ----
for (const metadata of [false,true]) test(`review8 metadata=${metadata} preserves drilled block`,async t=>{
 const root=await mkdtemp(join(tmpdir(),'colocad-review8-'));
 t.after(()=>rm(root,{recursive:true,force:true}));
 const cad=new CadClient({binary:process.env.AGENT_CAD_BINARY,outputRoot:root,wallMs:120000});
 const original=await readFile(new URL('../crates/agent-cad/tests/fixtures/metadata-2d.step',import.meta.url),'utf8');
 const step=metadata?original:original.replace(/^#900[123]=.*;\n/gm,'');
 const name=metadata?'metadata-2d':'metadata-removed';
 const evidence=process.env.COLOCAD_UNIT_EVIDENCE_DIR;
 const save=async (suffix,value)=>{if(evidence){await mkdir(evidence,{recursive:true});await writeFile(join(evidence,name+suffix),value);}};
 await save('.step',step);
 const model=attachStep({version:1,units:'mm',parameters:{},features:[]},'fixture',step);
 const inspected=await cad.inspectStep(model,'fixture');
 await save('-inspect.json',JSON.stringify(inspected,null,2));
 assert.equal(inspected.ok,true,JSON.stringify(inspected));
 assert.equal(inspected.data.definitions,1);assert.equal(inspected.data.instances.length,1);
 const built=await cad.build({...inspected.data.model_template,assets:model.assets},{queryable:false});
 await save('-build.json',JSON.stringify(built,null,2));
 assert.equal(built.ok,true,JSON.stringify(built));
 const geometry=built.data.geometry;
 assert.equal(geometry.solids,1);
 assert.ok(Math.abs(geometry.volume_mm3-(3200-40*Math.PI))<1e-4);
 for(let i=0;i<3;i++)assert.ok(Math.abs(geometry.bbox_mm[1][i]-geometry.bbox_mm[0][i]-[20,16,10][i])<1e-5);
 const output=await readFile(join(built.data.artifact_directory,'model.step'));
 await save('-output.step',output);
});

test('review8 unknown associations to metadata retain the length gate',async t=>{
 const root=await mkdtemp(join(tmpdir(),'colocad-review8-negative-'));
 t.after(()=>rm(root,{recursive:true,force:true}));
 const cad=new CadClient({binary:process.env.AGENT_CAD_BINARY,outputRoot:root,wallMs:120000});
 const original=await readFile(new URL('../crates/agent-cad/tests/fixtures/metadata-2d.step',import.meta.url),'utf8');
 for(const id of [9001,9002,9003]){
  const step=original.replace('ENDSEC;\nEND-ISO-10303-21;',`#9004=UNRECOGNIZED_GEOMETRY(#${id});\nENDSEC;\nEND-ISO-10303-21;`);
  const model=attachStep({version:1,units:'mm',parameters:{},features:[]},'fixture',step);
  const inspected=await cad.inspectStep(model,'fixture');
  if(process.env.COLOCAD_UNIT_EVIDENCE_DIR){
   await mkdir(process.env.COLOCAD_UNIT_EVIDENCE_DIR,{recursive:true});
   await writeFile(join(process.env.COLOCAD_UNIT_EVIDENCE_DIR,`sdk-negative-association-${id}.step`),step);
   await writeFile(join(process.env.COLOCAD_UNIT_EVIDENCE_DIR,`sdk-negative-association-${id}.json`),JSON.stringify(inspected,null,2));
  }
  assert.equal(inspected.ok,false,`associated entity #${id}: ${JSON.stringify(inspected)}`);
  assert.equal(inspected.error.code,'IMPORT_INCOMPLETE');
 }
});
