/**
 * [INPUT]: 公开参考STEP、实际交付STEP和独占报告路径；锁定的OCCT WASM依赖。
 * [OUTPUT]: 独立有效性、实体数、体积和包围盒比较及文件/引擎SHA。
 * [POS]: 只读交付审计；不参与Colocad运行时，不证明全局材料等价或制造接受。
 * [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
 */
import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import initOC from 'replicad-opencascadejs';
import {setOC,importSTEP,measureShapeVolumeProperties} from 'replicad';

const [referencePath,deliveredPath,reportPath]=process.argv.slice(2);
assert.ok(referencePath&&deliveredPath&&reportPath,'usage: node resource-budgets-occt.mjs REFERENCE DELIVERED NEW_REPORT');
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const wasm=await readFile(fileURLToPath(import.meta.resolve('replicad-opencascadejs/wasm')));
const pkg=JSON.parse(await readFile(fileURLToPath(import.meta.resolve('replicad-opencascadejs/package.json')),'utf8'));
const oc=await initOC({wasmBinary:wasm});setOC(oc);
const report={scope:'read-only independent OCCT validity, census, volume and bounds; not global material equivalence or manufacturing',
 engine:{package:pkg.name,version:pkg.version,wasm_sha256:sha(wasm)},platform:process.platform,arch:process.arch};

async function inspect(path){
 const bytes=await readFile(resolve(path)),shape=await importSTEP(new Blob([bytes]));
 const analyzer=new oc.BRepCheck_Analyzer(shape.wrapped,true,false,false);
 const properties=measureShapeVolumeProperties(shape),box=shape.boundingBox,solids=shape.solids;
 try{
  return {sha256:sha(bytes),bytes:bytes.length,valid:analyzer.IsValid(),solids:solids.length,
   volume_mm3:properties.volume,bbox_mm:box.bounds};
 }finally{
  analyzer.delete();properties.delete();box.delete();for(const solid of solids)solid.delete();shape.delete();
 }
}
try{
 report.reference=await inspect(referencePath);report.delivered=await inspect(deliveredPath);
 const a=report.reference,b=report.delivered;
 assert.ok(a.valid&&b.valid,'both independently read shapes must be valid');
 assert.equal(a.solids,1);assert.equal(b.solids,1);
 assert.ok(Number.isFinite(a.volume_mm3)&&Number.isFinite(b.volume_mm3)&&a.volume_mm3>0&&b.volume_mm3>0);
 assert.ok(Math.abs(a.volume_mm3-b.volume_mm3)<=Math.max(1e-5,Math.abs(a.volume_mm3)*1e-8));
 assert.ok(a.bbox_mm.flat().every((v,i)=>Number.isFinite(v)&&Number.isFinite(b.bbox_mm.flat()[i])&&Math.abs(v-b.bbox_mm.flat()[i])<1e-5));
 report.status='passed';
}catch(error){report.status='failed';report.error=error.message;process.exitCode=1;}
await writeFile(resolve(reportPath),JSON.stringify(report,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({status:report.status,engine:report.engine,scope:report.scope}));
