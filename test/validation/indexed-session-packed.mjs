// Run with the extracted npm archive on each supported target.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const [packagePath,sessionPath,mode]=process.argv.slice(2);
const pkg=resolve(packagePath), root=resolve(sessionPath);
const {ModelingSession}=await import(pathToFileURL(join(pkg,'src/session.mjs')));
if(mode!=='read-only') {
  const source={version:1,units:'mm',parameters:{length:10},features:[{
    id:'stock',op:'box',origin:[0,0,0],size:[{param:'length'},2,3],
  }],output:'stock',preview:{width:160,height:120,chord_mm:1}};
  const legacy=await ModelingSession.create(root,{storage:'json'});
  assert.equal((await legacy.build(source,{queryable:true})).ok,true);
  assert.equal((await ModelingSession.migrate(root)).steps,1);
  const session=await ModelingSession.open(root);
  const edited=await session.edit({length:20});
  assert.equal(edited.ok,true,JSON.stringify(edited.error));
  assert.ok(Math.abs(edited.data.geometry.volume_mm3-120)<1e-8);
  assert.equal((await session.edit({typo:2})).ok,false);
}
const read=new ModelingSession(root);
assert.equal((await read.latestGeometry()).id,'s0002');
assert.equal((await read.source()).parameters.length,20);
const page=JSON.parse(execFileSync(process.execPath,[join(pkg,'bin/colocad.mjs'),'history',root,'--after','1','--limit','1']));
assert.equal(page.steps[0].id,'s0002');assert.equal(page.nextCursor,2);
console.log(JSON.stringify({passed:true,platform:process.platform+'-'+process.arch,mode:mode??'write',accepted:'s0002'}));
