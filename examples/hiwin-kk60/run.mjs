import { resolve, join } from 'node:path';
import { writeFile,copyFile } from 'node:fs/promises';
import { ModelingSession } from '../../src/session.mjs';
import { kk60, stages } from './model.mjs';
const directory=resolve(process.argv[2]??'./out/hiwin-kk60');
const session=await ModelingSession.create(join(directory,'session'),{title:'HIWIN KK60 • catalog-based industrial actuator reconstruction',wallMs:120000});
for(const stage of stages){
 const result=await session.build({...kk60(stage.id),preview:{chord_mm:.25,view:'iso',width:960,height:640}},stage);
 console.log(JSON.stringify({stage:stage.id,ok:result.ok,solids:result.data?.geometry?.solids,time:result.data?.timing_ms,step:result.session_step,error:result.error}));
 if(!result.ok)throw Error(JSON.stringify(result.error));
}
const model=kk60();await writeFile(join(directory,'model.json'),JSON.stringify(model,null,2)+'\n');
const last=await session.latestGeometry();await copyFile(join(session.root,last.artifactDirectory,'model.step'),join(directory,'hiwin-kk60.step'));
console.log(JSON.stringify({directory,features:model.features.length,instances:model.assembly.length}));
