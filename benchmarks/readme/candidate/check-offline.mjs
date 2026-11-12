import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {chromium} from 'playwright';

const browser=await chromium.launch();
try {
  const page=await browser.newPage({viewport:{width:1280,height:800}});
  const errors=[],network=[];
  page.on('pageerror',e=>errors.push(e.message));
  page.on('request',req=>{if(/^https?:/.test(req.url()))network.push(req.url());});
  await page.context().setOffline(true);
  await page.goto(pathToFileURL(resolve(process.argv[2])).href);
  const rows=page.locator('#measurements tbody tr');
  assert.equal(await rows.count(),69);
  await page.locator('#filter').fill('part/1024');
  assert.equal(await rows.filter({visible:true}).count(),3);
  const options=await page.locator('#shape option').evaluateAll(els=>els.map(x=>x.value));assert.equal(options.length,6);
  for(const value of options) {
    await page.selectOption('#shape',value);
    assert.equal(await page.locator('[data-shape]:visible img').count(),2);
  }
  const images=await page.locator('img').evaluateAll(async els=>{
    await Promise.all(els.map(e=>e.decode()));
    return els.map(e=>e.naturalWidth);
  });
  assert.ok(images.every(n=>n>0));
  async function download(id) {
    const wait=page.waitForEvent('download');
    await page.locator(id).click();
    return JSON.parse(await readFile(await (await wait).path(),'utf8'));
  }
  const primary=await download('#download');assert.equal(primary.observations.length,207);
  const field=await download('#workdata');
  assert.equal(Object.keys(field.sessions).length,4);
  for(const run of Object.values(field.sessions))assert.equal(run.edits.length,256);
  await page.setViewportSize({width:390,height:844});
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth),390);
  assert.deepEqual(errors,[]);assert.deepEqual(network,[]);
  console.log(JSON.stringify({version:primary.version,rows:69,filteredRows:3,shapePairs:6,decodedImages:images.length,
    downloadedEdits:1024,mobileWidth:390,remoteRequests:0,browserErrors:0}));
} finally { await browser.close(); }
