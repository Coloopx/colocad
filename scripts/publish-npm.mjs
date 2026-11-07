#!/usr/bin/env node
// Release tooling only. Credentials stay in the child environment and are never
// written to the repository, npmrc, command arguments or printed output.
import {mkdtemp,writeFile,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
const exec=promisify(execFile);
export async function readArchive(file){
 const archive=resolve(file),{stdout}=await exec('tar',['-xOf',archive,'package/package.json'],{maxBuffer:1024*1024});
 const pkg=JSON.parse(stdout);if(pkg.name!=='@coloopx/colocad'||!/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(pkg.version))throw Error('Expected a versioned @coloopx/colocad archive');
 const integrity='sha512-'+createHash('sha512').update(await readFile(archive)).digest('base64');return {archive,name:pkg.name,version:pkg.version,integrity};
}
export async function publishArchive(file,{credential=process.env.COLOCAD_NPM_TOKEN,expectedUser,tag='alpha',latest=false,checkOnly=false,resume=false,runNpm,waitForRetry=ms=>new Promise(done=>setTimeout(done,ms))}={}){
 const info=await readArchive(file);if(!credential)throw Error('Set COLOCAD_NPM_TOKEN using your secret manager or a hidden prompt');
 if(!/^[a-z][a-z0-9-]*$/.test(tag))throw Error('Invalid dist-tag');
 const temp=await mkdtemp(join(tmpdir(),'colocad-publish-')),config=join(temp,'npmrc');
 const env={...Object.fromEntries(Object.entries(process.env).filter(([key])=>!key.toLowerCase().startsWith('npm_config_'))),COLOCAD_NPM_TOKEN:credential,NPM_CONFIG_USERCONFIG:config,NPM_CONFIG_REGISTRY:'https://registry.npmjs.org/',NPM_CONFIG_LOGS_MAX:'0'};
 const invoke=runNpm??(async(args,options)=>(await exec('npm',args,{...options,maxBuffer:1024*1024})).stdout);
 const npm=(...args)=>invoke(args,{cwd:temp,env});
 try{
  await writeFile(config,'registry=https://registry.npmjs.org/\n//registry.npmjs.org/:_authToken=${COLOCAD_NPM_TOKEN}\n',{mode:0o600});
  const actor=(await npm('whoami')).trim();if(expectedUser&&actor!==expectedUser)throw Error('Unexpected npm identity: '+actor);
  const result={...info,actor,checkOnly};if(checkOnly)return result;
  const spec=info.name+'@'+info.version;let remote;
  try{remote=JSON.parse(await npm('view',spec,'dist.integrity','--json','--prefer-online'));}
  catch(e){if(!/E404/.test(e.stderr??e.message))throw e;}
  if(remote&&remote!==info.integrity)throw Error('This immutable version already has a different archive');
  if(!remote&&!resume){
   try{await npm('publish',info.archive,'--ignore-scripts','--access','public','--tag',tag,'--loglevel=warn');}
   catch(e){if(!/previously (?:staged|published) version|EPUBLISHCONFLICT/i.test(e.stderr??e.message))throw e;}
  }
  // Publish-time scanning can hide an accepted/staged version for many minutes.
  // Only public integrity proves availability; conflicts are not proof of success.
  const delays=[1000,2000,4000,8000,16000,30000,30000,30000,30000,30000,30000];
  for(let attempt=0;;attempt++){
   try{remote=JSON.parse(await npm('view',spec,'dist.integrity','--json','--prefer-online'));break;}
   catch(e){
    if(!/E404/.test(e.stderr??e.message))throw e;
    if(attempt>=delays.length)throw Object.assign(Error('PUBLICATION_PENDING: '+spec+' is not publicly readable. It may be awaiting scanning or staged approval. Check npm stage list; use --resume to verify the same archive without uploading again.'),{code:'PUBLICATION_PENDING'});
    await waitForRetry(delays[attempt]);
   }
  }
  if(remote!==info.integrity)throw Error('Registry archive integrity does not match the tested tarball');
  await npm('dist-tag','add',spec,tag);if(latest)await npm('dist-tag','add',spec,'latest');
  return {...result,published:true,tags:JSON.parse(await npm('view',info.name,'dist-tags','--json','--prefer-online'))};
 }finally{await rm(temp,{recursive:true,force:true});}
}
if(process.argv[1]&&import.meta.url===pathToFileURL(resolve(process.argv[1])).href){
 try{const [archive,...args]=process.argv.slice(2),options={};if(!archive)throw Error('usage: node scripts/publish-npm.mjs ARCHIVE.tgz [--check-only] [--resume] [--latest] [--expected-user NAME] [--tag alpha]');
  for(let i=0;i<args.length;i++){if(args[i]==='--check-only')options.checkOnly=true;else if(args[i]==='--resume')options.resume=true;else if(args[i]==='--latest')options.latest=true;else if(args[i]==='--expected-user'||args[i]==='--tag'){const key=args[i]==='--tag'?'tag':'expectedUser',value=args[++i];if(!value||value.startsWith('--'))throw Error('Missing option value');options[key]=value;}else throw Error('Unknown option: '+args[i]);}
  console.log(JSON.stringify(await publishArchive(archive,options),null,2));
 }catch(e){const token=process.env.COLOCAD_NPM_TOKEN;const message=String(e.stderr||e.message);console.error(token?message.replaceAll(token,'[REDACTED]'):message);process.exitCode=1;}
}
