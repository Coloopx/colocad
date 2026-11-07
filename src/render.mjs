import { readFile,writeFile,mkdir,mkdtemp,rm,copyFile,access } from 'node:fs/promises';
import { constants } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve,join,extname } from 'node:path';
import { pathToFileURL } from 'node:url';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { createHash } from 'node:crypto';
import { readSessionBundle } from './session.mjs';
export function parseRenderOptions(args){
 const options={width:1440,height:900,fps:20,secondsPerStep:1.5};
 const keys={'--width':['width',640,3840,true],'--height':['height',480,2160,true],'--fps':['fps',1,60,false],'--seconds-per-step':['secondsPerStep',.1,30,false]};
 for(let i=0;i<args.length;i++){const key=keys[args[i]];if(!key)throw Error('Unknown render option: '+args[i]);const value=Number(args[++i]);if(!Number.isFinite(value)||value<key[1]||value>key[2]||(key[0]!=='secondsPerStep'&&!Number.isInteger(value))||(key[3]&&value%2))throw Error('Invalid render option: '+key[0]);options[key[0]]=value;}
 return options;
}
export function filmFrames(steps,{fps=20,secondsPerStep=1.5}={}){
 const frames=[],last=Math.max(0,steps.findLastIndex(s=>s.status==='succeeded'));
 const add=(step,duration,phase)=>{const count=Math.max(1,Math.round(duration*fps));for(let i=0;i<count;i++)frames.push({step,phase,progress:i/Math.max(count-1,1)});};
 add(last,2,'intro');for(let i=0;i<steps.length;i++)add(i,secondsPerStep,'history');add(last,4,'assembly');return frames;
}
const escapeJson=x=>JSON.stringify(x).replaceAll('<','\\u003c').replaceAll('\u2028','\\u2028').replaceAll('\u2029','\\u2029');
async function movieHtml(bundle){
 const runtime=await readFile(new URL('../viewer/dist/standalone.js',import.meta.url),'utf8');
 // Geometry and exact downloads already live in the persisted session; movie only needs meshes/receipts.
 for(const s of bundle.steps){delete s.stepFile;delete s.png;}
 return `<!doctype html><html lang="en"><meta charset="utf-8"><style>
 *{box-sizing:border-box}body{margin:0;background:#0b1423;color:#e1eaf4;font:14px system-ui,sans-serif;overflow:hidden}main{height:100vh;padding:25px 30px;display:grid;grid-template-rows:58px 1fr 48px;gap:16px}header{display:flex;align-items:center;gap:22px}.brand{font-size:22px;letter-spacing:3px;font-weight:800}.brand span{color:#64d5c5}.title{font-size:15px;color:#9fb2c7}.tag{margin-left:auto;color:#70d5c7;font-size:11px;letter-spacing:1px}.body{display:grid;grid-template-columns:1fr 290px;gap:22px;min-height:0}.views{display:grid;grid-template-columns:1fr 1fr;grid-template-rows:1fr 1fr;gap:12px;min-height:0}.view{position:relative;border:1px solid #395166;border-radius:12px;overflow:hidden;background:#c7d5de;min-height:0}.camera{position:absolute;left:14px;top:13px;color:#244054;z-index:3;font-size:10px;letter-spacing:1.6px;font-weight:700;pointer-events:none}colocad-viewer{height:100%;min-height:0!important}.panel{padding:4px 0;display:flex;flex-direction:column;min-height:0}.eyebrow{font-size:10px;letter-spacing:2px;color:#65ccbf}.label{font-size:23px;font-weight:550;line-height:1.3;margin:12px 0 17px}.metrics{display:grid;grid-template-columns:1fr 1fr;gap:12px;margin-bottom:18px}.metric{border-top:1px solid #294057;padding-top:9px;color:#8fa6bd;font-size:9px;letter-spacing:.8px}.metric b{display:block;color:#f0f5fb;font-size:24px;letter-spacing:0;font-weight:550;margin-bottom:2px}.journal{flex:1;min-height:0;overflow:hidden}.entry{font-size:10px;color:#71889f;margin:7px 0;line-height:1.35;display:flex;gap:8px}.entry.active{color:#83e5d6}.entry.done{color:#c0cede}.number{width:18px;flex-shrink:0;font-variant-numeric:tabular-nums}.status{color:#74d4bf;font-size:11px;margin-top:12px}.note{color:#8fa6bd;font-size:10px;line-height:1.5;margin-top:8px}.bottom{display:flex;align-items:center;gap:20px;color:#92a8bf;font-size:10px;border-top:1px solid #2c4056;padding-top:12px}.progress{height:3px;background:#244055;position:absolute;bottom:0;left:0;width:100%}.fill{height:100%;background:#6dd5c5;width:0}.caption{margin-left:auto;font-size:11px;color:#c2d8e7}.stamp{color:#6fdac9}
 </style><main><header><div class="brand">COLOCAD <span>/</span></div><div class="title"></div><div class="tag">NATIVE B-REP · FOUR VIEWS</div></header><div class="body"><div class="views">${['ISOMETRIC','END / Z AXIS','SIDE / SECTION DISPLAY','TOP / Y AXIS'].map(label=>`<div class="view"><div class="camera">${label}</div><colocad-viewer></colocad-viewer></div>`).join('')}</div><aside class="panel"><div class="eyebrow">PERSISTED MODELING SESSION</div><div class="label"></div><div class="metrics"></div><div class="journal"></div><div class="status"></div><div class="note">Recorded geometry, not image generation.<br>Review pacing is edited; call times are measured.<br>Section and explosion are display controls.</div></aside></div><div class="bottom"><span class="stamp">SDK → LOCAL RUST → STEP → RECORDED MESH</span><span>Model source and receipts retained in the session</span><span class="caption"></span></div></main><div class="progress"><div class="fill"></div></div><script id="data" type="application/json">${escapeJson(bundle)}</script><script>${runtime.replaceAll('</script','<\\/script')}</script><script>
 const data=JSON.parse(document.getElementById('data').textContent),views=[...document.querySelectorAll('colocad-viewer')],cameras=['iso','front','side','top'];
 document.querySelector('.title').textContent=data.title;
 for(let i=0;i<views.length;i++){const v=views[i];v.session=data;const style=document.createElement('style');style.textContent=':host{min-height:0!important}.shell{display:block}.layout{display:block;height:100%}.stage{display:block;height:100%}.viewport{height:100%;min-height:0}header,footer,.timeline,.inspector,.stage-head,.controls,.viewbar,.watermark{display:none!important}';v.shadowRoot.append(style);v.controls.enableDamping=false;v.setProjection('orthographic');v.setSectionAxis('x');cancelAnimationFrame(v.frame);}
 let last=-1;window.filmFrame=({step,phase,progress},frame,total)=>{
 const s=data.steps[step];if(step!==last){for(const v of views)v.setStep(step);last=step;}
 const explode=phase==='assembly'?70*Math.sin(Math.PI*progress)**2:phase==='history'&&s.kind==='assembly'?30*(1-progress)**2:0;
 for(let i=0;i<views.length;i++){const v=views[i];v.cutaway=i===2;v.exploded=explode;v.applyDisplay();v.setView(cameras[i]);if(v.renderer&&v.center){v.camera.lookAt(v.center);v.resize();v.renderer.render(v.scene,v.camera);}}
 document.querySelector('.label').textContent=phase==='intro'?'A complete product, built from features':phase==='assembly'?'Inspect the finished assembly':s.label;
 const metrics=document.querySelector('.metrics');metrics.replaceChildren();for(const [name,value] of [['SOLIDS',s.geometry?.solids??'—'],['FEATURES',s.model?.features?.length??'—'],['THIS CALL',(s.durationMs/1000).toFixed(3)+' s'],['RECORDED CALLS',(data.steps.slice(0,step+1).reduce((n,x)=>n+(x.durationMs??0),0)/1000).toFixed(2)+' s']]){const d=document.createElement('div');d.className='metric';const b=document.createElement('b');b.textContent=value;d.append(b,document.createTextNode(name));metrics.append(d);}
 const journal=document.querySelector('.journal');journal.replaceChildren();const count=Math.max(2,Math.min(12,Math.floor(journal.clientHeight/30)));const start=Math.max(0,Math.min(step-Math.floor(count/2),data.steps.length-count));for(let n=start;n<Math.min(data.steps.length,start+count);n++){const d=document.createElement('div');d.className='entry '+(n===step?'active':n<step?'done':'');const num=document.createElement('span');num.className='number';num.textContent=String(n+1).padStart(2,'0');d.append(num,document.createTextNode(data.steps[n].label));journal.append(d);}
 document.querySelector('.status').textContent=s.status.toUpperCase()+' · '+s.id+' · '+s.kind;
 document.querySelector('.caption').textContent=phase==='assembly'?'Explode → reassemble · source geometry unchanged':'Saved revision '+(step+1)+' / '+data.steps.length+' · mm';document.querySelector('.fill').style.width=100*(frame+1)/total+'%';
 return views.map(v=>v.getState());};window.filmReady=views.every(v=>v.renderer);
 </script></html>`;
}
async function command(binary,args){
 const child=spawn(binary,args,{stdio:['ignore','pipe','pipe']});let out='',err='';child.stdout.on('data',x=>out+=x);child.stderr.on('data',x=>err=(err+x).slice(-6000));
 const [code]=await once(child,'close');if(code!==0)throw Error(binary+' failed: '+err);return out;
}
export async function renderSession(directory,destination,options={}){
 const settings=parseRenderOptions(Object.entries(options).flatMap(([k,v])=>[({width:'--width',height:'--height',fps:'--fps',secondsPerStep:'--seconds-per-step'})[k]??k,String(v)]));
 const output=resolve(destination),extension=extname(output).toLowerCase();if(!['.mp4','.gif'].includes(extension))throw Error('render output must be .mp4 or .gif');
 try{await access(output);throw Error('Output already exists: '+output);}catch(e){if(e.code!=='ENOENT')throw e;}
 let chromium;try{({chromium}=await import('playwright'));}catch{throw Error('Movie export needs optional Playwright: npm install -D playwright && npx playwright install chromium');}
 try{await command('ffmpeg',['-version']);}catch{throw Error('Movie export needs FFmpeg on PATH');}
 const bundle=await readSessionBundle(directory);if(!bundle.steps.length)throw Error('Session has no steps');
 const frames=filmFrames(bundle.steps,settings),temp=await mkdtemp(join(tmpdir(),'colocad-film-'));let browser,encoder;const errors=[];
 try{
  const html=join(temp,'film.html');await writeFile(html,await movieHtml(bundle));
  try{browser=await chromium.launch({headless:true});}catch{throw Error('Chromium unavailable; run npx playwright install chromium');}
  const page=await browser.newPage({viewport:{width:settings.width,height:settings.height},deviceScaleFactor:1});page.on('pageerror',e=>errors.push(e.message));await page.goto(pathToFileURL(html).href);await page.waitForFunction(()=>window.filmReady===true,{},{timeout:30000});
  const mp4=join(temp,'film.mp4');encoder=spawn('ffmpeg',['-y','-hide_banner','-loglevel','error','-f','image2pipe','-vcodec','png','-framerate',String(settings.fps),'-i','pipe:0','-an','-c:v','libx264','-preset','fast','-crf','20','-pix_fmt','yuv420p','-movflags','+faststart',mp4],{stdio:['pipe','ignore','pipe']});
  let poster;let stderr='';encoder.stderr.on('data',x=>stderr=(stderr+x).slice(-6000));encoder.stdin.on('error',()=>{});const closed=once(encoder,'close');closed.catch(()=>{});
  for(let i=0;i<frames.length;i++){await page.evaluate(({frame,i,total})=>window.filmFrame(frame,i,total),{frame:frames[i],i,total:frames.length});const png=await page.screenshot({type:'png'});if(i===0)poster=png;if(!encoder.stdin.write(png))await once(encoder.stdin,'drain');if(i%Math.max(1,settings.fps*5)===0)process.stderr.write(JSON.stringify({event:'render-progress',frame:i+1,frames:frames.length})+'\n');}
  encoder.stdin.end();const [code]=await closed;if(code!==0)throw Error('FFmpeg failed: '+stderr);if(errors.length)throw Error('Viewer errors: '+errors.join('; '));
  let rendered=mp4;
  if(extension==='.gif'){
   const palette=join(temp,'palette.png');await command('ffmpeg',['-y','-v','error','-i',mp4,'-vf','palettegen=stats_mode=diff','-frames:v','1',palette]);
   rendered=join(temp,'film.gif');await command('ffmpeg',['-y','-v','error','-i',mp4,'-i',palette,'-lavfi','paletteuse=dither=bayer:bayer_scale=4','-loop','0',rendered]);
  }
  const bytes=await readFile(rendered),metadata={format:'colocad/film/1',file:output,sha256:createHash('sha256').update(bytes).digest('hex'),bytes:bytes.length,...settings,frames:frames.length,durationSeconds:frames.length/settings.fps,reviewTiming:'edited pacing; not a benchmark',steps:bundle.steps.map(s=>({id:s.id,label:s.label,status:s.status,durationMs:s.durationMs,inputSha256:s.inputSha256,receiptSha256:s.receiptSha256})),browserErrors:errors};
  await mkdir(resolve(output,'..'),{recursive:true});await copyFile(rendered,output,constants.COPYFILE_EXCL);await writeFile(output+'.poster.png',poster);await writeFile(output+'.json',JSON.stringify(metadata,null,2)+'\n');return metadata;
 }finally{encoder?.kill('SIGTERM');await browser?.close();await rm(temp,{recursive:true,force:true});}
}
