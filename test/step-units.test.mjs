/**
 * [INPUT]: 公开SDK、显式body上下文的合成STEP与宿主配置native。
 * [OUTPUT]: 多context inspect→官方template build的公开消费回归。
 * [POS]: B4 SDK入口验收，不读取任何私有硬件或请求模型。
 * [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
 */
import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,readFile,rm} from 'node:fs/promises';
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
