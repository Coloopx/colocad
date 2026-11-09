import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';

const styles=`
:host{display:block;height:100%;min-height:680px;color:#dce6f4;font:13px/1.5 system-ui,sans-serif;background:#0b1423;--muted:#8da1ba;--border:#25334a;--accent:#61d5c9}*{box-sizing:border-box}button,select,input{font:inherit}button,select{border:1px solid var(--border);border-radius:7px;background:#152238;color:#dce6f4;padding:7px 11px;cursor:pointer}button:hover{border-color:var(--accent)}button:focus-visible,input:focus-visible,select:focus-visible{outline:2px solid var(--accent);outline-offset:2px}button[aria-pressed=true]{background:#214948;border-color:#61d5c9}.shell{height:100%;min-height:0;overflow:hidden;display:grid;grid-template-rows:65px 1fr 48px}header{display:flex;align-items:center;gap:20px;padding:0 24px;border-bottom:1px solid var(--border)}.brand{letter-spacing:3px;font-size:16px;font-weight:800;color:white}.pill{border:1px solid #3d5e6e;color:#9ad5cc;padding:3px 9px;border-radius:20px;font-size:10px;letter-spacing:1px}.title{margin-left:auto;color:var(--muted);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;max-width:52%}.layout{display:grid;grid-template-columns:248px minmax(0,1fr) 294px;min-height:0}.timeline{overflow:auto;border-right:1px solid var(--border);padding:20px 13px}.eyebrow{font-size:10px;font-weight:700;letter-spacing:1.8px;color:var(--muted);text-transform:uppercase}.timeline h2{font-size:18px;color:white;margin:7px 7px 3px}.timeline .hint{font-size:11px;color:var(--muted);margin:0 7px 18px}.step{display:block;text-align:left;width:100%;margin:6px 0;padding:10px;border:1px solid transparent;background:transparent;border-radius:9px}.step[aria-current=true]{background:#1b3545;border-color:#477578}.step b{display:block;font-size:12px;font-weight:550}.step small{color:var(--muted);font-size:10px}.dot{display:inline-block;width:6px;height:6px;border-radius:50%;background:#5ad0bd;margin-right:6px}.step.failed .dot,.step.interrupted .dot{background:#fa927f}.step .num{font-variant-numeric:tabular-nums;color:#6d849d;margin-right:8px}.stage{min-width:0;min-height:0;overflow:hidden;display:grid;grid-template-rows:auto minmax(240px,1fr) auto}.stage-head{padding:20px 22px 15px}.stage-head h1{font-size:23px;font-weight:550;margin:5px 0 7px;letter-spacing:-.5px}.metrics{display:flex;gap:16px;flex-wrap:wrap;color:var(--muted);font-size:11px}.metrics strong{color:#dce6f4;font-size:13px;font-weight:550}.viewport{position:relative;overflow:hidden;min-height:260px;background:radial-gradient(ellipse at 50% 35%,#e9eff3,#b9c8d3)}canvas{display:block;width:100%;height:100%;touch-action:none}.fallback{position:absolute;inset:0;width:100%;height:100%;object-fit:contain}.viewbar{position:absolute;top:14px;left:14px;display:flex;gap:5px}.viewbar button{color:#21374a;background:#edf3f5dd;border-color:#a6bac8;font-size:11px;padding:5px 10px}.viewbar button[aria-pressed=true]{background:#24485c;color:white}.watermark{position:absolute;bottom:14px;left:18px;color:#4c6578;font-size:10px;letter-spacing:1px;pointer-events:none}.error{position:absolute;inset:0;display:flex;align-items:center;justify-content:center;padding:30px;background:#111d2bf5;color:#ffbcad}.error div{max-width:460px}.error h2{font-size:24px}.error pre{white-space:pre-wrap}.hide{display:none!important}.controls{padding:14px 20px;display:flex;align-items:center;gap:12px;flex-wrap:wrap;border-top:1px solid var(--border)}label{display:flex;align-items:center;gap:7px;color:#b8c6d9;font-size:11px}input[type=range]{accent-color:#55cdbc;width:105px}input[type=checkbox]{accent-color:#55cdbc}.controls .note{margin-left:auto;color:var(--muted);font-size:10px}.inspector{border-left:1px solid var(--border);overflow:auto;padding:20px 15px}.tabs{display:flex;gap:4px;margin:10px 0 18px}.tabs button{font-size:11px;padding:6px 8px}.inspect-content{font-size:11px;color:#aebfd1}.part{display:flex;align-items:center;gap:7px;margin-bottom:5px;border-bottom:1px solid #1c2a3d;padding:4px 0}.part label{min-width:0;flex:1;overflow-wrap:anywhere}.part button{padding:1px 5px;font-size:10px}.swatch{width:7px;height:7px;border-radius:50%;flex-shrink:0}.feature{padding:7px 0;border-bottom:1px solid var(--border);overflow-wrap:anywhere}.feature b{color:#e1eaf5;font-weight:500}.feature small{display:block;color:#8198ad}pre{font:10px/1.6 ui-monospace,monospace;white-space:pre-wrap;overflow-wrap:anywhere}footer{padding:0 22px;border-top:1px solid var(--border);display:flex;align-items:center;gap:12px;font-size:10px;color:var(--muted)}footer .right{margin-left:auto}footer button{font-size:10px;padding:5px 9px}.status{color:#77d9bd}.status.failed,.status.interrupted{color:#ffad92}.subtle{color:var(--muted);font-size:10px}.selected{background:#294553}.total{color:var(--muted);font-size:11px;margin-top:16px}.intro{padding:30px}.summary{margin-top:14px;padding-top:14px;border-top:1px solid var(--border)}@media(max-width:1100px){.layout{grid-template-columns:215px minmax(0,1fr)}.inspector{display:none}}@media(max-width:650px){:host{min-height:800px}.shell{grid-template-rows:54px 1fr auto}header{padding:0 12px;gap:9px}.brand{font-size:13px}.pill{font-size:8px}.title{display:none}.layout{grid-template-columns:minmax(0,1fr);grid-template-rows:120px 1fr}.timeline{border-right:0;border-bottom:1px solid var(--border);padding:8px;display:flex;gap:6px;overflow:auto}.timeline-heading,.total{display:none}.step{width:170px;flex-shrink:0;margin:0}.stage-head{padding:12px}.stage-head h1{font-size:18px}.controls{padding:9px;gap:8px}.controls .note{display:none}.viewbar{top:8px;left:8px;gap:3px}.viewbar button{padding:4px 7px}footer{padding:9px;flex-wrap:wrap;gap:6px}footer .right{margin-left:0}}
`;
function el(tag,text,cls){const n=document.createElement(tag);if(text!==undefined)n.textContent=text;if(cls)n.className=cls;return n;}
function color(id){if(/end-cap|seal/.test(id))return '#344958';if(/ballscrew|bearing.*ball|guide-ball/.test(id))return '#b8c8d4';if(/rail/.test(id))return '#7897a9';if(/carriage/.test(id))return '#a2b9c5';if(/ball-nut/.test(id))return '#c9a76a';if(/spider/.test(id))return '#d68b50';if(/coupling/.test(id))return '#318dab';if(/bolt/.test(id))return '#455e73';if(/bearing|motor/.test(id))return '#8ca3b5';if(/fan/.test(id))return '#3d9fba';if(/compressor/.test(id))return '#649c8e';if(/combustor|Port|Liner/.test(id))return '#df864b';if(/turbine/.test(id))return '#c4a260';if(/shaft|Shaft/.test(id))return '#8b82b1';if(/nacelle|Casing|casing|Support/.test(id))return '#8799a8';if(/nozzle|Nozzle/.test(id))return '#9eacb3';return '#528da6';}
function download(name,bytes,type='application/json'){const url=URL.createObjectURL(new Blob([bytes],{type})),a=el('a');a.href=url;a.download=name;a.click();setTimeout(()=>URL.revokeObjectURL(url),5000);}

export class ColocadViewer extends HTMLElement{
 constructor(){super();this.attachShadow({mode:'open'});this.index=0;this.parts=[];this.geometries=new Map();this.exploded=0;this.cutaway=false;this.sectionAxis='y';this.view='iso';this.playing=false;this.tab='parts';this.selectedId=null;this.queryObjects=[];this.querySelection=null;}
 connectedCallback(){if(!this.initialized)this.init();}
 disconnectedCallback(){this.pause();cancelAnimationFrame(this.frame);this.resizeObserver?.disconnect();this.controls?.dispose();this.clearParts();for(const g of this.geometries.values())g.dispose();this.geometries.clear();this.renderer?.dispose();this.initialized=false;}
 set session(data){if(data?.format!=='colocad/viewer/1'||!Array.isArray(data.steps))throw Error('Unsupported viewer session');this.data=data;if(this.isConnected){if(!this.initialized)this.init();this.load();}}
 get session(){return this.data;}
 init(){
  this.initialized=true;this.shadowRoot.innerHTML=`<style>${styles}</style><div class="shell"><header><span class="brand">COLOCAD<span style="color:#5dd0c2"> /</span></span><span class="pill">MODELING HISTORY</span><span class="title"></span></header><div class="layout"><nav class="timeline" aria-label="Modeling steps"></nav><main class="stage"><div class="stage-head"><div class="eyebrow">RECORDED NATIVE GEOMETRY</div><h1></h1><div class="metrics"></div></div><div class="viewport"><img class="fallback hide" alt="Native CAD preview"><div class="viewbar"></div><div class="watermark">EXACT CAD → RECORDED MESH · MM</div><div class="error hide" data-testid="error"><div><h2>This attempt did not complete</h2><pre></pre></div></div></div><div class="controls"><button class="play">Play</button><label>Review speed <select aria-label="Review speed"><option value="1">1×</option><option value="2">2×</option><option value="4">4×</option></select></label><label><input type="checkbox" aria-label="Cutaway display">Cutaway</label><label>Explode <input type="range" aria-label="Exploded display" min="0" max="100" value="0"></label><span class="note">DISPLAY CONTROLS · SOURCE UNCHANGED</span></div></main><aside class="inspector"><div class="eyebrow">REVISION INSPECTOR</div><div class="tabs"></div><div class="inspect-content"></div></aside></div><footer><span class="status"></span><span class="progress"></span><span class="right">OFFLINE · ORIGINAL INPUTS + MEASURED RECEIPTS</span><button class="json">Download model JSON</button><button class="step-download">Download final STEP</button></footer></div>`;
  this.q=s=>this.shadowRoot.querySelector(s);
  this.q('.play').onclick=()=>this.playing?this.pause():this.play();
  this.q('[aria-label="Cutaway display"]').onchange=e=>{this.cutaway=e.target.checked;this.applyDisplay();};
  this.q('[aria-label="Exploded display"]').oninput=e=>{this.exploded=+e.target.value;this.applyDisplay();};
  this.q('.json').onclick=()=>{const step=this.data.steps[this.index];download(step.id+'-model.json',JSON.stringify(step.model,null,2));};
  this.q('.step-download').onclick=()=>{const s=this.data.steps.findLast(s=>s.stepFile);if(s)download('final-model.step',Uint8Array.from(atob(s.stepFile),c=>c.charCodeAt(0)),'application/step');};
  for(const view of ['iso','front','side','top']){const b=el('button',view[0].toUpperCase()+view.slice(1));b.onclick=()=>this.setView(view);b.dataset.view=view;this.q('.viewbar').append(b);}
  for(const tab of ['parts','query','features','source','receipt']){const b=el('button',tab[0].toUpperCase()+tab.slice(1));b.onclick=()=>{this.tab=tab;this.inspect();};b.dataset.tab=tab;this.q('.tabs').append(b);}
  try{
    this.renderer=new THREE.WebGLRenderer({antialias:true,alpha:true,preserveDrawingBuffer:true});this.renderer.setPixelRatio(Math.min(devicePixelRatio,2));this.renderer.localClippingEnabled=true;this.renderer.outputColorSpace=THREE.SRGBColorSpace;
    this.q('.viewport').prepend(this.renderer.domElement);this.scene=new THREE.Scene();this.camera=new THREE.PerspectiveCamera(36,1,.1,100000);this.camera.up.set(0,1,0);
    this.scene.add(new THREE.HemisphereLight(0xffffff,0x6d7c90,2.8));const key=new THREE.DirectionalLight(0xffffff,3);key.position.set(1,2,-1);this.scene.add(key);const rim=new THREE.DirectionalLight(0xb9dfff,1.2);rim.position.set(-1,0,2);this.scene.add(rim);
    this.group=new THREE.Group();this.scene.add(this.group);this.controls=new OrbitControls(this.camera,this.renderer.domElement);this.controls.enableDamping=true;
    this.renderer.domElement.addEventListener('pointerup',e=>{if(this.down&&Math.hypot(e.clientX-this.down[0],e.clientY-this.down[1])<4)this.pick(e);});this.renderer.domElement.addEventListener('pointerdown',e=>{this.down=[e.clientX,e.clientY];});
    this.resizeObserver=new ResizeObserver(()=>this.resize());this.resizeObserver.observe(this.q('.viewport'));
    const tick=()=>{this.controls.update();this.renderer.render(this.scene,this.camera);this.frame=requestAnimationFrame(tick);};tick();
  }catch(error){this.renderer=null;this.q('.fallback').classList.remove('hide');this.q('.watermark').textContent='NATIVE PNG FALLBACK · WEBGL UNAVAILABLE';}
  if(this.data)this.load();
 }
 load(){
  this.cutaway=this.data.display?.cutaway??false;this.view=this.data.display?.view??'iso';this.q('[aria-label="Cutaway display"]').checked=this.cutaway;
  this.pause();this.q('.title').textContent=this.data.title;const nav=this.q('.timeline');nav.replaceChildren();
  const head=el('div',undefined,'timeline-heading');head.append(el('div','BUILD JOURNAL','eyebrow'),el('h2','From feature to form'),el('p',this.data.steps.length+' persisted attempts','hint'));nav.append(head);
  this.data.steps.forEach((step,i)=>{const b=el('button',undefined,'step '+step.status);b.dataset.index=i;b.onclick=()=>{this.pause();this.setStep(i);};const title=el('b');title.append(el('span',String(i+1).padStart(2,'0'),'num'),document.createTextNode(step.label));const meta=el('small');meta.append(el('i',undefined,'dot'),document.createTextNode(step.kind+' · '+(step.durationMs??0).toFixed(0)+' ms'));b.append(title,meta);nav.append(b);});
  const total=this.data.steps.reduce((n,s)=>n+(s.durationMs??0),0);nav.append(el('div','Recorded calls: '+(total/1000).toFixed(2)+' s\nPlayback is a review, not a benchmark.','total'));
  this.q('.step-download').disabled=!this.data.steps.some(s=>s.stepFile);this.setStep(Math.max(0,this.data.steps.findLastIndex(s=>s.status==='succeeded')));
 }
 clearParts(){this.clearQuery();for(const p of this.parts)p.material.dispose();this.group?.clear();this.parts=[];}
 setStep(index){
  if(!this.data?.steps.length)return;this.index=Math.max(0,Math.min(index,this.data.steps.length-1));const step=this.data.steps[this.index];this.clearParts();this.selectedId=null;
  this.q('h1').textContent=step.label;this.q('.metrics').replaceChildren();
  for(const [label,value] of [['SOLIDS',step.geometry?.solids??'—'],['FEATURES',step.model?.features?.length??'—'],['CALL TIME',((step.durationMs??0)/1000).toFixed(3)+' s']]){const item=el('span');item.append(el('strong',String(value)),document.createTextNode(' '+label));this.q('.metrics').append(item);}
  this.q('.status').textContent=step.status.toUpperCase()+(step.degradations?.length?' · DEGRADED':'');this.q('.status').className='status '+step.status;this.q('.progress').textContent=`${this.index+1} / ${this.data.steps.length}`;
  this.shadowRoot.querySelectorAll('.step').forEach(b=>b.setAttribute('aria-current',String(+b.dataset.index===this.index)));
  this.q('.error').classList.toggle('hide',step.status==='succeeded');this.q('.error pre').textContent=step.error?JSON.stringify(step.error,null,2):step.status;
  if(step.png)this.q('.fallback').src=step.png;else this.q('.fallback').removeAttribute('src');
  if(this.renderer&&step.scene){for(const instance of step.scene.instances){
    const key=step.scene.definitions[instance.definition],mesh=this.data.meshes[key];let geometry=this.geometries.get(key);
    if(!geometry){geometry=new THREE.BufferGeometry();geometry.setAttribute('position',new THREE.Float32BufferAttribute(mesh.positions.flat(),3));geometry.setIndex(mesh.triangles.flat());if(mesh.normals?.length===mesh.positions.length)geometry.setAttribute('normal',new THREE.Float32BufferAttribute(mesh.normals.flat(),3));else geometry.computeVertexNormals();geometry.computeBoundingBox();this.geometries.set(key,geometry);}
    const material=new THREE.MeshStandardMaterial({color:color(instance.id),metalness:.42,roughness:.48,side:THREE.DoubleSide});const part=new THREE.Mesh(geometry,material);part.name=instance.id;part.userData.definition=instance.definition;
    const [x,y,z]=instance.axes,origin=instance.origin;const matrix=new THREE.Matrix4().set(x[0],y[0],z[0],origin[0],x[1],y[1],z[1],origin[1],x[2],y[2],z[2],origin[2],0,0,0,1);matrix.decompose(part.position,part.quaternion,part.scale);
    part.userData.sourceMatrix=matrix.clone();part.userData.origin=part.position.clone();part.userData.center=geometry.boundingBox.getCenter(new THREE.Vector3()).applyMatrix4(matrix);this.group.add(part);this.parts.push(part);
  }
  this.bounds=new THREE.Box3().setFromObject(this.group);this.center=this.bounds.getCenter(new THREE.Vector3());this.size=this.bounds.getSize(new THREE.Vector3()).length()||10;this.applyDisplay();this.setView(this.view);}
  if(step.operation==='query'&&step.receipt?.ok){this.tab='query';this.highlightQuery(0,0);}
  else if(this.tab==='query')this.tab='parts';
  this.inspect();this.dispatchEvent(new CustomEvent('stepchange',{detail:{index:this.index,id:step.id,status:step.status},bubbles:true}));
 }
 resize(){if(!this.renderer)return;const box=this.q('.viewport').getBoundingClientRect();this.renderer.setSize(box.width,box.height,false);this.camera.aspect=box.width/Math.max(box.height,1);if(this.camera.isOrthographicCamera)this.fitOrthographic();this.camera.updateProjectionMatrix();}
 setProjection(mode){
  if(!['perspective','orthographic'].includes(mode))throw Error('Unknown camera projection');
  if(!this.renderer)return;
  if(mode==='orthographic'&&!this.camera.isOrthographicCamera)this.camera=new THREE.OrthographicCamera(-1,1,1,-1,.01,100000);
  if(mode==='perspective'&&!this.camera.isPerspectiveCamera)this.camera=new THREE.PerspectiveCamera(36,1,.01,100000);
  this.controls.object=this.camera;this.setView(this.view);
 }
 fitOrthographic(){
  if(!this.center||!this.parts.length)return;
  this.camera.lookAt(this.center);this.camera.updateMatrixWorld();
  const b=new THREE.Box3().setFromObject(this.group);let horizontal=0,vertical=0;
  for(const x of [b.min.x,b.max.x])for(const y of [b.min.y,b.max.y])for(const z of [b.min.z,b.max.z]){const point=new THREE.Vector3(x,y,z).applyMatrix4(this.camera.matrixWorldInverse);horizontal=Math.max(horizontal,Math.abs(point.x));vertical=Math.max(vertical,Math.abs(point.y));}
  const half=Math.max(vertical,horizontal/(this.camera.aspect||1),.1)*1.12;
  this.camera.top=half;this.camera.bottom=-half;this.camera.left=-half*(this.camera.aspect||1);this.camera.right=half*(this.camera.aspect||1);
 }
 setSectionAxis(axis){if(!['x','y','z'].includes(axis))throw Error('Unknown section axis');this.sectionAxis=axis;this.applyDisplay();}
 setView(view){this.view=view;this.shadowRoot.querySelectorAll('[data-view]').forEach(b=>b.setAttribute('aria-pressed',String(b.dataset.view===view)));if(!this.renderer||!this.center)return;
  const dirs={iso:[1,.7,-1.05],front:[0,0,-1],side:[1,0,0],top:[0,1,0]};this.camera.up.set(0,view==='top'?0:1,view==='top'?1:0);const distance=this.size*1.65;this.camera.position.copy(this.center).add(new THREE.Vector3(...dirs[view]).normalize().multiplyScalar(distance));this.controls.target.copy(this.center);this.camera.near=Math.max(.01,this.size/10000);this.camera.far=this.size*100;this.camera.updateProjectionMatrix();this.controls.update();this.resize();
 }
 applyDisplay(){if(!this.renderer||!this.center)return;const normal=new THREE.Vector3(0,0,0);normal[this.sectionAxis]=-1;const clip=new THREE.Plane(normal,this.center[this.sectionAxis]);
  for(const part of this.parts){const delta=part.userData.center.clone().sub(this.center).multiplyScalar(this.exploded/100*.8);part.position.copy(part.userData.origin).add(delta);part.material.clippingPlanes=this.cutaway?[clip]:[];part.material.needsUpdate=true;}
 }
 inspect(){if(!this.data)return;const step=this.data.steps[this.index],out=this.q('.inspect-content');out.replaceChildren();this.shadowRoot.querySelectorAll('[data-tab]').forEach(b=>b.setAttribute('aria-pressed',String(b.dataset.tab===this.tab)));
  if(this.tab==='parts'){
    out.append(el('p',this.parts.length+' named occurrences · click to highlight','subtle'));
    for(const part of this.parts){const row=el('div',undefined,'part'+(part.name===this.selectedId?' selected':'')),label=el('label'),check=el('input');check.type='checkbox';check.checked=part.visible;check.setAttribute('aria-label','Show '+part.name);check.onchange=()=>{part.visible=check.checked;};const dot=el('span',undefined,'swatch');dot.style.background=color(part.name);label.append(check,dot,document.createTextNode(part.name));const focus=el('button','↗');focus.setAttribute('aria-label','Select '+part.name);focus.onclick=()=>this.select(part.name);row.append(label,focus);out.append(row);}
    if(step.degradations?.length)out.append(el('pre',JSON.stringify(step.degradations,null,2)));
  }else if(this.tab==='query'){
    if(step.operation!=='query')out.append(el('p','Record a query to inspect native references.','subtle'));
    else{
      out.append(el('p','Native-bound outlines and datum guides. Display controls do not change query coordinates.','subtle'));
      for(const [qi,result]of(step.receipt?.data?.results??[]).entries()){
        out.append(el('p',result.id+' · '+result.op+' · '+result.status));
        for(const [ii,item]of(result.items??[]).entries()){
          const b=el('button',item.id??item.ref?.topology_id??item.kind??'Result '+(ii+1));b.onclick=()=>this.highlightQuery(qi,ii);b.setAttribute('aria-label','Highlight '+result.id+' '+(ii+1));out.append(b);
        }
        if(result.measurement)out.append(el('pre',JSON.stringify(result.measurement,null,2)));
      }
      if(this.querySelection)out.append(el('pre',JSON.stringify(this.querySelection,null,2)));
      if(this.queryWarning)out.append(el('p',this.queryWarning,'subtle'));
    }
  }else if(this.tab==='features'){
    for(const f of step.model?.features??[]){const item=el('div',undefined,'feature');const trace=step.receipt?.data?.features?.find(x=>x.id===f.id);item.append(el('b',f.id),el('small',f.op+(trace?' · '+Number(trace.elapsed_ms??0).toFixed(2)+' ms'+(trace.cache_hit?' · reused':''):'')));out.append(item);}
  }else out.append(el('pre',JSON.stringify(this.tab==='source'?step.model:step.receipt??step.error,null,2)));
 }
 clearQuery(){for(const o of this.queryObjects??[]){o.removeFromParent();o.geometry?.dispose();if(Array.isArray(o.material))o.material.forEach(m=>m.dispose());else o.material?.dispose();}this.queryObjects=[];this.querySelection=null;this.queryWarning=null;}
 highlightQuery(queryIndex,itemIndex){
  this.clearQuery();const step=this.data?.steps[this.index],receipt=step?.receipt?.data,item=receipt?.results?.[queryIndex]?.items?.[itemIndex],guide=step?.queryDisplay;
  if(!this.renderer||!item)return;
  if(!guide||guide.query_revision_id!==receipt.query_revision_id){this.queryWarning='No matching native display references for this revision.';this.inspect();return;}
  const refs=[];const walk=v=>{if(!v||typeof v!=='object')return;if(v.topology_id&&v.query_revision_id){refs.push(v);return;}if(Array.isArray(v)){v.forEach(walk);return;}for(const [k,x]of Object.entries(v))if(!['excluded_members','adjacent_faces','incident_faces','incident_face_uses','requirements','symmetry'].includes(k))walk(x);};walk(item);
  const unique=[...new Map(refs.map(r=>[JSON.stringify(r),r])).values()];this.querySelection=item.ref??unique[0]??null;
  const line=(part,points,color=0xffad33)=>{if(points.length<2)return;const geometry=new THREE.BufferGeometry().setFromPoints(points.map(p=>new THREE.Vector3(...p))),material=new THREE.LineBasicMaterial({color,depthTest:false,transparent:true,opacity:.95});const o=new THREE.Line(geometry,material);o.renderOrder=20;part.add(o);this.queryObjects.push(o);};
  const dot=(part,point)=>{const geometry=new THREE.BufferGeometry().setFromPoints([new THREE.Vector3(...point)]),material=new THREE.PointsMaterial({color:0xffad33,size:7,sizeAttenuation:false,depthTest:false});const o=new THREE.Points(geometry,material);o.renderOrder=21;part.add(o);this.queryObjects.push(o);};
  const eligible=(part,ref)=>part.userData.definition===ref.definition_id&&(!ref.instance_path||JSON.stringify(guide.occurrences?.[part.name]?.instance_path)===JSON.stringify(ref.instance_path));
  for(const part of this.parts){const drawn=new Set();for(const ref of unique){
    if(ref.query_revision_id!==guide.query_revision_id||ref.definition_sha256!==guide.definition_digests[ref.definition_id]||!eligible(part,ref))continue;
    const definition=guide.definitions[ref.definition_id];if(!definition)continue;
    const edges=ref.kind==='edge'?[ref.topology_id]:ref.kind==='face'?definition.faces[ref.topology_id]?.edges:ref.kind==='loop'?definition.loops[ref.topology_id]?.edges:[];
    for(const id of edges??[]){if(!drawn.has(id)&&definition.edges[id]){drawn.add(id);line(part,definition.edges[id].points_mm);}}
    if(ref.kind==='vertex'&&definition.vertices[ref.topology_id])dot(part,definition.vertices[ref.topology_id].point_mm);
  }
  if(unique.some(r=>eligible(part,r))){
    const inv=part.userData.sourceMatrix.clone().invert(),convert=p=>receipt.space==='assembly'?new THREE.Vector3(...p).applyMatrix4(inv).toArray():p;
    const axis=item.kind==='axis'?item.geometry:item.axis;
    if(axis?.point_mm&&axis.direction){const o=axis.point_mm,d=axis.direction,L=Math.max(this.size*.15,2);line(part,[convert(o.map((v,i)=>v-d[i]*L)),convert(o.map((v,i)=>v+d[i]*L))],0xff54c7);}
    const group=item.group??(item.members?item:null);if(group){for(const member of group.members??[])if(member.center_mm)dot(part,convert(member.center_mm));}
    const p=item.geometry?.point_mm??item.origin_mm??item.center_mm;if(p)dot(part,convert(p));
  }}
  if(!this.queryObjects.length)this.queryWarning='This result has no drawable native topology references.';
  this.inspect();this.dispatchEvent(new CustomEvent('queryselect',{detail:{queryIndex,itemIndex,ref:this.querySelection},bubbles:true}));
 }
 select(id){this.selectedId=id;for(const part of this.parts)part.material.emissive.set(part.name===id?0x294c66:0x000000);this.inspect();this.dispatchEvent(new CustomEvent('partselect',{detail:{id},bubbles:true}));}
 pick(event){const box=this.renderer.domElement.getBoundingClientRect(),pointer=new THREE.Vector2((event.clientX-box.left)/box.width*2-1,-(event.clientY-box.top)/box.height*2+1),ray=new THREE.Raycaster();ray.setFromCamera(pointer,this.camera);const hits=ray.intersectObjects(this.parts.filter(p=>p.visible)).filter(h=>this.parts.includes(h.object)&&(!this.cutaway||h.point[this.sectionAxis]<=this.center[this.sectionAxis]));if(hits.length)this.select(hits[0].object.name);}
 play(){this.playing=true;this.q('.play').textContent='Pause';if(this.index===this.data.steps.length-1)this.setStep(0);const advance=()=>{if(!this.playing)return;if(this.index>=this.data.steps.length-1){this.pause();return;}this.setStep(this.index+1);this.timer=setTimeout(advance,1500/+this.q('select').value);};this.timer=setTimeout(advance,1500/+this.q('select').value);}
 pause(){this.playing=false;clearTimeout(this.timer);if(this.q)this.q('.play').textContent='Play';}
 getState(){return {stepIndex:this.index,status:this.data?.steps[this.index]?.status,parts:this.parts.length,visibleParts:this.parts.filter(p=>p.visible).length,view:this.view,exploded:this.exploded,cutaway:this.cutaway,playing:this.playing,selectedId:this.selectedId,webgl:!!this.renderer,queryHighlights:this.queryObjects?.length??0,querySelection:this.querySelection};}
}
if(!customElements.get('colocad-viewer'))customElements.define('colocad-viewer',ColocadViewer);
