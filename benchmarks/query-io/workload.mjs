// Run in the memory benchmark image. /pkg is the candidate or baseline package.
import assert from 'node:assert/strict';
import {readFile, writeFile, stat} from 'node:fs/promises';
import {join} from 'node:path';
import {createHash} from 'node:crypto';
import {ModelingSession} from '/pkg/src/session.mjs';
import {attachStep} from '/pkg/kernel/dist/index.js';

const [mode, directory, arg] = process.argv.slice(2);
const digest = value => createHash('sha256').update(value).digest('hex');
if (mode === 'prepare') {
  const source = await readFile(arg, 'utf8');
  const session = await ModelingSession.create(directory);
  const model = attachStep({version:1,units:'mm',parameters:{},features:[
    {op:'import_step',id:'plate',asset:'foreign',body_index:0},
  ],output:'plate',preview:{width:160,height:120,chord_mm:1}},'foreign',source);
  const receipt = await session.build(model,{queryable:true});
  assert.equal(receipt.ok,true,JSON.stringify(receipt.error));
  await writeFile(join(directory,'expected.json'),JSON.stringify({
    source_sha256:digest(source), revision:receipt.data.query_revision_id,
    volume_mm3:receipt.data.geometry.volume_mm3,
  }));
} else if(mode === 'measure') {
  const count = Number(arg);
  assert.ok(Number.isSafeInteger(count) && count>0);
  const session = new ModelingSession(directory);
  const expected = JSON.parse(await readFile(join(directory,'expected.json')));
  const request = {schema:'colocad.query/1',query_revision_id:expected.revision,
    target:{definition_id:'plate'},space:'definition',tolerance:{linear_mm:1e-5,angular_rad:1e-7},
    queries:[{id:'top',op:'faces',where:{planar_normal_aligned_with:[0,0,1]},limit:16}]};
  let resultHash;
  for(let i=0;i<count;i++) {
    const receipt = await session.query(request);
    assert.equal(receipt.ok,true,JSON.stringify(receipt.error));
    const row = receipt.data.results[0];
    assert.equal(row.status,'complete');
    assert.ok(row.items.length>0);
    const currentHash=digest(JSON.stringify(row));
    if(i) assert.equal(currentHash,resultHash);
    resultHash=currentHash;
    for(const op of ['geometry_build_calls','step_export_calls','tessellation_calls']) {
      assert.equal(receipt.data.execution[op],0);
    }
  }
  const ledger=await session.read();
  assert.equal(ledger.steps.length,count+1);
  assert.equal((await session.latestGeometry()).id,ledger.steps[0].id);
  const inputs=new Set(ledger.steps.map(s=>s.inputFile));
  let sourceBytes=0;
  for(const input of inputs) sourceBytes+=(await stat(join(directory,input))).size;
  await writeFile('/out/result.json',JSON.stringify({ok:true,count,resultHash,...expected,distinctInputPaths:inputs.size,sourceBytes}));
} else throw Error('expected prepare or measure');
