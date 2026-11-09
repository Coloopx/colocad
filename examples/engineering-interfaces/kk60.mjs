// Whole catalog-backed product + independently authored accessory; only explicit pairs are checked.
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {resolve,join,relative} from 'node:path';
import assert from 'node:assert/strict';
import {CadClient,attachStep} from '../../kernel/dist/index.js';
import {kk60} from '../hiwin-kk60/model.mjs';
const out=resolve(process.argv[2]??'out/kk60-interfaces');await mkdir(out,{recursive:true});
const source=new URL('../../specs/experiments/0004/fixtures/engineering/',import.meta.url),manifest=JSON.parse(await readFile(new URL('manifest.json',source),'utf8')),row=manifest.cases.find(c=>c.id==='kk60-accessory-bracket'),step=await readFile(new URL(row.file,source),'utf8');assert.equal(createHash('sha256').update(step).digest('hex'),row.sha256);
let model=kk60();model.features.push({id:'accessoryBracket',op:'import_step',asset:'accessory',body_index:0});model=attachStep(model,'accessory',step);model.assembly.push({id:'accessory-bracket',input:'accessoryBracket'});
const tolerance={linear_mm:.01,angular_rad:1e-4}; // frozen catalog-reconstruction acceptance, not manufacturing tolerance
const declaration=(definition,normal,diameter,termination,originDatum)=>({id:'mount',definition,profile:'planar_hole_mount/v1',selectors:{plane:{where:originDatum?{planar_coplanar_with:{datum:originDatum}}:{surface_type:'plane',planar_normal_aligned_with:normal},cardinality:{min:1,max:1}},hole_group:{opening_on:'plane',where:{pattern:'rectangular',member_count:4,equal_diameter:true},cardinality:{min:1,max:1}}},datum:{origin:{kind:'hole_group_center'},normal:'outward',x_axis:{kind:'orientation_datum',name:'designX'}},orientation_datums:[{kind:'direction',name:'designX',vector:[1,0,0]}],requirements:{verification_tolerance:tolerance,pattern:{kind:'rectangular',count:[2,2],pitch_mm:[30,30]},bore_diameter_mm:diameter,termination}});
model.datums=[{id:'carriageTop',definition:'datumSlotB',tolerance,derive:{kind:'selected',derive:{kind:'plane'},selection:{kind:'rank',input:{kind:'filter',input:{kind:'entities',entity:'face'},where:{planar_normal_aligned_with:[0,1,0]}},by:{property:'center_projection',direction:[0,1,0]},descending:true,rank:0,tie_tolerance:.01}}}];
model.interfaces=[declaration('datumSlotB',[0,1,0],5,'blind','carriageTop'),declaration('accessoryBracket',[0,0,-1],5.5,'through')];
model.mates={anchors:model.assembly.filter(i=>i.id!=='accessory-bracket').map(i=>i.id),constraints:[{id:'accessoryMount',kind:'fixed',alignment:'opposed',a:{instance:'carriage-machined',interface:'mount'},b:{instance:'accessory-bracket',interface:'mount'}}]};
const poses=[-105,0,105].map(z=>({id:z<0?'retracted':z>0?'extended':'center',a:{translation:[0,0,z]}}));
model.checks=[{kind:'interface_compatibility',a:{instance:'carriage-machined',interface:'mount'},b:{instance:'accessory-bracket',interface:'mount'},plane_relation:'opposed',correspondence:{kind:'pattern',require_all:true,symmetry:'require_unique'},tolerance,requirements:{max_plane_gap_mm:.01,max_hole_axis_offset_mm:.01,hole_size:{kind:'independent_ranges',a_diameter_mm:{min:4.99,max:5.01},b_diameter_mm:{min:5.49,max:5.51}}}},
 {kind:'geometric_relationship',a:{instance:'accessory-bracket'},b:{instance:'carriage-machined'},tolerance,max_overlap_mm3:1e-6,pose_samples:poses.map(s=>({...s,b:s.a}))},
 ...['rail-machined','motor-bearing-support'].map(id=>({kind:'geometric_relationship',a:{instance:'accessory-bracket'},b:{instance:id},tolerance,max_overlap_mm3:1e-6,min_clearance_mm:.2,pose_samples:poses}))];
await writeFile(join(out,'model.json'),JSON.stringify(model,null,2)+'\n');
const cad=new CadClient({outputRoot:join(out,'builds'),wallMs:120000});const built=await cad.build(model);await writeFile(join(out,'run.json'),JSON.stringify({source:row,sourcedProduct:new URL('../hiwin-kk60/sources.json',import.meta.url).pathname,built},null,2)+'\n');assert.equal(built.ok,true,JSON.stringify(built));
console.log(JSON.stringify({passed:true,instances:built.data.geometry.solids,checks:built.data.assembly.checks,artifact:relative(out,built.data.artifact_directory)}));
