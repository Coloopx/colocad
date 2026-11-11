// Execute the exact public Markdown example and inspect packaged guidance.
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {resolve,join} from 'node:path';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
assert.ok(!process.env.AGENT_CAD_BINARY);
const pkg=resolve('node_modules/@coloopx/colocad'),out=resolve('documentation-check');
const files=['README.md','README.zh-CN.md','ROADMAP.md','ROADMAP.zh-CN.md',
  'docs/benchmark-alpha9.md','docs/benchmark-alpha9.zh-CN.md','docs/benchmarks/alpha9/scorecard.html','docs/benchmarks/alpha9/scorecard.svg','docs/benchmarks/alpha9/observations.json',
  'docs/cadquery-comparison.md','docs/cadquery-comparison.zh-CN.md','docs/cadquery-coverage.json',
  'docs/best-practices.md','docs/best-practices.zh-CN.md','docs/agent-modeling-handbook.md','docs/agent-modeling-handbook.zh-CN.md',
  'skills/colocad/SKILL.md','skills/colocad/references/tuning.md','skills/colocad/references/modeling-loop.md','skills/colocad/references/prompt-patterns.md'];
const digests={};
for(const f of files){const b=await readFile(join(pkg,f));assert.ok(b.length);digests[f]=createHash('sha256').update(b).digest('hex');}
const md=await readFile(join(pkg,'docs/best-practices.md'),'utf8');
const matches=[...md.matchAll(/```js\n([\s\S]*?)\n```/g)];assert.equal(matches.length,1);
await mkdir(out);const code=matches[0][1]+'\n';await writeFile(join(out,'query-workflow.mjs'),code);
await writeFile(join(out,'example.log'),execFileSync(process.execPath,['query-workflow.mjs'],{cwd:out,encoding:'utf8'}));
const replay=JSON.parse(execFileSync(process.execPath,[join(pkg,'bin/colocad.mjs'),'replay','query-session','query-review.html'],{cwd:out,encoding:'utf8'}));
assert.equal(replay.steps,5);assert.equal(replay.meshDefinitions,2);
const summary={passed:true,platform:process.platform+'-'+process.arch,files:digests,script_sha256:createHash('sha256').update(code).digest('hex'),replay:{steps:replay.steps,meshDefinitions:replay.meshDefinitions,bytes:replay.bytes}};
await writeFile(join(out,'summary.json'),JSON.stringify(summary,null,2)+'\n');console.log(JSON.stringify({passed:true,files:files.length,script_sha256:summary.script_sha256}));
