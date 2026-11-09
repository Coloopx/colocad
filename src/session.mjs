import { mkdir, readFile, writeFile, rename, rm, realpath } from 'node:fs/promises';
import { resolve, join, relative, isAbsolute, sep } from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import { performance } from 'node:perf_hooks';
import { CadClient, createCadTools } from './index.mjs';

const format='colocad/session/1';
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const json=value=>JSON.stringify(value,null,2)+'\n';
async function atomic(file,value){const temp=file+'.'+randomUUID()+'.tmp';await writeFile(temp,json(value));await rename(temp,file);}
async function load(file){return JSON.parse(await readFile(file,'utf8'));}
function inside(root,path){
  if(typeof path!=='string'||isAbsolute(path)||path.split(/[\\/]/).includes('..'))throw Error('SESSION_PATH: expected relative artifact path');
  const target=resolve(root,path);if(target!==root&&!target.startsWith(root+sep))throw Error('SESSION_PATH');return target;
}
async function checked(root,path,hash){
  const file=inside(root,path),actual=await realpath(file),base=await realpath(root);
  if(!actual.startsWith(base+sep))throw Error('SESSION_PATH: symlink leaves session');
  const bytes=await readFile(file);if(hash&&sha(bytes)!==hash)throw Error('SESSION_INTEGRITY: '+path);return bytes;
}
function alive(pid){if(!Number.isInteger(pid)||pid<1)return true;try{process.kill(pid,0);return true;}catch(e){return e.code!=='ESRCH';}}

export class ModelingSession {
  constructor(root,config={}){this.root=resolve(root);this.config=config;this.cad=new CadClient({...config,outputRoot:join(this.root,'artifacts')});}
  static async create(directory,{title='CAD modeling session',...config}={}){
    const s=new ModelingSession(directory,config);await mkdir(s.root,{recursive:true});
    await writeFile(join(s.root,'session.json'),json({format,id:randomUUID(),title,createdAt:new Date().toISOString(),steps:[]}),{flag:'wx'});return s;
  }
  static async open(directory,config={}){
    const s=new ModelingSession(directory,config);await s.read();
    await s.lock(async()=>{const ledger=await s.read();let changed=false;
      for(const step of ledger.steps)if(step.status==='running'){Object.assign(step,{status:'interrupted',finishedAt:new Date().toISOString(),error:{code:'INTERRUPTED',message:'Previous process ended before committing its result'}});changed=true;}
      if(changed)await atomic(join(s.root,'session.json'),ledger);
    },true);return s;
  }
  async read(){const ledger=await load(join(this.root,'session.json'));if(ledger.format!==format||!Array.isArray(ledger.steps))throw Error('SESSION_FORMAT: unsupported ledger');return ledger;}
  async lock(fn,recover=false){
    const file=join(this.root,'.writer.lock');
    if(recover){try{const owner=await load(file);if(!alive(owner.pid))await rm(file);}catch(e){if(e.code!=='ENOENT')throw e;}}
    try{await writeFile(file,json({pid:process.pid}),{flag:'wx'});}catch(e){if(e.code==='EEXIST')throw Error('SESSION_BUSY: another local writer owns this session');throw e;}
    try{return await fn();}finally{await rm(file,{force:true});}
  }
  async source(){const l=await this.read(),s=l.steps.findLast(x=>x.status==='succeeded'&&!['observe','query'].includes(x.operation));if(!s)throw Error('SESSION_EMPTY: build a model first');return JSON.parse(await checked(this.root,s.inputFile,s.inputSha256));}
  async latestGeometry(){return (await this.read()).steps.findLast(x=>x.status==='succeeded'&&!['observe','query'].includes(x.operation));}
  async build(model,metadata={}){const input=structuredClone(model);return this.lock(()=>this.run(input,'build',metadata));}
  async edit(parameters,metadata={}){return this.lock(async()=>{
    const model=await this.source();
    // Keep invalid edits in history too; the native validator provides the receipt.
    for(const key of Object.keys(parameters))if(!Object.hasOwn(model.parameters,key))return this.run(model,'edit',metadata,{ok:false,error:{code:'UNKNOWN_PARAMETER',message:key}},parameters);
    model.parameters={...model.parameters,...parameters};return this.run(model,'edit',metadata,undefined,parameters);
  });}
  async preview(model,options={},metadata={}){return this.lock(async()=>{
    const next=structuredClone(model);next.preview={...next.preview,...options};const last=await this.latestGeometry();
    if(last){const prior=await this.source(),clean=x=>{const v=structuredClone(x);delete v.preview;return v;};
      if(JSON.stringify(clean(prior))===JSON.stringify(clean(next))&&(prior.preview?.chord_mm??.2)===(next.preview.chord_mm??.2)&&(prior.preview?.pose??'normal')===(next.preview.pose??'normal'))return this.run(next,'observe',metadata);
    }return this.run(next,'build',metadata);
  });}
  async observe(options={},metadata={}){return this.lock(async()=>{const model=await this.source();model.preview={...model.preview,...options};return this.run(model,'observe',metadata);});}
  async query(request,metadata={}){const input=structuredClone(request);return this.lock(async()=>this.run(await this.source(),'query',metadata,undefined,undefined,input));}
  async run(model,operation,{label=operation,kind=['observe','query'].includes(operation)?'inspection':'geometry',queryable}={},preset,requestedParameters,queryRequest){
    const ledger=await this.read(),id='s'+String(ledger.steps.length+1).padStart(4,'0'),dir=join('steps',id);
    await mkdir(join(this.root,dir),{recursive:true});
    if(!['observe','query'].includes(operation))model.preview={view:'iso',chord_mm:.6,width:960,height:640,...model.preview};
    const bytes=json(model);await writeFile(join(this.root,dir,'input.json'),bytes);
    const step={id,label:String(label),kind:String(kind),operation,status:'running',startedAt:new Date().toISOString(),inputFile:dir+'/input.json',inputSha256:sha(bytes),receiptFile:dir+'/receipt.json'};
    if(requestedParameters)step.requestedParameters=requestedParameters;
    if(queryRequest){const requestBytes=json(queryRequest);step.queryRequestFile=dir+'/query.json';step.queryRequestSha256=sha(requestBytes);await writeFile(join(this.root,step.queryRequestFile),requestBytes);}
    ledger.steps.push(step);await atomic(join(this.root,'session.json'),ledger);
    const start=performance.now();let result;
    try{
      const previous=ledger.steps.slice(0,-1).findLast(x=>x.status==='succeeded'&&!['observe','query'].includes(x.operation));
      step.queryable=queryable??((model.datums?.length||model.interfaces?.length)?true:previous?.queryable??false);
      const accepted=operation==='query'?JSON.parse(await checked(this.root,previous.receiptFile,previous.receiptSha256)):undefined;
      if(accepted&&queryRequest.query_revision_id!==accepted.data.query_revision_id)preset={ok:false,error:{code:'QUERY_REVISION_MISMATCH',message:'Query must target the accepted geometry revision'}};
      result=preset??await (operation==='query'
        ?this.cad.query(inside(this.root,previous.artifactDirectory),queryRequest)
        :operation==='observe'
        ?this.cad.observeArtifact(inside(this.root,previous.artifactDirectory),model.preview)
        :this.cad.build(model,{previousRevision:previous?inside(this.root,previous.artifactDirectory):undefined,queryable:step.queryable}));
      if(operation==='query'&&result.ok){result.data.artifact_directory=inside(this.root,previous.artifactDirectory);result.data.source_artifact_directory=result.data.artifact_directory;result.data.source_artifact_sha256=accepted.data.artifact_sha256;}
    }catch(e){result={ok:false,error:{code:'SESSION_EXECUTION_FAILED',message:String(e.message)}};}
    step.durationMs=performance.now()-start;step.finishedAt=new Date().toISOString();step.status=result.ok?'succeeded':'failed';
    const saved=structuredClone(result);
    if(result.ok){
      step.artifactDirectory=relative(this.root,result.data.artifact_directory).split(sep).join('/');inside(this.root,step.artifactDirectory);
      saved.data.artifact_directory=step.artifactDirectory;
      if(saved.data.source_artifact_directory)saved.data.source_artifact_directory=relative(this.root,saved.data.source_artifact_directory).split(sep).join('/');
      step.geometry=result.data.geometry;step.nativeTimingMs=result.data.timing_ms;
      step.degradations=result.data.features?.filter(x=>x.degradation).map(x=>({id:x.id,...x.degradation}));
    }else step.error=result.error;
    const receiptBytes=json(saved);step.receiptSha256=sha(receiptBytes);
    await writeFile(join(this.root,step.receiptFile),receiptBytes);await atomic(join(this.root,'session.json'),ledger);
    return {...result,session_step:id};
  }
  capabilities(){return this.cad.capabilities();} schema(){return this.cad.schema();} validate(m){return this.cad.validate(m);}
  solveSketch(s,p){return this.cad.solveSketch(s,p);} inspectStep(m,a){return this.cad.inspectStep(m,a);}
}

export async function createSessionTools(session) {
  const tools=await createCadTools(session.cad);
  for(const tool of tools){
    if(tool.name==='cad_build')tool.execute=({model,options})=>session.build(model,{label:'Agent build',queryable:options?.queryable});
    if(tool.name==='cad_query'){
      delete tool.inputSchema.properties.artifact_directory;tool.inputSchema.required=['request'];
      tool.description='Query the latest accepted exact snapshot, persist request/result, and reject stale revisions without changing accepted source.';
      tool.execute=({request})=>session.query(request,{label:'Agent geometry query'});
    }
    if(tool.name==='cad_preview')tool.execute=({model,options})=>session.preview(model,options,{label:'Agent preview'});
    if(tool.name==='cad_edit'){
      delete tool.inputSchema.properties.model;
      tool.inputSchema.required=['parameters'];
      tool.description='Edit numeric parameters of the latest successful recorded model; preserve failed attempts and return a new revision.';
      tool.execute=({parameters})=>session.edit(parameters,{label:'Agent parameter edit'});
    }
  }
  return tools;
}

export async function readSessionBundle(directory,{cutaway=false,view='iso'}={}){
  const root=resolve(directory),ledger=await load(join(root,'session.json'));
  if(ledger.format!==format)throw Error('SESSION_FORMAT');
  const bundle={format:'colocad/viewer/1',title:ledger.title,id:ledger.id,createdAt:ledger.createdAt,display:{cutaway,view},steps:[],meshes:{}};
  const last=ledger.steps.findLast(x=>x.status==='succeeded'&&!['observe','query'].includes(x.operation));
  for(const entry of ledger.steps){
    const step={...entry};
    if(entry.inputFile)step.model=JSON.parse(await checked(root,entry.inputFile,entry.inputSha256));
    if(entry.receiptFile&&entry.receiptSha256)step.receipt=JSON.parse(await checked(root,entry.receiptFile,entry.receiptSha256));
    if(entry.queryRequestFile)step.queryRequest=JSON.parse(await checked(root,entry.queryRequestFile,entry.queryRequestSha256));
    if(entry.status==='succeeded'){
      const data=step.receipt.data,artifact=entry.artifactDirectory;inside(root,artifact);
      // Observation results keep their original exact-geometry source for later lookup.
      const source=data.source_artifact_directory??artifact;
      const sourceReport=entry.operation==='query'?{artifact_sha256:data.source_artifact_sha256}:source===artifact?data:await load(inside(root,source+'/report.json'));
      const guideHash=sourceReport.artifact_sha256?.['query-display.json'];
      if(guideHash)step.queryDisplay=JSON.parse(await checked(root,source+'/query-display.json',guideHash));
      const hash=sourceReport.artifact_sha256?.['preview.mesh.json'];
      if(!hash)throw Error('SESSION_INTEGRITY: missing native preview mesh hash');
      const scene=JSON.parse(await checked(root,source+'/preview.mesh.json',hash));
      step.scene={instances:scene.instances?.length?scene.instances:scene.meshes.map(([id])=>({id,definition:id,origin:[0,0,0],axes:[[1,0,0],[0,1,0],[0,0,1]]})),definitions:{}};
      for(const [id,mesh] of scene.meshes){const key=sha(JSON.stringify(mesh));bundle.meshes[key]??=mesh;step.scene.definitions[id]=key;}
      if(data.artifact_sha256?.['preview.png'])step.png='data:image/png;base64,'+(await checked(root,artifact+'/preview.png',data.artifact_sha256['preview.png'])).toString('base64');
      if(entry.id===last?.id)step.stepFile=(await checked(root,source+'/model.step',sourceReport.artifact_sha256['model.step'])).toString('base64');
    }
    bundle.steps.push(step);
  }
  return bundle;
}

export async function exportSessionHtml(directory,destination,options={}){
  const bundle=await readSessionBundle(directory,options);
  const runtime=await readFile(new URL('../viewer/dist/standalone.js',import.meta.url),'utf8');
  const payload=JSON.stringify(bundle).replaceAll('<','\\u003c').replaceAll('\u2028','\\u2028').replaceAll('\u2029','\\u2029');
  const html='<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>colocad modeling history</title><style>html,body{margin:0;height:100%;background:#0a1220}colocad-viewer{display:block;height:100vh}</style><colocad-viewer></colocad-viewer><script id="colocad-session" type="application/json">'+payload+'</script><script>'+runtime.replaceAll('</script','<\\/script')+'</script><script>document.querySelector("colocad-viewer").session=JSON.parse(document.getElementById("colocad-session").textContent);</script></html>';
  await mkdir(resolve(destination,'..'),{recursive:true});await writeFile(destination,html);
  return {file:resolve(destination),steps:bundle.steps.length,meshDefinitions:Object.keys(bundle.meshes).length,bytes:Buffer.byteLength(html)};
}
