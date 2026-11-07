// Browser contract: run after `npm run build:viewer`. Uses real native meshes.
import { chromium } from 'playwright';
import { ModelingSession,exportSessionHtml } from '../src/session.mjs';
import { mkdtemp,rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import assert from 'node:assert/strict';
const root=await mkdtemp(join(tmpdir(),'cad-browser-'));
const browser=await chromium.launch({headless:true});
try{
 const model={version:1,units:'mm',parameters:{length:20},features:[{id:'stock',op:'box',origin:[0,0,0],size:[{param:'length'},10,6]}],assembly:[{id:'part-one',input:'stock'},{id:'part-two',input:'stock',translation:[30,0,0]}]};
 const session=await ModelingSession.create(join(root,'session'),{title:'<img src=x onerror="window.BAD=true">'});
 assert.equal((await session.build(model,{label:'Two parts'})).ok,true);
 assert.equal((await session.edit({length:-5},{label:'Failed change'})).ok,false);
 assert.equal((await session.edit({length:25},{label:'Accepted change'})).ok,true);
 const html=join(root,'viewer.html');await exportSessionHtml(session.root,html);
 const page=await browser.newPage({viewport:{width:1440,height:960}});
 const errors=[],remote=[];page.on('pageerror',e=>errors.push(e.message));page.on('request',r=>{if(/^https?:/.test(r.url()))remote.push(r.url());});
 await page.goto(pathToFileURL(html).href);
 await page.waitForFunction(()=>document.querySelector('colocad-viewer')?.getState().parts===2);
 assert.equal(await page.evaluate(()=>document.documentElement.scrollHeight<=innerHeight),true);
 const state=()=>page.locator('colocad-viewer').evaluate(el=>el.getState());
 assert.equal((await state()).stepIndex,2);
 const position=()=>page.locator('colocad-viewer').evaluate(el=>el.parts[0].position.toArray());
 const before=await position();
 await page.getByRole('button',{name:'Front',exact:true}).click();assert.equal((await state()).view,'front');
 await page.getByLabel('Exploded display').fill('65');assert.equal((await state()).exploded,65);assert.notDeepEqual(await position(),before);
 await page.getByLabel('Cutaway display').check();assert.equal((await state()).cutaway,true);
 await page.getByRole('button',{name:'Parts',exact:true}).click();
 await page.getByLabel('Show part-one', {exact:true}).uncheck();assert.equal((await state()).visibleParts,1);
 await page.getByRole('button',{name:/Failed change/}).click();
 assert.equal((await state()).status,'failed');assert.equal((await state()).parts,0);
 assert.ok(await page.getByTestId('error').isVisible());
 await page.getByRole('button',{name:/Accepted change/}).click();
 await page.getByRole('button',{name:'Play',exact:true}).click();assert.equal((await state()).playing,true);
 await page.getByRole('button',{name:'Pause',exact:true}).click();assert.equal((await state()).playing,false);
 await page.getByRole('button',{name:/Accepted change/}).click();
 const download=page.waitForEvent('download');await page.getByRole('button',{name:'Download model JSON',exact:true}).click();assert.equal((await download).suggestedFilename(),'s0003-model.json');
 await page.reload();await page.waitForFunction(()=>document.querySelector('colocad-viewer')?.getState().parts===2);
 await page.setViewportSize({width:390,height:844});
 assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
 assert.equal(await page.evaluate(()=>window.BAD),undefined);
 await page.setViewportSize({width:1440,height:960});
 const framing=await page.locator('colocad-viewer').evaluate(v=>{v.exploded=0;v.applyDisplay();v.setProjection('orthographic');return ['iso','front','side','top'].map(view=>{v.setView(view);v.camera.updateMatrixWorld();let extent=0;for(const x of [v.bounds.min.x,v.bounds.max.x])for(const y of [v.bounds.min.y,v.bounds.max.y])for(const z of [v.bounds.min.z,v.bounds.max.z]){const point=v.center.clone().set(x,y,z).project(v.camera);extent=Math.max(extent,Math.abs(point.x),Math.abs(point.y));}return {view,extent,orthographic:v.camera.isOrthographicCamera};});});
 for(const f of framing){assert.equal(f.orthographic,true);assert.ok(f.extent<=.9,JSON.stringify(f));}
 assert.deepEqual(errors,[]);assert.deepEqual(remote,[]);
 console.log(JSON.stringify({passed:true,offline:true,controls:true,failedStep:true,download:true,mobile:true,errors,remoteRequests:remote.length}));
}finally{await browser.close();await rm(root,{recursive:true,force:true});}
