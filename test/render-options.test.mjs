import test from 'node:test';
import assert from 'node:assert/strict';
import { parseRenderOptions, filmFrames } from '../src/render.mjs';
test('R1/R2: deterministic film timing is separate from recorded build times',()=>{
 const options=parseRenderOptions(['--width','1280','--height','800','--fps','12','--seconds-per-step','1']);
 const frames=filmFrames([{status:'succeeded',durationMs:18000},{status:'failed',durationMs:8}],options);
 assert.equal(frames.length,96);assert.equal(frames[0].step,0);assert.ok(frames.some(x=>x.step===0));assert.ok(frames.some(x=>x.step===1));
});
test('R3: invalid movie options fail before rendering',()=>{
 for(const args of [['--fps','0'],['--width','1279'],['--height','NaN'],['--seconds-per-step','-1'],['--unknown']])assert.throws(()=>parseRenderOptions(args));
});
