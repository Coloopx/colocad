// Transport-neutral adapter: keep full receipts on disk and send a focused result
// to the model. Host adapters still decide how to attach a PNG and flag errors.
import {join} from 'node:path';
import {createSessionTools} from '../../src/session.mjs';
export function compactReceipt(receipt){
 if(!receipt.ok||!Array.isArray(receipt.data?.features))return structuredClone(receipt);
 const d=receipt.data,features=d.features;
 const artifacts=Object.fromEntries(Object.entries(d.artifact_sha256??{})
  .filter(([name])=>/^(model|preview)\.(json|step|png|mesh\.json|glb|stl)$/.test(name))
  .map(([name,sha256])=>[name,{path:join(d.artifact_directory,name),sha256}]));
 return {ok:true,...(receipt.session_step?{session_step:receipt.session_step}:{}),data:{
  artifact_directory:d.artifact_directory,source_artifact_directory:d.source_artifact_directory,
  model_hash:d.model_hash,source_hash:d.source_hash,query_revision_id:d.query_revision_id,execution_policy:d.execution_policy,geometry:structuredClone(d.geometry),
  timing_ms:structuredClone(d.timing_ms),validation:structuredClone(d.validation),artifacts,
  feature_summary:{count:features.length,reused:features.filter(f=>f.cache_hit).length,
   slowest:[...features].sort((a,b)=>(b.elapsed_ms??0)-(a.elapsed_ms??0)).slice(0,3).map(f=>({id:f.id,elapsed_ms:f.elapsed_ms,cache_hit:f.cache_hit})),
   degradations:features.filter(f=>f.degradation).map(f=>({id:f.id,...structuredClone(f.degradation)}))},
 }};
}
export async function createCompactSessionTools(session,{include=['cad_capabilities','cad_validate','cad_build','cad_preview','cad_edit']}={}){
 const tools=await createSessionTools(session),enabled=new Set(include);
 for(const name of enabled)if(!tools.some(t=>t.name===name))throw Error('Unknown CAD tool: '+name);
 return tools.filter(t=>enabled.has(t.name)).map(tool=>({...tool,
  description:tool.description+' Results summarize measurements and timing; full receipts remain in the recorded session. Report every listed degradation.',
  async execute(args){const result=compactReceipt(await tool.execute(args));if(result.session_step)result.receipt_file=join(session.root,'steps',result.session_step,'receipt.json');return result;},
 }));
}
