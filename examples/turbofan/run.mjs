import { resolve, join } from 'node:path';
import { writeFile } from 'node:fs/promises';
import { ModelingSession, exportSessionHtml } from '../../src/session.mjs';
import { turbofan, stages } from './model.mjs';

const directory=resolve(process.argv[2]??'./out/turbofan');
let session;
try{session=await ModelingSession.create(join(directory,'session'),{title:'Turbofan • from feature to assembly',wallMs:120000});}
catch(error){if(error.code!=='EEXIST')throw error;session=await ModelingSession.open(join(directory,'session'),{wallMs:120000});}
for(const stage of stages){
  if((await session.read()).steps.some(s=>s.label===stage.label&&s.status==='succeeded'))continue;
  const result=await session.build(turbofan(stage.id),stage);
  console.log(JSON.stringify({stage:stage.id,ok:result.ok,solids:result.data?.geometry?.solids,step:result.session_step,error:result.error}));
  if(!result.ok)throw Error(JSON.stringify(result.error));
}
await writeFile(join(directory,'model.json'),JSON.stringify(turbofan(),null,2)+'\n');
console.log(await exportSessionHtml(session.root,join(directory,'turbofan.html'),{cutaway:true}));
