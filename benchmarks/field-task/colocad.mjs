// The same measured mounting circles drive every order. No face numbers or per-fixture placements.
import assert from 'node:assert/strict';
import {readFile,writeFile,copyFile,mkdir} from 'node:fs/promises';
import {join,resolve} from 'node:path';
import {CadClient,attachStep} from '../../kernel/dist/index.js';
import {ModelingSession} from '../../src/session.mjs';
const [inputs,out,historyText='0',editText='0']=process.argv.slice(2),n=Number(historyText),edits=Number(editText);
assert.ok(!(n&&edits));
await mkdir(out,{recursive:true});const orders=JSON.parse(await readFile(new URL('orders.json',import.meta.url)));
const tol={linear_mm:.01,angular_rad:1e-4}, p=param=>({param}), mul=(a,b)=>({op:'mul',a,b}),add=(a,b)=>({op:'add',a,b});
const cad=new CadClient({outputRoot:resolve(out,'imports')});
let prior,revision,diagnostics=[],receipts=[];
const session=await ModelingSession.create(resolve(out,'session'));
for(const o of orders){
  const changed=o.source!==prior?.source;
  let measured=prior?.measured,body=prior?.body;
  if(changed){
    body=await readFile(join(inputs,o.source+'.step'),'utf8');
    const source=await cad.build(attachStep({version:1,units:'mm',parameters:{},features:[{id:'carriage',op:'import_step',asset:'source',body_index:0}],output:'carriage'},'source',body),{queryable:true});
    assert.ok(source.ok,JSON.stringify(source.error));
    const q=await cad.query(source.data.artifact_directory,{schema:'colocad.query/1',query_revision_id:source.data.query_revision_id,target:{definition_id:'carriage'},space:'definition',tolerance:tol,queries:[{id:'semantic',op:'holes',where:{termination:'blind'}},{id:'rings',op:'edges',where:{curve_type:'circle',radius_mm:{min:2.499,max:2.501}},limit:64}]});
    assert.ok(q.ok,JSON.stringify(q.error));const [h,r]=q.data.results;
    const circles=r.items.filter(c=>c.closed&&Math.abs(c.axis_direction[1])>.9999);
    const top=Math.max(...circles.map(c=>c.center_mm[1]));
    const ring=circles.filter(c=>Math.abs(c.center_mm[1]-top)<.01).map(c=>c.center_mm);
    diagnostics.push({source:o.source,blindStatus:h.status,recognizedBlind:h.items.length,mountCircles:ring.length});
    if(ring.length!==4){receipts.push({id:o.id,reason:'MOUNT_CARDINALITY',observed:ring.length});continue;}
    const xs=ring.map(c=>c[0]),zs=ring.map(c=>c[2]);const extent=v=>Math.max(...v)-Math.min(...v);
    assert.ok(Math.abs(extent(xs)-30)<.01&&Math.abs(extent(zs)-30)<.01);
    measured={x:xs.reduce((a,b)=>a+b)/4,z:zs.reduce((a,b)=>a+b)/4,y:top,positions:ring.map(([x,,z])=>[x,z])};
  }
  const {x,y,z,positions}=measured;
  const params={length:o.length,gauge:o.gauge,cameraPitch:o.cameraPitch};
  const frame={origin:[0,add(y+1,p('gauge')),0],x_axis:[1,0,0],y_axis:[0,0,1]};
  const holes=(id,input,positions,radius)=>({id,op:'holes',input,positions,radius,depth:add(p('gauge'),2),frame});
  let features=[{id:'carriage',op:'import_step',asset:'source',body_index:0},{id:'plate',op:'box',origin:[x-22,y,add(z,mul(-.5,p('length')))],size:[44,p('gauge'),p('length')]},holes('mount','plate',positions,2.75),holes('camera','mount',[-.5,.5].map(f=>[add(x,mul(f,p('cameraPitch'))),z]),1.7)];
  if(o.window)features.push({id:'cable',op:'box',origin:[x-4,y-1,z+5],size:[8,add(p('gauge'),2),4]},{id:'service',op:'cut',left:'camera',right:'cable'});
  const model=attachStep({version:1,units:'mm',parameters:params,features,assembly:[{id:'saddle',input:'carriage'},{id:'adapter',input:features.at(-1).id}],preview:{width:160,height:120,chord_mm:1}},'source',body);
  const built=o.id==='resize'?await session.edit(params):await session.build(model,{queryable:true});
  assert.ok(built.ok,JSON.stringify(built.error));revision=built.data.query_revision_id;
  const path=join(out,o.id+'.step');await copyFile(join(built.data.artifact_directory,'model.step'),path);
  receipts.push({id:o.id,placed:[x,y,z],step:o.id+'.step',revision});prior={source:o.source,measured,body};
}
assert.deepEqual(receipts.map(r=>r.reason??'STEP'),['STEP','STEP','STEP','MOUNT_CARDINALITY','STEP']);
const checkpoints=[],editSteps=[];
if(n||edits){
  const cg=f=>readFile('/sys/fs/cgroup/'+f,'utf8');
  const snapshot=async i=>{
    const current=Number(await cg('memory.current')),peak=Number(await cg('memory.peak'));
    const stat=Object.fromEntries((await cg('memory.stat')).trim().split('\n').map(s=>s.split(' ')));
    checkpoints.push({[edits?'edits':'queries']:i,current,peak,...Object.fromEntries(['anon','file','kernel','file_dirty','file_writeback','shmem','file_mapped','active_file','inactive_file','slab','sock','kernel_stack'].map(k=>[k,Number(stat[k])])),heapUsed:process.memoryUsage().heapUsed,revision});
  };
  await snapshot(0);
  for(let i=1;i<=n;i++){
    const r=await session.query({schema:'colocad.query/1',query_revision_id:revision,target:{definition_id:'service'},space:'definition',tolerance:tol,queries:[{id:'camera',op:'edges',where:{curve_type:'circle',radius_mm:{min:1.699,max:1.701}},limit:8}]});
    assert.ok(r.ok,JSON.stringify(r.error));assert.equal(r.data.execution.geometry_build_calls,0);
    assert.equal(r.data.results[0].items.length,4);
    if(i%128===0||i===n||i===1)await snapshot(i);
  }
  for(let i=1;i<=edits;i++){
    // Distinct configurations; fixed feature count, hole count, part count and active source size class.
    const gauge=5+((i-1)%5)*.5,cameraPitch=16+Math.floor((i-1)/5)*.25;
    const r=await session.edit({gauge,cameraPitch});assert.ok(r.ok,JSON.stringify(r.error));
    assert.equal(r.data.geometry.solids,2);revision=r.data.query_revision_id;
    editSteps.push({i,id:r.session_step,gauge,cameraPitch,revision,sha256:r.data.artifact_sha256['model.step']});
    if(i===1||i%4===0||i===edits)await snapshot(i);
  }
  await writeFile(join(out,'cgroup-events.json'),JSON.stringify({
    limit:Number(await cg('memory.max')),swap:Number(await cg('memory.swap.max')),
    events:Object.fromEntries((await cg('memory.events')).trim().split('\n').map(line=>line.split(' ')))
  }));
  const reopened=await ModelingSession.open(resolve(out,'session'));assert.equal((await reopened.head()).query_revision_id,revision);
  let count=0,after=0;
  do{const page=await reopened.history({after,limit:128});
    for(const e of page.steps){assert.equal(e.status,'succeeded');assert.equal(e.id,'s'+String(++count).padStart(4,'0'));}
    after=page.nextCursor;
  }while(after!==null);
  assert.equal(count,n+edits+4);assert.equal((await reopened.receipt((await reopened.history({after:n+edits+3,limit:1})).steps[0])).data.query_revision_id,revision);
}
await writeFile(join(out,'result.json'),JSON.stringify({ok:true,receipts,diagnostics,checkpoints,historyQueries:n,historyEdits:edits,editSteps,acceptedHead:revision}));
