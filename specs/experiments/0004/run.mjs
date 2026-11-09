// Reproducible M0 SDK/native experiment. Explicit dev binary prevents stale-package tests.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {mkdir,readFile,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {dirname,join,relative,resolve} from 'node:path';
import {CadClient} from '../../../kernel/dist/index.js';
const root=fileURLToPath(new URL('../../../',import.meta.url));
const output=resolve(process.argv[2]??join(root,'out/experiments/0004'));
const binary=process.env.AGENT_CAD_BINARY;
assert.ok(binary,'Set AGENT_CAD_BINARY to the native built from this checkout');
const sha=b=>createHash('sha256').update(b).digest('hex');
function git(...args){const p=spawnSync('git',args,{cwd:root,encoding:'utf8'});assert.equal(p.status,0,p.stderr);return p.stdout.trim();}
await mkdir(output,{recursive:true});
const source=JSON.parse(await readFile(new URL('./shaft-support.json',import.meta.url),'utf8'));
const cad=new CadClient({outputRoot:join(output,'builds'),binary});
const built=await cad.build(source);assert.equal(built.ok,true,JSON.stringify(built));
const solution=built.data.assembly.mate_solution;
assert.equal(solution.degrees_of_freedom,1);
for(const c of solution.constraints){assert.ok(c.physical_residual.linear_mm<1e-6);assert.ok(c.physical_residual.angular_rad<1e-6);}
const negative=await cad.edit(source,{shaft_radius:3.2});assert.equal(negative.ok,false);assert.equal(negative.error.code,'ASSEMBLY_COLLISION');
const sourceFiles=git('ls-files','--cached','--others','--exclude-standard','crates/agent-cad','kernel/src','Cargo.toml','Cargo.lock','scripts/schema.mjs').split('\n').sort();
const sourceSha256={};for(const path of sourceFiles)sourceSha256[path]=sha(await readFile(join(root,path)));
const report={
 schema:'colocad.sdd0004.experiment/1',date:new Date().toISOString(),
 scope:'M0 local SDK/native direction smoke; explicit numeric attachments, not geometry-backed datums or query acceptance',
 environment:{platform:process.platform,arch:process.arch,node:process.version},
 baselineCommit:git('rev-parse','HEAD'),dirty:git('status','--porcelain').length>0,
 sourceSha256,nativeSha256:sha(await readFile(binary)),kernel:built.data.kernel,
 fixtureSha256:sha(await readFile(new URL('./shaft-support.json',import.meta.url))),
 artifactDirectory:relative(output,built.data.artifact_directory),
 stepSha256:sha(await readFile(join(built.data.artifact_directory,'model.step'))),
 result:{passed:true,degreesOfFreedom:solution.degrees_of_freedom,iterations:solution.iterations,constraints:solution.constraints,placements:solution.placements},
 oversizedShaft:{passed:true,errorCode:negative.error.code},
 independentReadback:'pending; run verify-step.py with an OCCT environment'
};
await writeFile(join(output,'run.json'),JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({passed:true,report:join(output,'run.json'),step:join(built.data.artifact_directory,'model.step')}));
