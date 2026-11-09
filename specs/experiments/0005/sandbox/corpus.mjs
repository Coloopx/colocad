// Native CAD with a minimal Node process supervisor, inside the same limited cgroup.
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {spawn} from 'node:child_process';
import {createHash} from 'node:crypto';
import {join} from 'node:path';
import assert from 'node:assert/strict';
import {saveResources,resources} from './resource.mjs';
const root='/repo/specs/experiments/0005/corpus-v1',out=process.argv[2]??'/evidence',manifest=JSON.parse(await readFile(join(root,'manifest.json'))),rows=[];await mkdir(out,{recursive:true});const start=await resources('native_plus_minimal_node_supervisor_start');
const hash=b=>createHash('sha256').update(b).digest('hex');
for(const c of manifest.cases){
 const input=await readFile(join(root,c.model));assert.equal(hash(input),c.model_sha256);const started=performance.now(),directory=join(out,c.id);
 const p=spawn(process.env.AGENT_CAD_BINARY,['build','--out',directory,'--queryable'],{stdio:['pipe','pipe','pipe']});let stdout='',stderr='';p.stdout.on('data',b=>stdout+=b);p.stderr.on('data',b=>stderr+=b);p.stdin.end(input);const timer=setTimeout(()=>p.kill('SIGKILL'),180000);const exit=await new Promise(r=>p.once('exit',(code,signal)=>r({code,signal})));clearTimeout(timer);
 await writeFile(join(out,c.id+'-stdout.json'),stdout);await writeFile(join(out,c.id+'-stderr.log'),stderr);
 let receipt;try{receipt=JSON.parse(stdout);}catch{}
 const passed=c.expect==='success'?receipt?.ok===true:receipt?.ok===false&&c.error.codes.includes(receipt.error.code)&&(!c.error.feature_id||receipt.error.feature_id===c.error.feature_id);
 const row={id:c.id,family:c.family,expect:c.expect,passed,exit,wall_seconds:(performance.now()-started)/1000,error:receipt?.error,resources:await resources(c.id)};rows.push(row);await writeFile(join(out,'results.json'),JSON.stringify({manifest_sha256:hash(await readFile(join(root,'manifest.json'))),native_sha256:hash(await readFile(process.env.AGENT_CAD_BINARY)),passed:rows.length===manifest.cases.length&&rows.every(r=>r.passed),rows},null,2));console.log(JSON.stringify({id:row.id,passed,wall_seconds:row.wall_seconds,peak_bytes:row.resources.memory_peak_bytes}));
}
const final=await saveResources(out,'native_plus_minimal_node_supervisor');await writeFile(join(out,'start-resources.json'),JSON.stringify(start,null,2));assert.equal(rows.every(r=>r.passed),true);assert.equal(final.memory_events.oom_kill,0);assert.equal(final.memory_max,512*1024*1024);assert.equal(final.cpu_max,'100000 100000');
