// Exact foreign STEP -> semantic datums -> mates -> persistent read-only queries.
// Development checkout example: use the native built from this source revision.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {mkdir,readFile,writeFile} from 'node:fs/promises';
import {join,resolve,relative} from 'node:path';
import {CadClient,attachStep} from '../../kernel/dist/index.js';
const out=resolve(process.argv[2]??'out/engineering-queries');
await mkdir(out,{recursive:true});
const fixture=new URL('../../specs/experiments/0004/fixtures/',import.meta.url);
const step=await readFile(new URL('occt-flange.step',fixture),'utf8');
const oracle=JSON.parse(await readFile(new URL('occt-flange.json',fixture),'utf8'));
assert.equal(createHash('sha256').update(step).digest('hex'),oracle.sha256);
const tolerance={linear_mm:1e-5,angular_rad:1e-7};
const faces=where=>({kind:'filter',input:{kind:'entities',entity:'face'},where});
const datum=(id,definition,kind,selection)=>({id,definition,tolerance,derive:{kind:'selected',selection,derive:{kind}}});
const model=attachStep({version:1,units:'mm',parameters:{shaft_radius:4.8},features:[
 {id:'flange',op:'import_step',asset:'foreign',body_index:0},
 {id:'shaft',op:'cylinder',origin:[0,0,0],radius:{param:'shaft_radius'},height:20}
],datums:[
 datum('bore','flange','axis',faces({surface_type:'cylinder',radius_mm:{min:4.99999,max:5.00001}})),
 datum('bottom','flange','plane',faces({planar_normal_aligned_with:[0,0,-1]})),
 datum('axis','shaft','axis',faces({surface_type:'cylinder'})),
 datum('bottom','shaft','plane',faces({planar_normal_aligned_with:[0,0,-1]}))
],assembly:[
 {id:'base',input:'flange',translation:[25,15,2],rotation_deg:[0,25,10]},
 {id:'moving',input:'shaft',translation:[26,14,4],rotation_deg:[0,20,15]}
],mates:{anchors:['base'],expected_degrees_of_freedom:1,constraints:[
 {id:'center',kind:'concentric',alignment:'unoriented',a:{instance:'base',datum:'bore'},b:{instance:'moving',datum:'axis'}},
 {id:'seat',kind:'plane',distance:0,a:{instance:'base',datum:'bottom'},b:{instance:'moving',datum:'bottom'}}
]},checks:[{kind:'collision',parts:['base','moving'],max_overlap_mm3:1e-7}]},'foreign',step);
await writeFile(join(out,'model.json'),JSON.stringify(model,null,2)+'\n');
const cad=new CadClient({outputRoot:join(out,'builds')});
const built=await cad.build(model);assert.equal(built.ok,true,JSON.stringify(built));
assert.equal(built.data.assembly.mate_solution.degrees_of_freedom,1);
const request={schema:'colocad.query/1',query_revision_id:built.data.query_revision_id,target:{instance_path:['base']},space:'assembly',tolerance,queries:[{id:'mount_geometry',op:'named_datums',ids:['bore','bottom']}]};
await writeFile(join(out,'query.json'),JSON.stringify(request,null,2)+'\n');
const queried=await cad.query(built.data.artifact_directory,request);assert.equal(queried.ok,true,JSON.stringify(queried));
assert.equal(queried.data.execution.geometry_build_calls,0);
await writeFile(join(out,'query-receipt.json'),JSON.stringify(queried,null,2)+'\n');
const negative=await cad.edit(model,{shaft_radius:5.2});assert.equal(negative.ok,false);assert.equal(negative.error.code,'ASSEMBLY_COLLISION');
const report={schema:'colocad.query-example/1',sourceSha256:oracle.sha256,artifactDirectory:relative(out,built.data.artifact_directory),stepSha256:createHash('sha256').update(await readFile(join(built.data.artifact_directory,'model.step'))).digest('hex'),expectedVolumesMm3:[oracle.expected_volume_mm3,Math.PI*4.8**2*20],expectedMinimumDistanceMm:.2,built,negative,query:queried};
await writeFile(join(out,'run.json'),JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({passed:true,report:join(out,'run.json'),artifact:built.data.artifact_directory,request:join(out,'query.json')}));
