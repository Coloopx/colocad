import assert from 'node:assert/strict';
import {execFileSync,spawnSync} from 'node:child_process';
import {readFile,writeFile,rename,stat} from 'node:fs/promises';
import {join,resolve} from 'node:path';
import {pathToFileURL} from 'node:url';

// Exercise the actual package executable and SDK on each supported platform.
export async function checkSessionCli(pkg,root) {
  const {ModelingSession}=await import(pathToFileURL(join(pkg,'src/session.mjs')));
  const binary=join(pkg,'bin/colocad.mjs');
  const cli=(...args)=>JSON.parse(execFileSync(process.execPath,[binary,...args],{encoding:'utf8'}));
  const input=join(pkg,'skills/colocad/assets/plate-grid.json');
  let project=join(root,'project');
  await ModelingSession.create(project);
  assert.equal(cli('head',project),null);
  const first=cli('record',input,project,'--queryable');
  const original=cli('head',project);
  assert.equal(original.session_step,first.session_step);
  assert.equal(original.query_revision_id,first.data.query_revision_id);
  const req=join(root,'inspect.json'),params=join(root,'edit.json');
  const inspect=async (head,n,diameter)=>{
    await writeFile(req,JSON.stringify({schema:'colocad.query/1',query_revision_id:head.query_revision_id,
      target:{definition_id:'mounts'},space:'definition',tolerance:{linear_mm:1e-5,angular_rad:1e-7},
      queries:[{id:'bores',op:'holes',where:{termination:'through'}},
               {id:'up',op:'faces',where:{planar_normal_aligned_with:[0,0,1]},limit:16}]}));
    const r=cli('inspect',project,req);
    const [bores,planes]=r.data.results;
    assert.equal(bores.status,'complete');
    assert.equal(bores.items.length,n);
    for(const hole of bores.items) assert.ok(Math.abs(hole.segments[0].diameter_mm-diameter)<1e-5);
    assert.ok(planes.items.length);
    assert.equal(r.data.execution.geometry_build_calls,0);
    const [step]=(await new ModelingSession(project).history({after:Number(r.session_step.slice(1))-1})).steps;
    assert.equal(step.inputFile,'steps/'+head.session_step+'/input.json');
    assert.ok((await stat(join(project,step.queryRequestFile))).isFile());
  };
  await inspect(original,4,6);
  await rename(project,join(root,'moved')); project=join(root,'moved');
  await writeFile(params,JSON.stringify({length:100,xCount:3,xPitch:32,holeDiameter:8}));
  const edited=cli('edit',project,params), head=cli('head',project);
  assert.equal(head.session_step,edited.session_step);
  assert.notEqual(original.query_revision_id,head.query_revision_id);
  const stale=spawnSync(process.execPath,[binary,'inspect',project,req],{encoding:'utf8'});
  assert.equal(stale.status,1);
  assert.equal(JSON.parse(stale.stderr).error.code,'QUERY_REVISION_MISMATCH');
  await inspect(head,6,8);
  assert.equal(cli('head',project).session_step,edited.session_step);
  const session=new ModelingSession(project);
  const {steps}=await session.history();
  assert.deepEqual(steps.map(s=>s.status),['succeeded','succeeded','succeeded','failed','succeeded']);
  assert.deepEqual(steps.map(s=>s.inputFile),[steps[0].inputFile,steps[0].inputFile,steps[2].inputFile,steps[2].inputFile,steps[2].inputFile]);
  const acceptedInput=join(project,steps[2].inputFile);
  await writeFile(acceptedInput,(await readFile(acceptedInput,'utf8'))+' ');
  await assert.rejects(session.query(JSON.parse(await readFile(req,'utf8'))),/SESSION_INTEGRITY/);
  assert.equal((await session.read()).steps.length,5);
  return {ok:true,platform:process.platform+'-'+process.arch,steps:steps.length,holesBefore:4,holesAfter:6};
}

if(process.argv[1] && import.meta.url===pathToFileURL(resolve(process.argv[1])).href) {
  console.log(JSON.stringify(await checkSessionCli(resolve(process.argv[2]),resolve(process.argv[3]))));
}
