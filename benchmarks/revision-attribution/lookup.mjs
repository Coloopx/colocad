import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {dirname, resolve} from 'node:path';
import {pathToFileURL} from 'node:url';

const [runtime, directory, receiptFile, editText] = process.argv.slice(2);
const {CadClient} = await import(pathToFileURL(resolve(runtime, 'kernel/dist/index.js')));
const receipt = JSON.parse(await readFile(receiptFile,'utf8'));
const client = new CadClient({outputRoot:dirname(directory)});
const answer = await client.query(directory, {
  schema:'colocad.query/1', query_revision_id:receipt.data.query_revision_id,
  target:{definition_id:'service'}, space:'definition', tolerance:{linear_mm:.01, angular_rad:1e-4},
  queries:[{id:'camera', op:'edges', where:{curve_type:'circle', radius_mm:{min:1.699,max:1.701}},limit:8}]
});
assert.ok(answer.ok,JSON.stringify(answer.error));
assert.equal(answer.data.execution.geometry_build_calls,0);
const edges=answer.data.results[0].items;
assert.equal(edges.length,4);
const centers=edges.map(e=>e.center_mm[0]);
const pitch=Math.max(...centers)-Math.min(...centers);
assert.ok(Math.abs(pitch-(16+Math.floor((Number(editText)-1)/5)*.25))<.01);
console.log(JSON.stringify({pitch_mm:pitch,geometry_build_calls:0,revision:receipt.data.query_revision_id}));
