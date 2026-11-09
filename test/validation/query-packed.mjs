/** Run from an empty consumer project with the release archive installed.
 * Fixture generation is checkout data; every CAD call and CLI uses the installed package.
 */
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir,cp} from 'node:fs/promises';
import {join,resolve,relative} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {model as installation} from '../../examples/engineering-interfaces/model.mjs';

assert.ok(!process.env.AGENT_CAD_BINARY,'Pack acceptance must use the installed native, without overrides');
const pkg=resolve('node_modules/@coloopx/colocad');
const {CadClient,createCadTools}=await import(pathToFileURL(join(pkg,'src/index.mjs')));
const {ModelingSession}=await import(pathToFileURL(join(pkg,'src/session.mjs')));
const readJson=async p=>JSON.parse(await readFile(p,'utf8'));
const save=async(p,data)=>writeFile(p,JSON.stringify(data,null,2)+'\n');
const hash=b=>createHash('sha256').update(b).digest('hex');
const manifest=await readJson(join(pkg,'package.json')),release=await readJson(join(pkg,'kernel/release.json'));
const platform=process.platform+'-'+process.arch;
const native=join(pkg,'kernel/native',platform,'agent-cad');
assert.equal(manifest.version,release.version);assert.equal(hash(await readFile(native)),release.sha256[platform]);
const out=resolve(process.argv[2]??'query-release-check');await mkdir(out);
const cad=new CadClient({outputRoot:out,wallMs:120000});
const caps=await cad.capabilities();assert.equal(caps.ok,true,JSON.stringify(caps));
const tools=await createCadTools(cad),queryTool=tools.find(t=>t.name==='cad_query');assert.ok(queryTool?.inputSchema.$defs);
assert.equal((await cad.querySchema()).ok,true);
const tolerance={linear_mm:1e-5,angular_rad:1e-7};
const model={version:1,units:'mm',parameters:{height:8},features:[
 {id:'raw',op:'box',origin:[-20,-15,0],size:[40,30,{param:'height'}]},
 {id:'part',op:'holes',input:'raw',frame:{origin:[0,0,-1],x_axis:[1,0,0],y_axis:[0,1,0]},positions:[[-10,-6],[10,-6],[-10,6],[10,6]],radius:2,depth:{op:'add',a:{param:'height'},b:2}}
],output:'part',preview:{width:160,height:120,chord_mm:1},interfaces:[{
 id:'mount',definition:'part',profile:'planar_hole_mount/v1',
 selectors:{plane:{where:{planar_normal_aligned_with:[0,0,1]},cardinality:{min:1,max:1}},hole_group:{opening_on:'plane',where:{pattern:'rectangular',member_count:4,equal_diameter:true},cardinality:{min:1,max:1}}},
 datum:{origin:{kind:'hole_group_center'},normal:'outward'},
 requirements:{verification_tolerance:tolerance,pattern:{kind:'rectangular',count:[2,2],pitch_mm:[20,12]},bore_diameter_mm:4,termination:'through'}
}]};
const built=await cad.build(model);assert.equal(built.ok,true,JSON.stringify(built));
assert.ok(Math.abs(built.data.geometry.volume_mm3-(1200-16*Math.PI)*8)<1e-5);
const request=revision=>({schema:'colocad.query/1',query_revision_id:revision,target:{definition_id:'part'},space:'definition',tolerance,queries:[
 {id:'top',op:'faces',where:{planar_normal_aligned_with:[0,0,1]}},
 {id:'bores',op:'holes',where:{termination:'through'}},
 {id:'pattern',op:'hole_groups',where:{pattern:'rectangular',member_count:4}},
 {id:'mount',op:'interfaces',ids:['mount']}
]});
const q=request(built.data.query_revision_id);
const observed=await queryTool.execute({artifact_directory:built.data.artifact_directory,request:q});
assert.equal(observed.ok,true,JSON.stringify(observed));
assert.equal(observed.data.results[0].items.length,1);
assert.equal(observed.data.results[1].items.length,4);
assert.equal(observed.data.results[2].items.length,2);
assert.equal(observed.data.results[3].items[0].provenance,'declared');
for(const key of ['geometry_build_calls','step_export_calls','tessellation_calls'])assert.equal(observed.data.execution[key],0);
const moved=join(out,'relocated-snapshot');await cp(built.data.artifact_directory,moved,{recursive:true});
const reopened=await new CadClient({outputRoot:out,wallMs:120000}).query(moved,q);
assert.deepEqual(reopened.data.results,observed.data.results);
const cli=join(pkg,'bin/colocad.mjs'),input=join(out,'query.json'),cliReceipt=join(out,'cli-query.json');await save(input,q);
const command=JSON.parse(execFileSync(process.execPath,[cli,'query',moved,input,'--out',cliReceipt],{encoding:'utf8'}));
// The CLI serializes JSON once more; JSON.stringify canonicalizes numeric -0 to 0.
// Compare the wire representation without relaxing any geometric tolerance/ref.
assert.deepEqual(command.data.results,JSON.parse(JSON.stringify(observed.data.results)));

const session=await ModelingSession.create(join(out,'session'),{wallMs:120000});
const initial=await session.build(model);assert.equal(initial.ok,true,JSON.stringify(initial));
assert.equal((await session.query(request(initial.data.query_revision_id))).ok,true);
const edited=await session.edit({height:10});assert.equal(edited.ok,true,JSON.stringify(edited));
const stale=await session.query(request(initial.data.query_revision_id));assert.equal(stale.error.code,'QUERY_REVISION_MISMATCH');
const fresh=await session.query(request(edited.data.query_revision_id));assert.equal(fresh.ok,true,JSON.stringify(fresh));
assert.ok(Math.abs(fresh.data.results[0].items[0].center_mm[2]-10)<1e-5);
assert.equal(fresh.data.results[1].items.length,4);
const failed=await session.edit({height:-1});assert.equal(failed.ok,false);assert.equal((await session.source()).parameters.height,10);
execFileSync(process.execPath,[cli,'replay',session.root,join(out,'review.html')],{encoding:'utf8'});

// Same semantic installation recipe; independent OCCT-authored source geometry.
const fixtures=new URL('../../specs/experiments/0004/fixtures/engineering/',import.meta.url);
const fixtureManifest=await readJson(new URL('manifest.json',fixtures));
const source=async id=>{const row=fixtureManifest.cases.find(r=>r.id===id);const step=await readFile(new URL(row.file,fixtures),'utf8');assert.equal(hash(step),row.sha256);return {row,step};};
const support=await source('support'),runs=[];
for(const id of ['motor','motor-housing','motor-rotated','motor-wrong-pitch','motor-missing-hole']){
 const part=await source(id),result=await cad.build(installation(part.step,support.step));
 const row={id,sourceSha256:part.row.sha256,expectedVolumesMm3:[part.row.volume_mm3,support.row.volume_mm3],built:result};runs.push(row);
 if(!part.row.expected.positive){assert.equal(result.ok,false);assert.equal(result.error.code,part.row.expected.reject);continue;}
 assert.equal(result.ok,true,JSON.stringify(result));assert.equal(result.data.assembly.checks[0].passed,true);
 const check=await cad.query(result.data.artifact_directory,{schema:'colocad.query/1',query_revision_id:result.data.query_revision_id,target:{instance_path:['motorPart']},space:'assembly',tolerance,queries:[{id:'mount',op:'interfaces',ids:['mount']},{id:'gap',op:'measure',a:{instance_path:['motorPart']},b:{instance_path:['supportPart']}}]});
 assert.equal(check.ok,true,JSON.stringify(check));assert.equal(check.data.results[0].items[0].orientation.source,'geometry_derived_named_datum');assert.equal(check.data.execution.geometry_build_calls,0);
 row.query=check;row.artifactDirectory=relative(out,result.data.artifact_directory);row.stepSha256=hash(await readFile(join(result.data.artifact_directory,'model.step')));
}
await save(join(out,'runs.json'),{manifest:fixtureManifest,runs});
const ineffective={version:1,units:'mm',parameters:{},features:[{id:'stock',op:'box',origin:[0,0,0],size:[10,10,10]},{id:'tool',op:'box',origin:[20,0,0],size:[2,2,2]},{id:'cut',op:'cut',left:'stock',right:'tool'}],output:'cut'};
const rejected=await cad.build(ineffective);assert.equal(rejected.error.code,'NO_MATERIAL_REMOVED');assert.equal(rejected.error.feature_id,'cut');
await save(join(out,'receipts.json'),{caps,built,observed,initial,edited,stale,fresh,failed,rejected});
const summary={passed:true,version:manifest.version,platform,native_sha256:hash(await readFile(native)),execution_policy:caps.data.execution_policy,override:false,checks:['installed_sdk_and_query_tool','cli_query','exact_snapshot_relocation','four_bores_and_two_plane_scoped_groups','declared_interface','edit_and_stale_ref_rejection','failed_edit_preserves_source','offline_history_export','three_foreign_replacements_two_rejections','ineffective_cut_rejection'],foreign_positives:3,foreign_rejections:2,scope:'Installed archive correctness, not a performance comparison or live inference study'};
await save(join(out,'summary.json'),summary);console.log(JSON.stringify(summary));
