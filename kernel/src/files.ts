import {open,readFile,writeFile,link,unlink} from 'node:fs/promises';
import {randomUUID,createHash} from 'node:crypto';
import {dirname,basename,extname,join} from 'node:path';
import {CadClient} from './client.js';
import {attachStep} from './assets.js';
import type {Model,Receipt} from './types.js';

const limit=32*1024*1024;
const marker='/* AGENT_CAD_IR_1 ';
const hash=(s:string|Buffer)=>createHash('sha256').update(s).digest('hex');
function requireData(r:Receipt){if(!r.ok)throw Object.assign(Error(r.error.message),{code:r.error.code});return r.data;}
const unsupported=(path:string)=>Object.assign(Error('No local CAD file codec for '+extname(path)),{code:'UNSUPPORTED_FILE_FORMAT'});
async function bounded(path:string){
  const fd=await open(path,'r');
  try{
    const s=await fd.stat();if(!s.isFile()||s.size>limit)throw Error('CAD input must be a regular file up to 32 MiB');
    const bytes=await fd.readFile();if(bytes.length>limit)throw Error('CAD file grew beyond its budget');return bytes;
  }finally{await fd.close();}
}
/** Paths belong to the Harness in the current sandbox. Foreign STEP geometry is imported as an asset. */
export async function readModelFile(cad:CadClient,path:string):Promise<Model>{
  const ext=extname(path).toLowerCase();if(!['.json','.step','.stp'].includes(ext))throw unsupported(path);
  const raw=(await bounded(path)).toString('utf8');
  let model:Model;
  if(ext==='.json'){
    model=JSON.parse(raw) as Model;
  }else{
    if(!/^\s*ISO-10303-21;/i.test(raw))throw Error('invalid STEP exchange header');
    const start=raw.indexOf(marker);
    if(start<0){
      const id='source_part';
      const seed=attachStep({version:1,units:'mm',parameters:{},features:[],output:''},id,raw);
      const inspected=requireData(await cad.inspectStep(seed,id));
      model={...(inspected.model_template as Model),assets:seed.assets};
    }else{
      const end=raw.indexOf(' */\n',start);
      if(end<0||raw.indexOf(marker,start+marker.length)>=0)throw Error('invalid embedded model record');
      const fields=raw.slice(start+marker.length,end).split(' ');
      if(fields.length!==3)throw Error('invalid embedded model fields');
      const [stepHash,sourceHash,encoded]=fields;
      const step=raw.slice(0,start)+raw.slice(end+4);const source=Buffer.from(encoded,'base64');
      if(hash(step)!==stepHash||hash(source)!==sourceHash||source.toString('base64')!==encoded)throw Error('embedded model or STEP digest mismatch');
      model=JSON.parse(source.toString('utf8')) as Model;
    }
  }
  requireData(await cad.validate(model));return model;
}
async function publish(path:string,bytes:Buffer){
  const temp=join(dirname(path),'.'+basename(path)+'.'+randomUUID()+'.tmp');
  try{await writeFile(temp,bytes,{flag:'wx',mode:0o600});await link(temp,path);}
  finally{await unlink(temp).catch(e=>{if(e.code!=='ENOENT')throw e;});}
}
/** Export a new file. A STEP with embedModel=true can be reopened and edited from that single file. */
export async function writeModelFile(cad:CadClient,model:Model,path:string,options:{embedModel?:boolean}={}){
  const ext=extname(path).toLowerCase();const names:Record<string,string>={'.json':'model.json','.step':'model.step','.stp':'model.step','.glb':'model.glb','.stl':'model.stl','.png':'preview.png'};
  if(!Object.hasOwn(names,ext))throw unsupported(path);
  const preview=['.png','.stl','.glb'].includes(ext);
  const receipt=requireData(await cad.build(preview?{...model,preview:model.preview??{}}:model));
  let bytes=await readFile(join(String(receipt.artifact_directory),names[ext]));
  if(options.embedModel&&['.step','.stp'].includes(ext)){
    const step=bytes.toString('utf8'),source=Buffer.from(JSON.stringify(model),'utf8');
    const first=step.indexOf('\n')+1;if(!first)throw Error('invalid STEP output header');
    const comment=marker+hash(bytes)+' '+hash(source)+' '+source.toString('base64')+' */\n';
    bytes=Buffer.from(step.slice(0,first)+comment+step.slice(first));
  }
  if(bytes.length>limit)throw Error('file exceeds 32 MiB');
  await publish(path,bytes);return {path,bytes:bytes.length,sha256:hash(bytes),model_hash:String(receipt.model_hash)};
}
export async function editModelFile(cad:CadClient,source:string,parameters:Record<string,number>,destination:string){
  const model=await readModelFile(cad,source);
  for(const [key,value] of Object.entries(parameters))if(!Object.hasOwn(model.parameters,key)||!Number.isFinite(value))throw Error('unknown or nonfinite parameter: '+key);
  return writeModelFile(cad,{...model,parameters:{...model.parameters,...parameters}},destination,{embedModel:true});
}
