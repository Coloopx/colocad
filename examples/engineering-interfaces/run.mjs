import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {resolve,join,relative} from 'node:path';
import {CadClient} from '../../kernel/dist/index.js';
import {model,tolerance} from './model.mjs';
const out=resolve(process.argv[2]??'out/engineering-interfaces');await mkdir(out,{recursive:true});
const root=new URL('../../specs/experiments/0004/fixtures/engineering/',import.meta.url);
const manifest=JSON.parse(await readFile(new URL('manifest.json',root),'utf8'));
async function source(id){const row=manifest.cases.find(x=>x.id===id),step=await readFile(new URL(row.file,root),'utf8');assert.equal(createHash('sha256').update(step).digest('hex'),row.sha256);return {step,row};}
const support=await source('support'),cad=new CadClient({outputRoot:join(out,'builds')}),runs=[];
for(const id of ['motor','motor-housing','motor-rotated','motor-wrong-pitch','motor-missing-hole']){
 const part=await source(id),m=model(part.step,support.step);await writeFile(join(out,id+'-model.json'),JSON.stringify(m,null,2)+'\n');
 const built=await cad.build(m);runs.push({id,sourceSha256:part.row.sha256,expectedVolumesMm3:[part.row.volume_mm3,support.row.volume_mm3],built});
 await writeFile(join(out,'runs.json'),JSON.stringify({manifest,runs},null,2)+'\n');
 if(!part.row.expected.positive){assert.equal(built.ok,false,JSON.stringify(built));assert.equal(built.error.code,part.row.expected.reject);console.log(JSON.stringify({id,expectedRejection:built.error.code}));continue;}
 assert.equal(built.ok,true,JSON.stringify(built));assert.equal(built.data.assembly.checks[0].passed,true);
 const request={schema:'colocad.query/1',query_revision_id:built.data.query_revision_id,target:{instance_path:['motorPart']},space:'assembly',tolerance,queries:[{id:'interface',op:'interfaces',ids:['mount']},{id:'mount_relation',op:'measure',a:{instance_path:['motorPart']},b:{instance_path:['supportPart']}}]};
 const result=await cad.query(built.data.artifact_directory,request);assert.equal(result.ok,true,JSON.stringify(result));runs.at(-1).query=result;
 assert.equal(result.data.results[0].items[0].orientation.source,'geometry_derived_named_datum');assert.equal(result.data.execution.geometry_build_calls,0);
 runs.at(-1).artifactDirectory=relative(out,built.data.artifact_directory);runs.at(-1).stepSha256=createHash('sha256').update(await readFile(join(built.data.artifact_directory,'model.step'))).digest('hex');
 console.log(JSON.stringify({id,passed:true,relationship:built.data.assembly.checks[1].samples[0].relationship,artifact:built.data.artifact_directory}));
}
await writeFile(join(out,'runs.json'),JSON.stringify({manifest,runs},null,2)+'\n');
