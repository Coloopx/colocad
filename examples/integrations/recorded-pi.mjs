import { join,resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { createAssistantMessageEventStream } from '@earendil-works/pi-ai';
import { ModelingSession,exportSessionHtml } from '../../src/session.mjs';
import { turbofan } from '../turbofan/model.mjs';
import { createCadAgent } from './pi-core.mjs';

// Replace only streamFn/model/auth with your harness configuration for live inference.
// This deterministic example verifies the same Pi loop without requiring credentials.
export async function runRecordedPiExample(directory){
 const root=resolve(directory),session=await ModelingSession.create(join(root,'session'),{title:'Pi agent • fan row revision history'});
 const part=turbofan('fan-row');let turn=0;
 const model={id:'scripted-contract-test',name:'Scripted contract test',api:'openai-completions',provider:'test',baseUrl:'https://unused.invalid',reasoning:false,input:['text','image'],cost:{input:0,output:0,cacheRead:0,cacheWrite:0},contextWindow:100000,maxTokens:2000};
 const calls=[['cad_build',{model:part}],['cad_edit',{parameters:{unknownDimension:10}}],['cad_edit',{parameters:{fanRadius:144}}]];
 const streamFn=()=>{
   const call=calls[turn++],stopReason=call?'toolUse':'stop';
   const message={role:'assistant',content:call?[{type:'toolCall',id:'call-'+turn,name:call[0],arguments:call[1]}]:[{type:'text',text:'Fan row corrected and exported.'}],api:model.api,provider:model.provider,model:model.id,usage:{input:0,output:0,cacheRead:0,cacheWrite:0,totalTokens:0,cost:{input:0,output:0,cacheRead:0,cacheWrite:0,total:0}},stopReason,timestamp:Date.now()};
   const stream=createAssistantMessageEventStream();stream.push({type:'done',reason:stopReason,message});return stream;
 };
 const agent=await createCadAgent({model,streamFn,outputRoot:root,cadSession:session});const events=[];
 agent.subscribe(e=>{if(e.type==='tool_execution_end')events.push(e);});await agent.prompt('Build the fan row, then revise the tip radius.');
 const html=join(root,'history.html');await exportSessionHtml(session.root,html);
 return {html,toolErrors:events.filter(e=>e.isError).length,solids:events.at(-1)?.result?.details?.geometry?.solids,scope:'Real Pi core and Rust; deterministic model response stream'};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(await runRecordedPiExample(process.argv[2]??'./out/pi-history'),null,2));
