import test from 'node:test';
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdtemp,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {once} from 'node:events';
import {ModelingSession} from '../src/session.mjs';
const cli=new URL('../bin/colocad.mjs',import.meta.url).pathname;

test('CLI launches local viewer and exports the same offline history',async t=>{
 const root=await mkdtemp(join(tmpdir(),'colocad-view-'));t.after(()=>rm(root,{recursive:true,force:true}));
 const session=await ModelingSession.create(join(root,'session'),{title:'CLI viewer test'});
 const built=await session.build({version:1,units:'mm',parameters:{},features:[{id:'body',op:'box',origin:[0,0,0],size:[12,18,6]}],output:'body'});
 assert.equal(built.ok,true);
 const html=join(root,'review.html');
 const child=spawn(process.execPath,[cli,'view',session.root,'--no-open','--port','0','--export',html],{stdio:['ignore','pipe','pipe']});
 let output='',error='';child.stdout.on('data',chunk=>output+=chunk);child.stderr.on('data',chunk=>error+=chunk);
 t.after(()=>{if(child.exitCode===null)child.kill('SIGTERM');});
 const ready=await new Promise((resolve,reject)=>{
   const timeout=setTimeout(()=>reject(Error('viewer timeout: '+error)),10000);
   child.once('exit',code=>{clearTimeout(timeout);reject(Error('viewer exited '+code+': '+error));});
   child.stdout.on('data',()=>{try{const message=JSON.parse(output.trim().split('\n')[0]);clearTimeout(timeout);resolve(message);}catch{}});
 });
 assert.equal(ready.event,'viewer-ready');assert.equal(new URL(ready.url).hostname,'127.0.0.1');
 const response=await fetch(ready.url);assert.equal(response.status,200);assert.match(response.headers.get('content-type'),/text\/html/);
 const served=await response.text();assert.match(served,/CLI viewer test/);assert.equal(served,await readFile(html,'utf8'));
 assert.equal((await fetch(new URL('/private-file',ready.url))).status,404);
 const exit=once(child,'exit');child.kill('SIGINT');assert.equal((await exit)[0],0);
});

test('CLI refuses an invalid viewer port',async()=>{
 const child=spawn(process.execPath,[cli,'view','.','--port','65536','--no-open']);let error='';child.stderr.on('data',chunk=>error+=chunk);
 const [code]=await once(child,'exit');assert.equal(code,1);assert.match(error,/port/);
});
