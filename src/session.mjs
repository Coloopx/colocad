import { mkdir, readFile, writeFile, rename, rm, realpath, open, opendir, stat } from 'node:fs/promises';
import { resolve, join, relative, isAbsolute, sep } from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import { performance } from 'node:perf_hooks';
import { catalogName, initializeCatalog, withCatalog, withLegacyWriter } from './session-catalog.mjs';
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
async function sessionPath(root,path){
  const file=inside(root,path),actual=await realpath(file),base=await realpath(root);
  if(!actual.startsWith(base+sep))throw Error('SESSION_PATH: symlink leaves session');
  return actual;
}
async function checked(root,path,hash){
  const bytes=await readFile(await sessionPath(root,path));if(hash&&sha(bytes)!==hash)throw Error('SESSION_INTEGRITY: '+path);return bytes;
}

// An inspection binds to immutable source bytes. Verify them with one reusable
// buffer so recording another question does not copy or parse the whole model.
async function verifySource(root,step) {
  const handle=await open(await sessionPath(root,step.inputFile),'r');
  try {
    const buffer=Buffer.allocUnsafe(64*1024), digest=createHash('sha256');
    for (;;) {
      const {bytesRead}=await handle.read(buffer,0,buffer.length,null);
      if (!bytesRead) break;
      digest.update(buffer.subarray(0,bytesRead));
    }
    if(digest.digest('hex')!==step.inputSha256) throw Error('SESSION_INTEGRITY: '+step.inputFile);
  } finally { await handle.close(); }
}

const accepted = step => step.status==='succeeded' && !['observe','query'].includes(step.operation);

async function syncPath(path) {
  const handle=await open(path,'r');
  try { await handle.sync(); } finally { await handle.close(); }
}
async function writeStepFile(file,bytes) {
  const handle=await open(file,'w');
  try { await handle.writeFile(bytes); await handle.sync(); } finally { await handle.close(); }
}
async function syncArtifactTree(directory) {
  for await (const item of await opendir(directory)) {
    const path=join(directory,item.name);
    if(item.isDirectory()) await syncArtifactTree(path);
    else if(item.isFile()) await syncPath(path);
    else throw Error('SESSION_PATH: unexpected artifact entry '+item.name);
  }
  await syncPath(directory);
}

async function legacyStore(root,writable,callback) {
  const file=join(root,'session.json'), ledger=await load(file);
  if(!writable && ledger.format==='colocad/session/2') return withCatalog(root,await catalogName(root),false,callback);
  if(ledger.format!==format || !Array.isArray(ledger.steps)) throw Error('SESSION_FORMAT: unsupported ledger');
  if(writable) {
    let changed=false;
    for(const step of ledger.steps) if(step.status==='running') {
      Object.assign(step,{status:'interrupted',finishedAt:new Date().toISOString(),error:{code:'INTERRUPTED',message:'Previous process ended before committing its result'}});
      changed=true;
    }
    if(changed) await atomic(file,ledger);
  }
  return callback({
    header:()=>{const {steps,...header}=ledger;return header;},
    head:()=>ledger.steps.findLast(accepted),
    length:()=>ledger.steps.length,
    page:(after,limit)=>({steps:ledger.steps.slice(after,after+limit),nextCursor:after+limit<ledger.steps.length?after+limit:null}),
    begin:async step=>{ledger.steps.push(step); await atomic(file,ledger);},
    finish:()=>atomic(file,ledger),
  });
}

export class ModelingSession {
  constructor(root,config={}){this.root=resolve(root);this.config=config;this.cad=new CadClient({...config,outputRoot:join(this.root,'artifacts')});}
  static async create(directory,{title='CAD modeling session',storage='indexed',...config}={}){
    const s=new ModelingSession(directory,config);
    if(!['indexed','json'].includes(storage)) throw Error('SESSION_FORMAT: unknown storage');
    await mkdir(s.root,{recursive:true});
    const file=join(s.root,'session.json');
    if(await stat(file).then(()=>true,e=>{if(e.code==='ENOENT')return false;throw e;}))
      throw Object.assign(Error('EEXIST: session already exists'),{code:'EEXIST'});
    const header={format,id:randomUUID(),title,createdAt:new Date().toISOString()};
    if(storage==='indexed') await initializeCatalog(s.root,'init',header);
    else await withLegacyWriter(s.root,()=>writeFile(file,json({...header,steps:[]}),{flag:'wx'}));
    return s;
  }
  static async open(directory,config={}){
    const s=new ModelingSession(directory,config);
    await s.withStore(true,store=>store.header());
    return s;
  }
  static async migrate(directory){
    const s=new ModelingSession(directory);
    const name=await catalogName(s.root);
    if(name) return {migrated:false,steps:await withCatalog(s.root,name,false,store=>store.length())};
    return {migrated:true,...await initializeCatalog(s.root,'import')};
  }
  async withStore(writable,callback) {
    const root=this.root;
    const name=await catalogName(root);
    if(name) return withCatalog(root,name,writable,callback);
    if(!writable) return legacyStore(root,false,callback);
    const retry=Symbol('concurrent migration');
    const result=await withLegacyWriter(root,async()=>{
      if(await catalogName(root)) return retry;
      return legacyStore(root,true,callback);
    });
    return result===retry ? this.withStore(writable,callback) : result;
  }
  async read(){
    return this.withStore(false,async store=>{
      const header=await store.header(),steps=[];
      let cursor=0;
      do {
        const page=await store.page(cursor,128);
        steps.push(...page.steps);cursor=page.nextCursor;
      } while(cursor!==null);
      return {...header,steps};
    });
  }
  async history({after=0,limit=64}={}) {
    if(!Number.isSafeInteger(after)||after<0||!Number.isInteger(limit)||limit<1||limit>256)
      throw Error('SESSION_PAGE: expected a nonnegative integer cursor and limit 1..256');
    return this.withStore(false,store=>store.page(after,limit));
  }
  async sourceFrom(store) {
    const step=await store.head();
    if(!step) throw Error('SESSION_EMPTY: build a model first');
    return JSON.parse(await checked(this.root,step.inputFile,step.inputSha256));
  }
  async source(){return this.withStore(false,store=>this.sourceFrom(store));}
  async latestGeometry(){return this.withStore(false,async store=>(await store.head())??undefined);}
  async head() {
    return this.withStore(false,async store=>{
      const step=await store.head();
      if(!step) return null;
      const receipt=JSON.parse(await checked(this.root,step.receiptFile,step.receiptSha256));
      return {session_step:step.id,query_revision_id:receipt.data.query_revision_id??null,geometry:step.geometry};
    });
  }
  async build(model,metadata={}){const input=structuredClone(model);return this.withStore(true,store=>this.run(store,input,'build',metadata));}
  async edit(parameters,metadata={}){return this.withStore(true,async store=>{
    const model=await this.sourceFrom(store);
    for(const key of Object.keys(parameters))if(!Object.hasOwn(model.parameters,key))return this.run(store,model,'edit',metadata,{ok:false,error:{code:'UNKNOWN_PARAMETER',message:key}},parameters);
    model.parameters={...model.parameters,...parameters};return this.run(store,model,'edit',metadata,undefined,parameters);
  });}
  async preview(model,options={},metadata={}){return this.withStore(true,async store=>{
    const next=structuredClone(model);next.preview={...next.preview,...options};const last=await store.head();
    if(last){const prior=await this.sourceFrom(store),clean=x=>{const v=structuredClone(x);delete v.preview;return v;};
      if(JSON.stringify(clean(prior))===JSON.stringify(clean(next))&&(prior.preview?.chord_mm??.2)===(next.preview?.chord_mm??.2)&&(prior.preview?.pose??'normal')===(next.preview?.pose??'normal'))return this.run(store,next,'observe',metadata);
    }return this.run(store,next,'build',metadata);
  });}
  async observe(options={},metadata={}){return this.withStore(true,async store=>{const model=await this.sourceFrom(store);model.preview={...model.preview,...options};return this.run(store,model,'observe',metadata);});}
  async query(request,metadata={}){const input=structuredClone(request);return this.withStore(true,async store=>this.run(store,undefined,'query',metadata,undefined,undefined,input));}
  async run(store,model,operation,{label=operation,kind=['observe','query'].includes(operation)?'inspection':'geometry',queryable}={},preset,requestedParameters,queryRequest){
    const id='s'+String(await store.length()+1).padStart(4,'0'),dir=join('steps',id);
    const previous=await store.head();
    const step={id,label:String(label),kind:String(kind),operation,status:'running',startedAt:new Date().toISOString(),receiptFile:dir+'/receipt.json'};
    if(operation==='query') {
      if(!previous) throw Error('SESSION_EMPTY: build a model first');
      if(previous.inputSha256) {
        await verifySource(this.root,previous);
        step.inputFile=previous.inputFile;
        step.inputSha256=previous.inputSha256;
      } else {
        // Older ledgers may have no digest. Materialize a checked input for this step.
        model=await this.sourceFrom(store);
      }
    } else if(operation!=='observe') {
      model.preview={view:'iso',chord_mm:.6,width:960,height:640,...model.preview};
    }
    await mkdir(join(this.root,dir),{recursive:true});
    if(!step.inputFile) {
      const bytes=json(model);
      step.inputFile=dir+'/input.json';
      step.inputSha256=sha(bytes);
      await writeStepFile(join(this.root,step.inputFile),bytes);
    }
    if(requestedParameters)step.requestedParameters=requestedParameters;
    if(queryRequest){const requestBytes=json(queryRequest);step.queryRequestFile=dir+'/query.json';step.queryRequestSha256=sha(requestBytes);await writeStepFile(join(this.root,step.queryRequestFile),requestBytes);}
    await syncPath(join(this.root,dir));
    await syncPath(join(this.root,'steps'));
    await syncPath(this.root);
    await store.begin(step);
    const start=performance.now();let result;
    try{
      step.queryable=queryable??((model?.datums?.length||model?.interfaces?.length)?true:previous?.queryable??false);
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
    if(result.ok && operation!=='query') {
      const directory=inside(this.root,step.artifactDirectory);
      await syncArtifactTree(directory);await syncPath(resolve(directory,'..'));await syncPath(this.root);
    }
    await writeStepFile(join(this.root,step.receiptFile),receiptBytes);await syncPath(join(this.root,dir));
    await store.finish(step);
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
  const root=resolve(directory),ledger=await new ModelingSession(root).read();
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
