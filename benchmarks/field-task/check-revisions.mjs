// Verify persisted receipts and native snapshots from a fresh process, outside measurement.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {createReadStream} from 'node:fs';
import {readFile,writeFile} from 'node:fs/promises';
import {join,resolve} from 'node:path';
import {ModelingSession} from '../../src/session.mjs';
import {CadClient} from '../../kernel/dist/index.js';

const [trial,oracleFile,dest]=process.argv.slice(2);
const root=resolve(trial,'session');
const producer=JSON.parse(await readFile(join(trial,'result.json'),'utf8'));
const oracle=JSON.parse(await readFile(oracleFile,'utf8'));
assert.equal(oracle.length,256);
const session=await ModelingSession.open(root);
const client=new CadClient({outputRoot:join(root,'artifacts')});
const head=await session.head();
assert.equal(head.query_revision_id,producer.acceptedHead);
const checkpointEdits=new Set([1,32,64,128,192,256]);
let count=0,assets=0,after=0;
const snapshots=[];
async function checkFile(path,want) {
  const hash=createHash('sha256');
  for await(const bytes of createReadStream(path))hash.update(bytes);
  assert.equal(hash.digest('hex'),want,path);
}
do{
  const page=await session.history({after,limit:32});
  for(const entry of page.steps){
    count++;
    assert.equal(entry.id,'s'+String(count).padStart(4,'0'));
    assert.equal(entry.status,'succeeded');
    const receipt=await session.receipt(entry);
    assert.ok(receipt.ok);
    await checkFile(join(root,entry.inputFile),entry.inputSha256);
    const directory=join(root,entry.artifactDirectory);
    for(const [name,digest] of Object.entries(receipt.data.artifact_sha256)){
      await checkFile(join(directory,name),digest);assets++;
    }
    const edit=count-4;
    if(edit<1)continue;
    assert.equal(receipt.data.artifact_sha256['model.step'],oracle[edit-1].sha256);
    if(!checkpointEdits.has(edit))continue;
    const query={schema:'colocad.query/1',query_revision_id:receipt.data.query_revision_id,
      target:{definition_id:'service'},space:'definition',tolerance:{linear_mm:.01,angular_rad:.0001},
      queries:[{id:'camera',op:'edges',where:{curve_type:'circle',radius_mm:{min:1.699,max:1.701}},limit:8}]};
    const result=await client.query(directory,query);assert.ok(result.ok,JSON.stringify(result.error));
    const items=result.data.results[0].items;assert.equal(items.length,4);
    const xs=items.map(item=>item.center_mm[0]);
    const pitch=Math.max(...xs)-Math.min(...xs);
    assert.ok(Math.abs(pitch-(16+Math.floor((edit-1)/5)*.25))<.01);
    assert.equal(result.data.execution.geometry_build_calls,0);
    snapshots.push({edit,pitch,revision:receipt.data.query_revision_id});
  }
  after=page.nextCursor;
}while(after!==null);
assert.deepEqual(await session.head(),head);
assert.equal(count,260);assert.equal(snapshots.length,6);
await writeFile(dest,JSON.stringify({accepted:count,distinctEditedSteps:oracle.length,artifactHashesChecked:assets,oldSnapshots:snapshots,headAfterReopen:head.query_revision_id},null,2)+'\n');
