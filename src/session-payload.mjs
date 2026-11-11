// A single catalog writer appends one independent gzip member per query receipt.
// fsync data and directory before committing its address. Uncommitted tails are
// unreachable after a crash; subsequent members never overwrite a committed one.
import {constants} from 'node:fs';
import {mkdir,open,realpath,rename,rm} from 'node:fs/promises';
import {join} from 'node:path';
import {randomUUID,createHash} from 'node:crypto';
import {gzipSync,gunzipSync} from 'node:zlib';

const limit=32*1024*1024, nofollow=constants.O_NOFOLLOW??0;
async function sync(dir){const f=await open(dir,'r');try{await f.sync();}finally{await f.close();}}
async function directory(root) {
  const base=await realpath(root), dir=join(base,'inspections');
  try { await mkdir(dir); await sync(base); } catch(e){if(e.code!=='EEXIST')throw e;}
  if(await realpath(dir)!==dir) throw Error('SESSION_PATH: inspection directory is a symlink');
  return dir;
}

export async function storeRequest(root,bytes,digest) {
  const dir=await directory(root), name='request-'+digest+'.json', path=join(dir,name);
  try {
    const f=await open(path,constants.O_RDONLY|nofollow);
    try { if((await f.stat()).size!==Buffer.byteLength(bytes) || await f.readFile('utf8')!==bytes) throw Error('SESSION_INTEGRITY: '+name); }
    finally {await f.close();}
  } catch(e) {
    if(e.code!=='ENOENT')throw e;
    const temp=join(dir,'.request-'+randomUUID());
    const f=await open(temp,'wx',0o600);
    try {try{await f.writeFile(bytes);await f.sync();}finally{await f.close();}await rename(temp,path);}
    finally{await rm(temp,{force:true});}
  }
  // Also cover a retry after a prior writer was killed between rename and directory sync.
  await sync(dir);
  return 'inspections/'+name;
}

export async function storeReceipt(root,id,bytes) {
  if(Buffer.byteLength(bytes)>limit)return null; // retain the ordinary file path for large receipts
  const dir=await directory(root), name='receipts-'+Math.floor((Number(id.slice(1))-1)/128)+'.gz';
  const data=gzipSync(bytes,{level:1});
  const f=await open(join(dir,name),constants.O_APPEND|constants.O_CREAT|constants.O_WRONLY|nofollow,0o600);
  try {
    const {size:offset}=await f.stat();
    // No nonempty segment is reachable before its name is durable. After a crash
    // an empty orphan follows the same rule on retry.
    if(!offset)await sync(dir);
    await f.writeFile(data); await f.sync();
    return {file:'inspections/'+name,offset,length:data.length};
  } finally {await f.close();}
}

export async function readPacked(root,ref,expected) {
  if(!ref || !/^inspections\/receipts-\d+\.gz$/.test(ref.file) ||
    !Number.isSafeInteger(ref.offset)||ref.offset<0||!Number.isSafeInteger(ref.length)||
    ref.length<1||ref.length>limit+65536||!Number.isSafeInteger(ref.offset+ref.length)||
    !/^[a-f0-9]{64}$/.test(expected))throw Error('SESSION_INTEGRITY: invalid receipt extent');
  const base=await realpath(root), path=join(base,ref.file);
  if(await realpath(path)!==path)throw Error('SESSION_PATH: receipt pack symlink');
  const f=await open(path,constants.O_RDONLY|nofollow);
  try {
    if((await f.stat()).size<ref.offset+ref.length)throw Error('SESSION_INTEGRITY: truncated receipt');
    const data=Buffer.allocUnsafe(ref.length);
    for(let n=0;n<data.length;){const r=await f.read(data,n,data.length-n,ref.offset+n);if(!r.bytesRead)throw Error('SESSION_INTEGRITY: truncated receipt');n+=r.bytesRead;}
    let bytes;try{bytes=gunzipSync(data,{maxOutputLength:limit});}catch{throw Error('SESSION_INTEGRITY: invalid receipt gzip');}
    if(createHash('sha256').update(bytes).digest('hex')!==expected)throw Error('SESSION_INTEGRITY: receipt checksum');
    return JSON.parse(bytes);
  } finally{await f.close();}
}
