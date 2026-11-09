// Actual remote inference; Agent, tools, session and native CAD are local to this cgroup.
import {readFile,writeFile,mkdir,appendFile} from 'node:fs/promises';
import {join} from 'node:path';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
import {ModelingSession} from '/repo/src/session.mjs';
import {createCompactSessionTools} from '/repo/examples/integrations/compact-tools.mjs';
import {resources,saveResources} from './resource.mjs';
const {Agent}=await import('/opt/agent/node_modules/@earendil-works/pi-agent-core/dist/index.js');
const {streamSimple}=await import('/opt/agent/node_modules/@earendil-works/pi-ai/dist/api/openai-completions.js');
const out=process.argv[2]??'/evidence';await mkdir(out,{recursive:true});
const config=JSON.parse(await readFile('/run/colocad-auth.json','utf8')),mode=process.argv[3]??'plate';
const model={...config.model,provider:'acceptance-gateway',api:config.api,baseUrl:config.baseUrl,cost:{input:0,output:0,cacheRead:0,cacheWrite:0},contextWindow:128000,maxTokens:8192};
const s=await ModelingSession.create(join(out,'session'),{title:'Live Pi '+mode+' acceptance',wallMs:120000});let seed;
if(mode==='product'){
 const m=JSON.parse(await readFile('/repo/specs/experiments/0005/corpus-v1/product-kk60-accessory.json'));m.preview={width:320,height:240,chord_mm:1};seed=await s.build(m,{queryable:true,label:'Frozen sourced product before live edit'});assert.equal(seed.ok,true,JSON.stringify(seed.error));
}
const start=await resources('resident_pi_and_local_harness_start');const calls=[];
const definitions=await createCompactSessionTools(s,{include:['cad_capabilities','cad_validate','cad_build','cad_edit','cad_query']});
const tools=definitions.map(t=>({name:t.name,label:t.name,description:t.description,parameters:t.inputSchema,executionMode:'sequential',async execute(id,args,signal){
 signal?.throwIfAborted();const begin=performance.now();const result=await t.execute(args);calls.push({tool:t.name,toolCallId:id,arguments:args,result,duration_ms:performance.now()-begin,resources:await resources(t.name)});await writeFile(join(out,'tool-calls.json'),JSON.stringify(calls,null,2));
 const content=[{type:'text',text:JSON.stringify(result)}];return {content,details:result,isError:!result.ok};
}}));
const skill=await readFile('/repo/skills/colocad/SKILL.md','utf8');
const agent=new Agent({initialState:{model,systemPrompt:'You are validating a local parametric CAD package. Use the provided CAD tools to perform the task, not prose-only instructions. Agent, harness and CAD run inside this sandbox. Only explicit successful receipts establish success. Preserve errors, never claim a failed operation passed. Use query_revision_id from the latest build/edit. Avoid gratuitous rebuilds. Tool JSON schemas are authoritative.\n'+skill,tools},getApiKey:()=>config.apiKey,toolExecution:'sequential',streamFn:(m,c,o)=>streamSimple(m,c,{...o,maxTokens:8192})});
const prompt=mode==='plate'?`Build an exact parametric 80 x 50 x 6 mm mounting plate, origin [0,0,0], with four through holes centered at [10,10], [70,10], [10,40], [70,40], diameter 6 mm. Declare numeric parameter holeRadius=3 and use that parameter in a holes feature. Final feature ID part. Use version:1 units:mm model, box plus holes, holes frame origin [0,0,-1], x_axis [1,0,0], y_axis [0,1,0], depth 8. Validate then build with options.queryable=true and preview width320 height240 chord_mm1. Query op holes to verify four actual through holes. Then edit holeRadius to 3.5, and query the NEW revision's holes to verify diameter7. Keep all four hole centers unchanged. Query tolerance linear_mm=1e-5 angular_rad=1e-7, target definition_id part, space definition. Return saved STEP path and measured results. No extra model changes.`:
`An accepted catalog-backed KK60 actuator plus independent accessory bracket is already in the recorded session (82 solids). Source dimensions are a reconstruction, not certified OEM CAD. Current travel parameter=0 mm; current query_revision_id=${seed.data.query_revision_id}. Query the named installation interface mount on instance_path [accessory-bracket], space assembly. Then use cad_edit to set travel=20 mm. On the NEW revision query the same interface plus op measure with a instance_path [accessory-bracket] and b instance_path [motor-bearing-support]. Verify 82 solids remain, the interface origin translates +20 mm along Z, and the bracket is separated from the motor support by at least0.2 mm. Tolerance linear_mm=.01 angular_rad=.0001. The latest build's required geometry checks must also succeed. Return the saved STEP path, actual interface origin and distance. Do not rebuild the provided product from prose or change any geometry other than travel.`;
await writeFile(join(out,'prompt.txt'),prompt);let turns=0;const trace=[];agent.subscribe(e=>{if(e.type==='message_end'){trace.push(e.message);if(e.message.role==='assistant'&&++turns>12)agent.abort();}});
const timer=setTimeout(()=>agent.abort(),420000);let failure;
try{await agent.prompt(prompt);}catch(e){failure=String(e.message);}finally{clearTimeout(timer);await writeFile(join(out,'messages.json'),JSON.stringify(trace,null,2));}
const source=await s.source().catch(()=>null),ledger=await s.read(),accepted=await s.latestGeometry(),final=await saveResources(out,'resident_pi_plus_local_harness_plus_native');
const queryCalls=calls.filter(c=>c.tool==='cad_query'&&c.result.ok);let verified=false,measurements={};
if(mode==='plate'&&source){
 const last=queryCalls.at(-1)?.result.data.results[0];const holes=last?.items??[];verified=source.parameters.holeRadius===3.5&&holes.length===4&&holes.every(h=>h.termination==='through'&&h.segments.every(s=>Math.abs(s.diameter_mm-7)<1e-5));measurements={holes};
}else if(source){
 const before=queryCalls.flatMap(c=>c.result.data.results).find(r=>r.op==='interfaces')?.items[0];const after=queryCalls.flatMap(c=>c.result.data.results).findLast(r=>r.op==='interfaces')?.items[0];const measure=queryCalls.flatMap(c=>c.result.data.results).findLast(r=>r.op==='measure')?.measurement;
 verified=source.parameters.travel===20&&accepted.geometry.solids===82&&before&&after&&Math.abs(after.origin_mm[2]-before.origin_mm[2]-20)<.01&&measure?.relationship==='separated'&&measure.boundary_distance_mm>=.2;measurements={before:before?.origin_mm,after:after?.origin_mm,relationship:measure};
}
const usage=trace.filter(m=>m.role==='assistant').reduce((s,m)=>({input:s.input+(m.usage?.input??0),output:s.output+(m.usage?.output??0),cacheRead:s.cacheRead+(m.usage?.cacheRead??0),cacheWrite:s.cacheWrite+(m.usage?.cacheWrite??0)}),{input:0,output:0,cacheRead:0,cacheWrite:0});
const summary={passed:!!verified&&!failure&&final.memory_events.oom_kill===0,mode,inference:'live_remote_model_no_scripted_responses',model:model.id,api:model.api,cost:'unknown; zero-valued SDK cost metadata is not a billing claim',usage,tool_calls:calls.length,tool_errors:calls.filter(c=>!c.result.ok).length,assistant_turns:turns,session_steps:ledger.steps.map(x=>({id:x.id,operation:x.operation,status:x.status,error:x.error})),measurements,start_resources:start,final_resources:final,skill_sha256:createHash('sha256').update(skill).digest('hex'),native_sha256:createHash('sha256').update(await readFile(process.env.AGENT_CAD_BINARY)).digest('hex'),failure,provider_errors:trace.filter(m=>m.role==='assistant'&&m.stopReason==='error').map(m=>m.errorMessage)};
await writeFile(join(out,'summary.json'),JSON.stringify(summary,null,2));console.log(JSON.stringify({passed:summary.passed,mode,usage,tool_calls:summary.tool_calls,peak_bytes:final.memory_peak_bytes,failure}));if(!summary.passed)process.exitCode=1;
