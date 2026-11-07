// Executable local-tool example. Does not start a model or bypass host permissions.
import { mkdir,cp,readFile,writeFile,symlink } from 'node:fs/promises';
import { resolve,join,dirname } from 'node:path';
import { fileURLToPath,pathToFileURL } from 'node:url';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
const exec=promisify(execFile);
const sourcePackage=fileURLToPath(new URL('../../',import.meta.url));
export async function runShellHarnessExample({host,project,packagePath=sourcePackage}){
 if(!['codex','claude-code'].includes(host))throw Error('host must be codex or claude-code');
 project=resolve(project);packagePath=resolve(packagePath);await mkdir(project,{recursive:true});
 await writeFile(join(project,'package.json'),JSON.stringify({name:'colocad-'+host+'-example',private:true,type:'module'}));
 const installed=join(project,'node_modules/@coloopx/colocad');await mkdir(dirname(installed),{recursive:true});await symlink(packagePath,installed,'dir');
 const skill=join(project,host==='codex'?'.agents/skills/colocad':'.claude/skills/colocad');
 await cp(join(installed,'skills/colocad'),skill,{recursive:true,errorOnExist:true,force:false});
 // Exercise the actual copied helper in the consumer working directory.
 const helper=JSON.parse((await exec(process.execPath,[join(skill,'scripts/build.mjs'),join(skill,'assets/plate-grid.json'),join(project,'helper-part'),'--preview'],{cwd:project})).stdout);
 const model=JSON.parse(await readFile(join(helper.directory,'model.json'),'utf8'));
 const cli=join(installed,'bin/colocad.mjs'),history=join(project,'history'),file=join(project,'model.json');
 await writeFile(file,JSON.stringify(model));
 await exec(process.execPath,[cli,'record',file,history,'--label','Four-hole plate'],{cwd:project});
 model.parameters.holeDiameter=8;await writeFile(file,JSON.stringify(model));
 const built=JSON.parse((await exec(process.execPath,[cli,'record',file,history,'--label','Increase bore diameter'],{cwd:project})).stdout);
 const html=join(project,'history.html');
 const exported=JSON.parse((await exec(process.execPath,[cli,'replay',history,html],{cwd:project})).stdout);
 return {host,skill,html,steps:exported.steps,solids:built.data.geometry.solids,scope:'Copied skill and local tool execution; no live LLM invocation'};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){
 const [host,project]=process.argv.slice(2);if(!project)throw Error('usage: node shell-harness.mjs codex|claude-code NEW_PROJECT');
 console.log(JSON.stringify(await runShellHarnessExample({host,project}),null,2));
}
