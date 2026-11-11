import {mkdir, mkdtemp, readFile, realpath, readdir, rename, rm, writeFile} from 'node:fs/promises';
import {dirname, extname, isAbsolute, join, relative, resolve, sep} from 'node:path';
import {Matrix4,Vector3} from 'three';
import initManifold from 'manifold-3d/manifold';
import {binarySTL,fromManifold,materialDefault,meshSolid,readMesh,transformMesh,writeGlb} from './hybrid-mesh.mjs';
import {checkBrep,exactChunks,exactFeature,overlapVolume,placeBrep,stepBytes,vec} from './hybrid-brep.mjs';
import {writeViewerHtml} from './session.mjs';
import {combineStep,writeFacetedStep} from './hybrid-step.mjs';

let meshReady;
function meshKernel() {
  meshReady??=initManifold().then(api=>{api.setup();return api;});
  return meshReady;
}

function validId(id) {
  if(typeof id!=='string'||!(/^[a-zA-Z0-9][a-zA-Z0-9_-]{0,95}$/).test(id)) throw Error('Invalid feature or instance ID');
  return id;
}

function get(map,id) {
  if(!map.has(id)) throw Error('Unknown reference '+id);
  return map.get(id);
}

function placement(item) {
  const translation=vec(item.translate??[0,0,0],'translate'), rotation=item.rotate;
  const matrix=new Matrix4().makeTranslation(...translation);
  if(rotation) {
    const center=vec(rotation.center??[0,0,0],'rotation center'), axis=new Vector3(...vec(rotation.axis,'rotation axis'));
    if(!Number.isFinite(rotation.degrees)||axis.lengthSq()<1e-18) throw Error('Invalid axis rotation');
    matrix.multiply(new Matrix4().makeTranslation(...center))
      .multiply(new Matrix4().makeRotationAxis(axis.normalize(),rotation.degrees*Math.PI/180))
      .multiply(new Matrix4().makeTranslation(...center.map(x=>-x)));
    return {matrix,translation,rotation:{...rotation,center,axis:axis.toArray()}};
  }
  return {matrix,translation};
}

function viewerMesh(chunk) {
  const base64=arr=>Buffer.from(arr.buffer,arr.byteOffset,arr.byteLength).toString('base64');
  return {positionsF32:base64(chunk.positions),indicesU32:base64(chunk.indices)};
}

async function evaluate(spec,asset,stage) {
  if(spec.format!=='colocad/hybrid/1'||spec.units!=='mm'||!spec.features?.length||!spec.instances?.length)
    throw Error('Expected colocad/hybrid/1, mm, features and instances');
  const started=performance.now();
  const tolerance=spec.chordToleranceMm??.05;
  if(!(Number.isFinite(tolerance)&&tolerance>0)) throw Error('Invalid mesh chord tolerance');
  const definitions=new Map(), parts=new Map(), owned=[];
  const keep=x=>{owned.push(x);return x;};
  const receipt={format:spec.format,units:'mm',chordToleranceMm:tolerance,features:[],instances:[],checks:[]};
  const bundle={format:'colocad/viewer/1',title:spec.title??'Hybrid assembly',steps:[],meshes:{}};
  const scene={instances:[],definitions:{}};
  const chunksOf = def => def.chunks??=exactChunks(def.exact,tolerance);
  const solidOf = async def => def.solid??=keep(meshSolid(await meshKernel(),chunksOf(def)));
  const placedSolid=async p=>{
    if(!p.solid) {
      const source=await solidOf(p.def);
      p.solid=p.matrix.equals(new Matrix4())?source:keep(source.transform(p.matrix.elements));
    }
    return p.solid;
  };
  try {
    for(const f of spec.features) {
      validId(f.id);
      if(definitions.has(f.id)) throw Error('Duplicate feature '+f.id);
      const def={}, featureStart=performance.now();
      try {
        if(f.op==='import_mesh') {
          def.chunks=await readMesh(await asset(f.path),extname(f.path).slice(1).toLowerCase(),f.units);
          def.volume= (await solidOf(def)).volume();
        } else if(f.op==='faceted_brep') {
          const d=get(definitions,f.input), temp=join(stage,'.convert.step');
          if(d.exact) throw Error('faceted_brep expects a mesh');
          await writeFacetedStep(temp,'faceted',await solidOf(d));
          try {def.exact=keep(await exactFeature({op:'import_step'},null,()=>readFile(temp)));def.volume=checkBrep(def.exact);}
          finally {await rm(temp,{force:true});}
        } else if(['mesh_cut','mesh_union','mesh_intersect'].includes(f.op)) {
          const a=await solidOf(get(definitions,f.input)), b=await solidOf(get(definitions,f.tool));
          def.solid=keep(f.op==='mesh_cut'?a.subtract(b):f.op==='mesh_union'?a.add(b):a.intersect(b));
          if(def.solid.status()!=='NoError'||def.solid.isEmpty()||!(def.solid.volume()>0)) throw Error('Empty or invalid mesh Boolean');
          def.volume=def.solid.volume();def.chunks=[fromManifold(def.solid)];
        } else {
          def.exact=keep(await exactFeature(f,id=>{
            const d=get(definitions,id);
            if(!d.exact) throw Error('Exact operation requires B-rep inputs. Select mesh_cut, mesh_union or mesh_intersect for a faceted result.');
            return d.exact;
          },asset));
          def.volume=checkBrep(def.exact);
        }
      } catch(e) {throw Error('Feature '+f.id+' ('+f.op+'): '+(e.message??e),{cause:e});}
      definitions.set(f.id,def);
      receipt.features.push({id:f.id,op:f.op,elapsed_ms:performance.now()-featureStart,representation:def.exact?'brep':'mesh',volumeMm3:def.volume});
    }
    const exactParts=[];
    for(const item of spec.instances) {
      validId(item.id);
      if(parts.has(item.id)) throw Error('Duplicate instance '+item.id);
      const def=get(definitions,item.input), {matrix,rotation,translation}=placement(item);
      const color=item.material?.color, opacity=item.material?.opacity;
      if(color!==undefined&&!/^#[a-fA-F0-9]{6}$/.test(color)) throw Error('Expected material.color as #rrggbb');
      if(opacity!==undefined&&!(Number.isFinite(opacity)&&opacity>=0&&opacity<=1)) throw Error('Invalid material opacity');
      const part={id:item.id,chunks:[],def,matrix};
      const exact=def.exact&&keep(placeBrep(def.exact,rotation,translation));
      if(exact) {part.exact=exact;exactParts.push({name:item.id,shape:exact,color:color??materialDefault,alpha:opacity??1});}
      const chunks=chunksOf(def);
      chunks.forEach((raw,i)=>{
        const chunk=transformMesh({...raw,color:color??raw.color??materialDefault,opacity:opacity??raw.opacity??1},matrix);
        part.chunks.push(chunk);
        const key=item.input+'~'+i;
        bundle.meshes[key]??=viewerMesh(raw);scene.definitions[key]=key;
        const v=matrix.elements;
        scene.instances.push({id:chunks.length===1?item.id:item.id+'/'+(i+1),definition:key,origin:v.slice(12,15),
          axes:[v.slice(0,3),v.slice(4,7),v.slice(8,11)],material:{color:chunk.color,opacity:chunk.opacity}});
      });
      parts.set(item.id,part);
      receipt.instances.push({id:item.id,input:item.input,representation:exact?'brep':'mesh',triangles:part.chunks.reduce((s,c)=>s+c.indices.length/3,0),
        stl:item.id+'.stl',step:exact?'exact.step':spec.facetedStep===true?item.id+'.faceted.step':null});
    }
    for(const gate of spec.checks??[]) {
      const a=get(parts,gate.a), b=get(parts,gate.b);
      if(a===b) throw Error('A check needs two different instances');
      const limit=gate.maxOverlapMm3??0;
      if(!Number.isFinite(limit)||limit<0) throw Error('Invalid maximum overlap');
      const exact=a.exact&&b.exact;
      let volume;
      if(exact) volume=overlapVolume(a.exact,b.exact);
      else {
        const common=(await placedSolid(a)).intersect(await placedSolid(b));
        try {if(common.status()!=='NoError')throw Error('Mesh intersection failed');volume=common.volume();} finally {common.delete();}
      }
      if(!Number.isFinite(volume)||volume<0) throw Error('Invalid intersection volume');
      receipt.checks.push({a:a.id,b:b.id,method:exact?'brep-common':'triangle-solid-common',overlapMm3:volume,maxOverlapMm3:limit,passed:volume<=limit});
    }
    receipt.inspections=[];
    for(const q of spec.inspections??[]) {
      const part=get(parts,q.instance);
      if(q.op==='wall_probe') {
        const from=vec(q.from,'probe start'), to=vec(q.to,'probe end');
        const lo=q.minMm,hi=q.maxMm;
        if(!Number.isFinite(lo)||!Number.isFinite(hi)||lo<0||hi<lo) throw Error('Expected finite wall thickness interval');
        const direction=new Vector3(...to).sub(new Vector3(...from)).normalize();
        const hits=(await placedSolid(part)).rayCast(from,to);
        // Both ends must be in air, with alternating entry and exit crossings.
        const clean=hits.length>=2&&hits.length%2===0&&hits.every((hit,i)=>
          (i%2?1:-1)*new Vector3(...hit.normal).dot(direction)>1e-7);
        const spans=[];
        if(clean) for(let i=0;i<hits.length;i+=2)
          spans.push(new Vector3(...hits[i].position).distanceTo(new Vector3(...hits[i+1].position)));
        receipt.inspections.push({...q,method:'triangle-ray',spansMm:spans,passed:clean&&spans.every(v=>v>=lo&&v<=hi)});
      } else if(q.op==='slice') {
        if(!Number.isFinite(q.zMm)) throw Error('Expected slice height in mm');
        const slice=(await placedSolid(part)).slice(q.zMm);
        try {receipt.inspections.push({...q,method:'triangle-plane',areaMm2:slice.area(),contours:slice.toPolygons().length});}
        finally {slice.delete();}
      } else if(q.op==='overhang') {
        if(!Number.isFinite(q.angleDeg)||q.angleDeg<=0||q.angleDeg>=90||!Number.isFinite(q.bedZ)) throw Error('Expected overhang angle from vertical (0..90) and bedZ in mm');
        let area=0, triangles=0;const limit=-Math.sin(q.angleDeg*Math.PI/180);
        const a=new Vector3(),b=new Vector3(),c=new Vector3(),normal=new Vector3();
        for(const {positions,indices} of part.chunks) for(let i=0;i<indices.length;i+=3) {
          a.fromArray(positions,3*indices[i]);b.fromArray(positions,3*indices[i+1]);c.fromArray(positions,3*indices[i+2]);
          if(Math.max(a.z,b.z,c.z)<=q.bedZ+1e-6)continue;
          normal.crossVectors(b.sub(a),c.sub(a));const twiceArea=normal.length();
          if(normal.z<limit*twiceArea){area+=twiceArea/2;triangles++;}
        }
        receipt.inspections.push({...q,method:'downward-facet-angle',areaMm2:area,triangles});
      } else throw Error('Unknown inspection '+q.op);
    }
    receipt.ok=receipt.checks.every(x=>x.passed)&&receipt.inspections.every(x=>x.passed!==false);
    const exactData=exactParts.length?await stepBytes(exactParts):null, facets=[];
    if(exactData) await writeFile(join(stage,'exact.step'),exactData);
    for(const p of parts.values()) {
      await writeFile(join(stage,p.id+'.stl'),binarySTL(p.chunks));
      if(!p.exact&&spec.facetedStep===true) {
        const file=join(stage,p.id+'.faceted.step');
        await writeFacetedStep(file,p.id,await placedSolid(p));facets.push(file);
      }
    }
    if(facets.length) await combineStep(join(stage,'assembly.step'),exactData,facets);
    receipt.assemblyStep=facets.length?'assembly.step':parts.size===exactParts.length?'exact.step':null;
    await writeFile(join(stage,'assembly.glb'),await writeGlb([...parts.values()]));
    bundle.steps.push({id:'assembly',kind:'assembly',operation:'build',status:'succeeded',label:bundle.title,scene,
      durationMs:performance.now()-started,model:spec,geometry:{instances:parts.size},receipt:{data:receipt}});
    await writeViewerHtml(bundle,join(stage,'review.html'));
    await writeFile(join(stage,'report.json'),JSON.stringify(receipt,null,2)+'\n');
    return receipt;
  } finally {for(const o of owned.reverse())o.delete();}
}

export async function buildHybrid(spec,destination,{assetRoot=process.cwd()}={}) {
  const root=await realpath(assetRoot),out=resolve(destination);
  async function asset(path) {
    if(typeof path!=='string'||isAbsolute(path)) throw Error('Expected project-relative asset path');
    const file=await realpath(resolve(root,path)), rel=relative(root,file);
    if(rel==='..'||rel.startsWith('..'+sep)||isAbsolute(rel)) throw Error('Asset outside project');
    return readFile(file);
  }
  await mkdir(dirname(out),{recursive:true});
  const stage=await mkdtemp(join(dirname(out),'.colocad-hybrid-'));
  let acquired=false;
  try {
    const receipt=await evaluate(spec,asset,stage);
    await mkdir(out);acquired=true;
    for(const file of await readdir(stage)) await rename(join(stage,file),join(out,file));
    return receipt;
  } catch(e) {
    if(acquired) await rm(out,{recursive:true,force:true});
    throw e;
  } finally {await rm(stage,{recursive:true,force:true});}
}
