// Access a cold directory through its catalog locator. Hold the catalog read
// transaction while opening loose files so a concurrent edit cannot retire them.
import {constants} from 'node:fs';
import {open,realpath,readdir} from 'node:fs/promises';
import {join,dirname,basename} from 'node:path';
import {createHash} from 'node:crypto';
import {ResourceLimitError} from '../kernel/dist/resource-limits.js';

const integrity=()=>Error('SESSION_INTEGRITY: invalid geometry segment');
const valid=n=>Number.isSafeInteger(n)&&n>=0;
const plain=name=>typeof name==='string' && /^[^/\\\0]+$/.test(name) && name!=='.' && name!=='..';
async function local(root,path) {
  const base=await realpath(root),actual=join(base,path);
  if(!path || path.split('/').some(name=>!plain(name)) || actual!==await realpath(actual))throw Error('SESSION_PATH: expected a session-owned file');
  return actual;
}
async function regular(root,path) {
  const file=await open(await local(root,path),constants.O_RDONLY|constants.O_NOFOLLOW|constants.O_NONBLOCK);
  try {
    if(!(await file.stat()).isFile())throw Error('SESSION_PATH: expected a regular file');
    return file;
  } catch(error){await file.close();throw error;}
}
async function readSpan(file,offset,length) {
  const bytes=Buffer.allocUnsafe(length);
  for(let n=0;n<length;){
    const r=await file.read(bytes,n,length-n,offset+n);
    if(!r.bytesRead)throw integrity();
    n+=r.bytesRead;
  }
  return bytes;
}
async function directory(root,store,dir,action) {
  const loc=await store.lookup?.(dir);
  if(!loc)return action(null,null);
  if(!valid(loc.segment)||!valid(loc.offset)||!valid(loc.length)||loc.length>16*1024*1024)throw integrity();
  const file=await regular(root,'segments/history-'+loc.segment+'.pack');
  try {
    if((await file.stat()).size<loc.offset+loc.length)throw integrity();
    const index=await readSpan(file,loc.offset,loc.length);
    if(createHash('sha256').update(index).digest('hex')!==loc.sha256)throw integrity();
    const members=JSON.parse(index);
    for(const [name,item] of Object.entries(members)){
      if(!plain(name)||!valid(item.offset)||!valid(item.length)||item.offset+item.length>loc.offset||!/^[0-9a-f]{64}$/.test(item.sha256))throw integrity();
    }
    return await action(file,members);
  } finally {await file.close();}
}
export async function readMember(root,store,path,expected,limit,resource='reportBytes') {
  return directory(root,store,dirname(path),async(pack,members)=>{
    const name=basename(path);
    const member=members && Object.hasOwn(members,name)?members[name]:undefined;
    if(pack&&!member)throw integrity();
    const f=pack??await regular(root,path);
    try {
      const size=member?.length??(await f.stat()).size;
      if(size>limit)throw new ResourceLimitError(resource,size,limit);
      const bytes=await readSpan(f,member?.offset??0,size);
      const digest=createHash('sha256').update(bytes).digest('hex');
      if((expected&&expected!==digest)||(member&&member.sha256!==digest))throw Error('SESSION_INTEGRITY: '+path);
      return bytes;
    } finally {if(!pack)await f.close();}
  });
}

export async function copyDirectory(root,store,dir,dest,hashes,limits) {
  return directory(root,store,dir,async(pack,members)=>{
    const names=members?Object.keys(members):await readdir(await local(root,dir));
    for(const name of Object.keys(hashes))if(!names.includes(name))throw integrity();
    for(const name of names){
      if(!plain(name))throw integrity();
      const member=members?.[name],file=pack??await regular(root,join(dir,name));
      try {
        const size=member?.length??(await file.stat()).size;
        const resource=name==='model.step'?'outputStepBytes':name==='report.json'?'reportBytes':null;
        if(resource&&size>limits[resource])throw new ResourceLimitError(resource,size,limits[resource]);
        const out=await open(join(dest,name),'wx',0o600),hash=createHash('sha256');
        try {
          const buffer=Buffer.allocUnsafe(Math.min(65536,size));
          for(let n=0;n<size;){
            const {bytesRead}=await file.read(buffer,0,Math.min(buffer.length,size-n),(member?.offset??0)+n);
            if(!bytesRead)throw integrity();
            const chunk=buffer.subarray(0,bytesRead);
            await out.writeFile(chunk);hash.update(chunk);n+=bytesRead;
          }
          const digest=hash.digest('hex');
          if((member&&digest!==member.sha256)||(hashes[name]&&digest!==hashes[name]))throw integrity();
        } finally {await out.close();}
      } finally {if(!pack)await file.close();}
    }
  });
}
