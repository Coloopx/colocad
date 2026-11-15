/**
 * [INPUT]: 明确的新原生binary、新输出目录和精确填充字节数；公开有效STEP。
 * [OUTPUT]: 默认拒绝/显式预算下inspect-build-reopen及源/二进制SHA回执。
 * [POS]: 可在cgroup内单次执行的规模审计；注释填充不代表复杂厂商CAD。
 * [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
 */
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {resolve,join} from 'node:path';
import {createHash} from 'node:crypto';
import {CadClient,attachStep} from '@coloopx/colocad/kernel';

const [nativePath,outputPath,byteValue]=process.argv.slice(2);
assert.ok(nativePath&&outputPath&&byteValue,'usage: node resource-budgets.mjs NATIVE NEW_OUTPUT BYTES');
const binary=resolve(nativePath),output=resolve(outputPath),bytes=Number(byteValue);
const digest=value=>createHash('sha256').update(value).digest('hex');
const fixture=await readFile(new URL('../../crates/agent-cad/tests/fixtures/occt-drilled-block.step',import.meta.url),'utf8');
const extra=bytes-Buffer.byteLength(fixture)-Buffer.byteLength('\n/**/\n');
assert.ok(Number.isSafeInteger(extra)&&extra>=0&&bytes<=256*1024*1024);
await mkdir(output,{recursive:false});
const step=fixture.replace('ISO-10303-21;','ISO-10303-21;\n/*'+'x'.repeat(extra)+'*/\n');
assert.equal(Buffer.byteLength(step),bytes);
await writeFile(join(output,'source.step'),step,{flag:'wx'});
const limits={inputBytes:512*1024*1024,stepBytes:256*1024*1024,totalStepBytes:256*1024*1024};
const report={scope:'public geometry padded with comments; byte channel, not vendor assembly scale',
 started_at:new Date().toISOString(),bytes,source_sha256:digest(step),fixture_sha256:digest(fixture),
 native_sha256:digest(await readFile(binary)),platform:process.platform,arch:process.arch,limits,stages:[]};
const save=()=>writeFile(join(output,'receipt.json'),JSON.stringify(report,null,2)+'\n');
try{
 assert.throws(()=>attachStep({version:1,units:'mm',parameters:{},features:[]},'source',step),e=>e.code==='RESOURCE_LIMIT');
 report.default_registration='rejected';
 const cad=new CadClient({binary,outputRoot:join(output,'native'),wallMs:120000,resourceLimits:limits});
 const model=attachStep({version:1,units:'mm',parameters:{},features:[]},'source',step,{maxBytes:limits.stepBytes});
 const call=async(name,operation)=>{
  const at=Date.now(),r=await operation();
  report.stages.push({stage:name,elapsed_ms:Date.now()-at,receipt:r});await save();
  assert.equal(r.ok,true,JSON.stringify(r));return r;
 };
 const inspection=await call('inspect',()=>cad.inspectStep(model,'source'));
 const built=await call('build',()=>cad.build({...inspection.data.model_template,assets:model.assets},{queryable:false}));
 const original=attachStep({version:1,units:'mm',parameters:{},features:[]},'reopen',
  await readFile(join(built.data.artifact_directory,'model.step'),'utf8'));
 const inspection2=await call('reinspect',()=>cad.inspectStep(original,'reopen'));
 const reopened=await call('rebuild',()=>cad.build({...inspection2.data.model_template,assets:original.assets},{queryable:false}));
 const a=built.data.geometry,b=reopened.data.geometry;
 assert.equal(a.solids,b.solids);
 assert.ok(Math.abs(a.volume_mm3-b.volume_mm3)<=Math.max(1e-5,Math.abs(a.volume_mm3)*1e-8));
 assert.ok(a.bbox_mm.flat().every((v,i)=>Math.abs(v-b.bbox_mm.flat()[i])<1e-5));
 report.status='passed';report.geometry=a;report.output_step=join(built.data.artifact_directory,'model.step');
}catch(error){report.status='failed';report.error={message:error.message,code:error.code};process.exitCode=1;}
report.finished_at=new Date().toISOString();await save();
console.log(JSON.stringify({status:report.status,bytes,stages:report.stages.map(s=>({stage:s.stage,ok:s.receipt.ok,elapsed_ms:s.elapsed_ms})),native_sha256:report.native_sha256}));
