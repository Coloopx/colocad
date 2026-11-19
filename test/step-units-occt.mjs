/**
 * [INPUT]: step_units生成的合成STEP及锁定OCCT WASM；独占报告路径。
 * [OUTPUT]: 合成及公开OCCT嵌套输入/交付与手算常量的独立断言，不匹配实体序号。
 * [POS]: B4同长度上下文验收；raw mapped输入仅诊断，不冒称两引擎位姿一致。
 * [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
 */
import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
import {resolve,join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import initOC from 'replicad-opencascadejs';
import {setOC,importSTEP,measureShapeVolumeProperties} from 'replicad';

const [directory,output,names]=process.argv.slice(2);
assert.ok(directory&&output,'usage: node step-units-occt.mjs UNIT_EVIDENCE_DIR NEW_REPORT.json [CASE_NAMES_COMMA_SEPARATED]');
const sha=value=>createHash('sha256').update(value).digest('hex');
const wasm=await readFile(fileURLToPath(import.meta.resolve('replicad-opencascadejs/wasm')));
const pkg=JSON.parse(await readFile(fileURLToPath(import.meta.resolve('replicad-opencascadejs/package.json')),'utf8'));
const oc=await initOC({wasmBinary:wasm});setOC(oc);
// ---- 期望来自手算常量，不读取Colocad报告作为期望 ----
const expected={
 'metadata-2d':[1,3200-40*Math.PI,[[0,0,0],[20,16,10]]],
 'metadata-removed':[1,3200-40*Math.PI,[[0,0,0],[20,16,10]]],
 'same-length-angle':[1,24,[[0,0,0],[2,3,4]]],
 'same-length-uncertainty':[2,48,[[0,0,0],[2,3,4]]],
 'uniform-cm':[1,24000,[[0,0,0],[20,30,40]]],
 'nested-same-length':[2,48,[[-31,44,0],[-29,97,4]]],
 'occt-nested-uncertainty':[2,2000,[[80,0,0],[100,30,5]]],
 'occt-nested-readback':[2,2000,[[80,0,0],[100,30,5]]],
};
async function properties(path){
 const bytes=await readFile(path),shape=await importSTEP(new Blob([bytes]));
 const analyzer=new oc.BRepCheck_Analyzer(shape.wrapped,true,false,false);
 const props=measureShapeVolumeProperties(shape),box=shape.boundingBox,solids=shape.solids;
 try{return {sha256:sha(bytes),valid:analyzer.IsValid(),solids:solids.length,volume_mm3:props.volume,bbox_mm:box.bounds};}
 finally{analyzer.delete();props.delete();box.delete();for(const solid of solids)solid.delete();shape.delete();}
}
function matches(actual,[solids,volume,bounds]){
 return actual.valid&&actual.solids===solids&&Number.isFinite(actual.volume_mm3)
  &&Math.abs(actual.volume_mm3-volume)<1e-4
  &&bounds.flat().every((v,i)=>Number.isFinite(actual.bbox_mm.flat()[i])&&Math.abs(v-actual.bbox_mm.flat()[i])<1e-5);
}
const report={scope:'independent read-only OCCT transfer and literal exported-geometry expectations; raw mapped source diagnostic only',
 engine:{package:pkg.name,version:pkg.version,wasm_sha256:sha(wasm)},platform:process.platform,arch:process.arch,
 no_solid_index_correspondence_assumed:true,cases:[]};
const selected=names?names.split(','):Object.keys(expected);
assert.ok(selected.length&&selected.every(name=>Object.hasOwn(expected,name)),'unknown audit case');
for(const [name,wanted] of Object.entries(expected).filter(([name])=>selected.includes(name))){
 const entry={name,expected:{solids:wanted[0],volume_mm3:wanted[1],bbox_mm:wanted[2]},raw_source_expected_asserted:name!=='nested-same-length'};
 try{
  entry.source=await properties(join(resolve(directory),name+'.step'));
  entry.output=await properties(join(resolve(directory),name+'-output.step'));
  entry.passed=entry.source.valid&&matches(entry.output,wanted)&&(!entry.raw_source_expected_asserted||matches(entry.source,wanted));
 }catch(error){entry.passed=false;entry.error=error.message;}
 report.cases.push(entry);
}
report.passed=report.cases.every(x=>x.passed);
await writeFile(resolve(output),JSON.stringify(report,null,2)+'\n',{flag:'wx'});
if(!report.passed)process.exitCode=1;
console.log(JSON.stringify({passed:report.passed,cases:report.cases.map(x=>({name:x.name,passed:x.passed})),engine:report.engine}));
