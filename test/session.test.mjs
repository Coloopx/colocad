import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, readFile, writeFile, rm, rename, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { ModelingSession, exportSessionHtml } from '../src/session.mjs';
const model = { version: 1, units: 'mm', parameters: { length: 20 }, features: [
  { id: 'stock', op: 'box', origin: [0,0,0], size: [{param:'length'},10,6] },
], output: 'stock' };
async function root(t) { const p=await mkdtemp(join(tmpdir(),'colocad-session-'));t.after(()=>rm(p,{recursive:true,force:true}));return p; }

test('S1: immutable native history survives reopening and relocation', async t => {
  const dir=await root(t), input=structuredClone(model);
  const session=await ModelingSession.create(join(dir,'original'),{title:'A local build'});
  const result=await session.build(input,{label:'Stock',kind:'geometry'});
  assert.equal(result.ok,true,JSON.stringify(result));input.parameters.length=99;
  const ledger=await session.read();assert.equal(ledger.steps.length,1);
  assert.equal(ledger.steps[0].status,'succeeded');assert.ok(ledger.steps[0].durationMs>0);
  assert.equal(ledger.steps[0].artifactDirectory.startsWith('/'),false);
  await rename(join(dir,'original'),join(dir,'moved'));
  const reopened=await ModelingSession.open(join(dir,'moved'));
  assert.equal((await reopened.source()).parameters.length,20);
  assert.equal((await reopened.observe({view:'top'},{label:'Top inspection'})).ok,true);
  const edited=await reopened.edit({length:25},{label:'Longer'});
  assert.equal(edited.ok,true);assert.ok(Math.abs(edited.data.geometry.volume_mm3-1500)<.01);
  assert.equal((await reopened.read()).steps.length,3);
});

test('S2: failed attempt persists and edits start from last successful source', async t => {
  const s=await ModelingSession.create(await root(t));
  await s.build(model,{label:'Original'});
  const bad=await s.edit({length:-1},{label:'Invalid length'});assert.equal(bad.ok,false);
  assert.equal((await s.source()).parameters.length,20);
  assert.equal((await s.edit({length:23},{label:'Corrected'})).ok,true);
  assert.deepEqual((await s.read()).steps.map(x=>x.status),['succeeded','failed','succeeded']);
  assert.equal((await s.source()).parameters.length,23);
});

test('S3: concurrent writer refused; crashed attempt preserved as interrupted', async t => {
  const dir=await root(t),s=await ModelingSession.create(dir,{storage:'json'});
  await writeFile(join(dir,'.writer.lock'),JSON.stringify({pid:process.pid}));
  await assert.rejects(s.build(model),/SESSION_BUSY/);await rm(join(dir,'.writer.lock'));
  const ledger=await s.read();ledger.steps.push({id:'s0001',status:'running',label:'Interrupted',startedAt:new Date().toISOString()});
  await writeFile(join(dir,'session.json'),JSON.stringify(ledger));
  await writeFile(join(dir,'.writer.lock'),JSON.stringify({pid:2147483647}));
  const reopened=await ModelingSession.open(dir);
  assert.equal((await reopened.read()).steps[0].status,'interrupted');
  assert.equal((await reopened.build(model)).ok,true);
});

test('S4/S5: portable HTML contains actual meshes, escaped input, and rejects tampering', async t => {
  const dir=await root(t),s=await ModelingSession.create(join(dir,'session'),{title:'</script><script>bad()</script>'});
  await s.build(model,{label:'<img src=x onerror=bad()>'});
  const output=join(dir,'history.html');const manifest=await exportSessionHtml(s.root,output);
  assert.equal(manifest.steps,1);
  const html=await readFile(output,'utf8');assert.match(html,/<colocad-viewer/);
  assert.doesNotMatch(html,/<script>bad\(\)<\/script>/);assert.doesNotMatch(html,/<script[^>]+src=["']https?:/);
  const ledger=await s.read(),entry=ledger.steps[0];
  await writeFile(join(s.root,entry.artifactDirectory,'preview.mesh.json'),'{}');
  await assert.rejects(exportSessionHtml(s.root,join(dir,'bad.html')),/INTEGRITY/);
  entry.artifactDirectory='../outside';await writeFile(join(s.root,'session.json'),JSON.stringify(ledger));
  await assert.rejects(exportSessionHtml(s.root,join(dir,'escape.html')),/PATH/);
});
