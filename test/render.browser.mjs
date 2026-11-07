import assert from 'node:assert/strict';
import { mkdtemp,rm,readFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { execFileSync } from 'node:child_process';
import { ModelingSession } from '../src/session.mjs';
const root=await mkdtemp(join(tmpdir(),'colocad-film-test-'));
try{
 const s=await ModelingSession.create(join(root,'session'),{title:'Movie integration test'});
 assert.equal((await s.build({version:1,units:'mm',parameters:{},features:[{id:'stock',op:'box',origin:[0,0,0],size:[10,20,30]}],output:'stock'},{label:'Native solid'})).ok,true);
 const output=join(root,'movie.mp4');
 execFileSync(process.execPath,['bin/colocad.mjs','render',s.root,output,'--width','960','--height','600','--fps','2','--seconds-per-step','.5'],{stdio:['ignore','pipe','pipe']});
 const result=JSON.parse(await readFile(output+'.json','utf8'));assert.equal(result.frames,13);assert.deepEqual(result.browserErrors,[]);
 const probe=JSON.parse(execFileSync('ffprobe',['-v','error','-show_streams','-of','json',output],{encoding:'utf8'}));
 assert.equal(probe.streams[0].width,960);assert.equal(probe.streams[0].height,600);assert.equal(+probe.streams[0].nb_frames,13);
 assert.throws(()=>execFileSync(process.execPath,['bin/colocad.mjs','render',s.root,output],{stdio:'pipe'}));
 console.log(JSON.stringify({passed:true,frames:13,cli:true,mp4:true,sourceHash:result.steps[0].inputSha256,browserErrors:[]}));
}finally{await rm(root,{recursive:true,force:true});}
