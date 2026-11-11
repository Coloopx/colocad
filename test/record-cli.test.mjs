import { ModelingSession } from '../src/session.mjs';
import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp,readFile,rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { runShellHarnessExample } from '../examples/integrations/shell-harness.mjs';
for(const host of ['codex','claude-code'])test('H2: '+host+' skill layout records local commands and exports offline history',async t=>{
 const root=await mkdtemp(join(tmpdir(),'cad-'+host+'-'));t.after(()=>rm(root,{recursive:true,force:true}));
 const result=await runShellHarnessExample({host,project:root});
 assert.equal(result.steps,2);assert.equal(result.solids,1);
 const ledger=await new ModelingSession(join(root,'history')).read();
 assert.equal(ledger.steps[1].model,undefined);
 assert.equal(ledger.steps[1].status,'succeeded');
 assert.match(await readFile(result.html,'utf8'),/colocad-viewer/);
});
