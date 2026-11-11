import { ModelingSession } from '../../src/session.mjs';
import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp,rm,readFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { runRecordedPiExample } from './recorded-pi.mjs';
test('H1: real Pi loop persists engine build, failed edit, correction and offline replay',async t=>{
 const root=await mkdtemp(join(tmpdir(),'pi-history-'));t.after(()=>rm(root,{recursive:true,force:true}));
 const result=await runRecordedPiExample(root);
 const ledger=await new ModelingSession(join(root,'session')).read();
 assert.deepEqual(ledger.steps.map(x=>x.status),['succeeded','failed','succeeded']);
 assert.equal(result.solids,17);assert.equal(result.toolErrors,1);
 assert.match(await readFile(result.html,'utf8'),/colocad-viewer/);
});
