import {readdir,writeFile,mkdir} from 'node:fs/promises';
import {spawn} from 'node:child_process';
import {join} from 'node:path';
import assert from 'node:assert/strict';
import {resources,saveResources} from './resource.mjs';
const out=process.argv[2]??'/evidence';await mkdir(out,{recursive:true});const rows=[];
const tests=(await readdir('/repo/test')).filter(n=>n.endsWith('.test.mjs')).map(n=>'/repo/test/'+n);
for(const [id,command,args] of [['node-regression',process.execPath,['--test',...tests]],['native-parser-probes','/snapshot-probe',[]]]){
 const p=spawn(command,args,{stdio:['ignore','pipe','pipe']});let stdout='',stderr='';p.stdout.on('data',b=>stdout+=b);p.stderr.on('data',b=>stderr+=b);const timer=setTimeout(()=>p.kill('SIGKILL'),180000);const result=await new Promise(r=>p.once('exit',(code,signal)=>r({code,signal})));clearTimeout(timer);await writeFile(join(out,id+'.log'),stdout+stderr);rows.push({id,...result,resources:await resources(id)});
}
const resource=await saveResources(out,'node_regression_and_native_parser');await writeFile(join(out,'results.json'),JSON.stringify({passed:rows.every(r=>r.code===0)&&resource.memory_events.oom_kill===0,rows},null,2));assert.ok(rows.every(r=>r.code===0));console.log(JSON.stringify({passed:true,peak_bytes:resource.memory_peak_bytes}));
