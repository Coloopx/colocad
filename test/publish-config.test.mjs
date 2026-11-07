import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,mkdir,writeFile,readFile,stat,access,rm} from 'node:fs/promises';
import {join} from 'node:path';
import {tmpdir} from 'node:os';
import {execFileSync} from 'node:child_process';
import {publishArchive,readArchive} from '../scripts/publish-npm.mjs';
async function archive(t){const root=await mkdtemp(join(tmpdir(),'publish-test-'));t.after(()=>rm(root,{recursive:true,force:true}));await mkdir(join(root,'package'));await writeFile(join(root,'package/package.json'),JSON.stringify({name:'@coloopx/colocad',version:'0.1.0-alpha.5'}));const file=join(root,'package.tgz');execFileSync('tar',['-czf',file,'-C',root,'package']);return file;}
test('release preflight isolates npm config and does not persist the credential',async t=>{
 let config;const result=await publishArchive(await archive(t),{credential:'test-credential-value',expectedUser:'release-user',checkOnly:true,runNpm:async(args,{cwd,env})=>{
  assert.deepEqual(args,['whoami']);assert.equal(env.COLOCAD_NPM_TOKEN,'test-credential-value');config=env.NPM_CONFIG_USERCONFIG;assert.equal(env.NPM_CONFIG_REGISTRY,'https://registry.npmjs.org/');assert.ok(config.startsWith(cwd));assert.equal((await stat(config)).mode&0o777,0o600);const bytes=await readFile(config,'utf8');assert.ok(bytes.includes('${COLOCAD_NPM_TOKEN}'));assert.ok(!bytes.includes('test-credential-value'));return 'release-user\n';
 }});assert.equal(result.actor,'release-user');assert.equal(result.checkOnly,true);await assert.rejects(access(config));assert.ok(!JSON.stringify(result).includes('test-credential-value'));
});
test('wrong identity or a different immutable archive prevents publishing and tag updates',async t=>{
 const file=await archive(t),calls=[];const runNpm=async args=>{calls.push(args[0]);return args[0]==='whoami'?'unexpected-user\n':JSON.stringify('sha512-other');};
 await assert.rejects(publishArchive(file,{credential:'test',expectedUser:'release-user',runNpm}),/Unexpected npm identity/);assert.deepEqual(calls,['whoami']);calls.length=0;
 await assert.rejects(publishArchive(file,{credential:'test',runNpm}),/immutable version/);assert.deepEqual(calls,['whoami','view']);
});

test('a transient post-publish 404 retries the read, not publication',async t=>{
 const file=await archive(t),info=await readArchive(file),calls=[];let reads=0,waits=0;
 const result=await publishArchive(file,{credential:'test',latest:true,waitForRetry:async()=>{waits++;},runNpm:async args=>{
  calls.push(args.join(' '));if(args[0]==='whoami')return 'release-user';
  if(args[0]==='view'&&args[2]==='dist.integrity'){if(++reads<3)throw Object.assign(Error('not yet visible'),{stderr:'E404'});return JSON.stringify(info.integrity);}
  if(args[0]==='view')return JSON.stringify({alpha:info.version,latest:info.version});return '';
 }});
 assert.equal(result.published,true);assert.equal(waits,1);assert.equal(calls.filter(x=>x.startsWith('publish ')).length,1);assert.equal(calls.filter(x=>x.startsWith('dist-tag add ')).length,2);
});

test('resume never uploads an accepted version and reports a bounded pending state',async t=>{
 const file=await archive(t),calls=[];
 await assert.rejects(publishArchive(file,{credential:'test',resume:true,latest:true,waitForRetry:async()=>{},runNpm:async args=>{
  calls.push(args[0]);if(args[0]==='whoami')return 'release-user';throw Object.assign(Error('not visible'),{stderr:'E404'});
 }}),e=>e.code==='PUBLICATION_PENDING');
 assert.ok(!calls.includes('publish'));assert.ok(!calls.includes('dist-tag'));assert.ok(calls.length<20);
});

test('a staged conflict waits for integrity and rejects another archive before changing tags',async t=>{
 const file=await archive(t),calls=[];let reads=0;
 await assert.rejects(publishArchive(file,{credential:'test',latest:true,waitForRetry:async()=>{},runNpm:async args=>{
  calls.push(args[0]);if(args[0]==='whoami')return 'release-user';
  if(args[0]==='publish')throw Object.assign(Error('conflict'),{stderr:'E409 Cannot publish over previously staged version'});
  if(args[0]==='view'){if(++reads<3)throw Object.assign(Error('not visible'),{stderr:'E404'});return JSON.stringify('sha512-different-archive');}
  return '';
 }}),/integrity/);
 assert.equal(calls.filter(x=>x==='publish').length,1);assert.ok(!calls.includes('dist-tag'));
});
