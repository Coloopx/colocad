// The local helper owns the SQLite connection for one SDK operation. Closing
// its request pipe also releases the OS writer lock after a terminated caller.
import { spawn } from 'node:child_process';
import { open, readFile, writeFile, link, rm } from 'node:fs/promises';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';
import { join } from 'node:path';
import { randomUUID } from 'node:crypto';

const binary = fileURLToPath(new URL(
  `../kernel/native/${process.platform}-${process.arch}/colocad-catalog`, import.meta.url,
));

export async function catalogName(root) {
  const manifest = await open(join(root, 'session.json'));
  try {
    // The v2 descriptor contains only a format and a generated filename.
    if ((await manifest.stat()).size > 256) return null;
    const data = JSON.parse(await manifest.readFile('utf8'));
    if (data.format === 'colocad/session/1') return null;
    if (!['colocad/session/2','colocad/session/3'].includes(data.format) || !/^session-[a-f0-9-]{36}\.sqlite$/.test(data.catalog))
      throw Error('SESSION_FORMAT: invalid catalog descriptor');
    return data.catalog;
  } finally { await manifest.close(); }
}

async function connect(root, mode, name, callback) {
  const child = spawn(binary, [root, mode, name], {stdio:['pipe','pipe','pipe']});
  const responses = createInterface({input:child.stdout, crlfDelay:Infinity})[Symbol.asyncIterator]();
  let diagnostic = '';
  let failure;
  child.on('error', error => { failure=error; });
  child.stderr.on('data', chunk => { diagnostic=(diagnostic+chunk).slice(-2048); });
  child.stdin.on('error', error => { failure=error; });
  const closed = new Promise(resolve => child.once('close', resolve));
  async function receive() {
    const {value, done} = await responses.next();
    if (done) throw failure ?? Error('SESSION_CATALOG: helper closed the connection '+diagnostic);
    const result = JSON.parse(value);
    if (!result.ok) {
      const error=Error(result.error);
      if(result.error.startsWith('EEXIST:')) error.code='EEXIST';
      throw error;
    }
    return result.data;
  }
  async function send(command) {
    await new Promise((resolve,reject) => child.stdin.write(JSON.stringify(command)+'\n', err=>err?reject(err):resolve()));
    return receive();
  }
  try { return await callback({receive,send,pid:child.pid}); }
  finally { child.stdin.end(); await closed; }
}

// Alpha.8 clients honor this PID file. The Rust process owns both the OS
// lock and the PID recorded here, including during a long streaming import.
async function legacyLock(root,pid,action) {
  const path=join(root,'.writer.lock');
  try {
    const owner=JSON.parse(await readFile(path,'utf8'));
    if(!Number.isInteger(owner.pid) || owner.pid<1) throw Error('SESSION_BUSY: unknown writer');
    try { process.kill(owner.pid,0); }
    catch(error) { if(error.code!=='ESRCH') throw error; await rm(path); }
  } catch(error) { if(error.code!=='ENOENT') throw error; }
  const temp=path+'.'+randomUUID();
  await writeFile(temp,JSON.stringify({pid}),{flag:'wx'});
  try { await link(temp,path); }
  catch(error) {
    if(error.code==='EEXIST') throw Error('SESSION_BUSY: another local writer owns this session');
    throw error;
  } finally { await rm(temp,{force:true}); }
  try { return await action(); } finally { await rm(path,{force:true}); }
}

export function withLegacyWriter(root,action) {
  return connect(root,'guard','-',async ({receive,pid})=>{
    await receive();
    return legacyLock(root,pid,action);
  });
}

export function initializeCatalog(root,mode,header={}) {
  return connect(root,mode,`session-${randomUUID()}.sqlite`,async ({receive,send,pid})=>{
    await receive();
    return legacyLock(root,pid,()=>send(header));
  });
}

export function withCatalog(root,name,writable,callback) {
  return connect(root,writable?'write':'read',name, async ({receive,send}) => {
    await receive();
    if(writable) await send({op:'recover',now:new Date().toISOString()});
    return callback({
      header:()=>send({op:'header'}),
      head:()=>send({op:'head'}),
      length:()=>send({op:'length'}),
      page:(after,limit)=>send({op:'page',after,limit}),
      begin:step=>send({op:'begin',step}),
      finish:step=>send({op:'finish',step}),
    });
  });
}
