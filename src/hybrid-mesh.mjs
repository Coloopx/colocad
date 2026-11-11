import { Document, NodeIO } from '@gltf-transform/core';
import { Color, Matrix4, ShapeUtils, Vector2, Vector3 } from 'three';
import { STLLoader } from 'three/addons/loaders/STLLoader.js';

export const cadFromGltf = new Matrix4().makeRotationX(Math.PI / 2).scale(new Vector3(1000,1000,1000));
const gltfFromCad = cadFromGltf.clone().invert();
export const materialDefault = '#a6afbc';

export function transformMesh(mesh, matrix) {
  const positions = new Float32Array(mesh.positions.length);
  const v = new Vector3();
  for (let i=0; i<positions.length; i+=3)
    v.fromArray(mesh.positions,i).applyMatrix4(matrix).toArray(positions,i);
  let indices=mesh.indices;
  if (matrix.determinant()<0) {
    indices=indices.slice();
    for (let i=0;i<indices.length;i+=3) [indices[i+1],indices[i+2]]=[indices[i+2],indices[i+1]];
  }
  return {...mesh,positions,indices};
}

function objMesh(text) {
  const positions=[], indices=[];
  for (let offset=0; offset<text.length;) {
    let end=text.indexOf('\n',offset);
    if(end===-1) end=text.length;
    const raw=text.slice(offset,end);offset=end+1;
    const [cmd,...args]=raw.split('#')[0].trim().split(/\s+/);
    if (cmd==='v') {
      if(args.length!==3 || !args.map(Number).every(Number.isFinite)) throw Error('OBJ requires finite xyz vertices');
      positions.push(...args.map(Number));
    } else if (cmd==='f') {
      const corners=args.map(token=>{
        const n=Number(token.split('/')[0]), i=n>0?n-1:positions.length/3+n;
        if(!Number.isInteger(n)||n===0||i<0||i>=positions.length/3) throw Error('Invalid OBJ face index');
        return i;
      });
      if(corners.length===3) { indices.push(...corners); continue; }
      // Project the polygon onto its dominant plane before ear-clipping concave faces.
      const points=corners.map(i=>new Vector3().fromArray(positions,3*i)), normal=new Vector3();
      points.forEach((v,i)=>normal.add(new Vector3().crossVectors(v,points[(i+1)%points.length])));
      const axis=['x','y','z'].sort((a,b)=>Math.abs(normal[b])-Math.abs(normal[a]))[0];
      const [a,b]=['x','y','z'].filter(x=>x!==axis);
      const faces=ShapeUtils.triangulateShape(points.map(p=>new Vector2(p[a],p[b])),[]);
      if(!faces.length || faces.length!==corners.length-2) throw Error('Invalid OBJ polygon');
      for(const [i,j,k] of faces) {
        const forward=new Vector3().subVectors(points[j],points[i]).cross(new Vector3().subVectors(points[k],points[i])).dot(normal)>0;
        indices.push(corners[i],corners[forward?j:k],corners[forward?k:j]);
      }
    } else if (!['','o','g','s','vn','vt','mtllib','usemtl'].includes(cmd)) throw Error('Unsupported OBJ record '+cmd);
  }
  return {positions:new Float32Array(positions),indices:new Uint32Array(indices)};
}

export async function readMesh(bytes,ext,units) {
  if(ext==='glb') {
    if(units && units!=='m') throw Error('GLB uses metres');
    const doc=await new NodeIO().readBinary(bytes), root=doc.getRoot();
    if(root.listAnimations().length||root.listSkins().length) throw Error('Bake animated or skinned GLB geometry first');
    const scene=root.getDefaultScene()??(root.listScenes().length===1&&root.listScenes()[0]);
    if(!scene) throw Error('GLB requires a default scene');
    const chunks=[];
    scene.traverse(node=>{
      const matrix=cadFromGltf.clone().multiply(new Matrix4().fromArray(node.getWorldMatrix()));
      for(const prim of node.getMesh()?.listPrimitives()??[]) {
        if(prim.getMode()!==4 || prim.listTargets().length) throw Error('GLB requires static TRIANGLES primitives');
        const attr=prim.getAttribute('POSITION');
        if(!attr||attr.getType()!=='VEC3'||attr.getComponentType()!==5126) throw Error('GLB requires float VEC3 POSITION');
        const positions=attr.getArray(), ids=prim.getIndices()?.getArray()??Uint32Array.from({length:attr.getCount()},(_,i)=>i);
        const mat=prim.getMaterial(), rgba=mat?.getBaseColorFactor()??[1,1,1,1];
        const color='#'+new Color().fromArray(rgba).getHexString();
        chunks.push(transformMesh({positions,indices:new Uint32Array(ids),color,opacity:rgba[3]},matrix));
      }
    });
    if(!chunks.length) throw Error('Empty GLB scene');
    return chunks;
  }
  const scale={mm:1,cm:10,m:1000,in:25.4}[units];
  if(!Number.isFinite(scale)) throw Error('STL and OBJ require units mm, cm, m or in');
  let mesh;
  if(ext==='stl') {
    const data=new STLLoader().parse(bytes.buffer.slice(bytes.byteOffset,bytes.byteOffset+bytes.byteLength));
    const positions=data.getAttribute('position').array;
    mesh={positions,indices:Uint32Array.from({length:positions.length/3},(_,i)=>i)};
    data.dispose();
  } else if (ext==='obj') mesh=objMesh(bytes.toString('utf8'));
  else throw Error('Expected STL, OBJ or GLB');
  return [transformMesh(mesh,new Matrix4().makeScale(scale,scale,scale))];
}

export function meshSolid(api,chunks) {
  const count=chunks.reduce((v,c)=>v+c.positions.length,0), positions=new Float32Array(count);
  const indices=new Uint32Array(chunks.reduce((v,c)=>v+c.indices.length,0));
  let p=0,j=0;
  for(const c of chunks) {
    if(!c.positions.length||c.positions.length%3||!c.indices.length||c.indices.length%3||
       !c.positions.every(Number.isFinite)||c.indices.some(i=>i>=c.positions.length/3)) throw Error('Invalid triangle data');
    positions.set(c.positions,p);
    for(const i of c.indices) indices[j++]=i+p/3;
    p+=c.positions.length;
  }
  const mesh=new api.Mesh({numProp:3,vertProperties:positions,triVerts:indices});
  mesh.merge();
  const solid=new api.Manifold(mesh);
  if(solid.status()!=='NoError'||solid.isEmpty()||!(solid.volume()>0)) {
    const status=solid.status(); solid.delete(); throw Error('Expected a closed, positively oriented manifold mesh ('+status+')');
  }
  return solid;
}

export function fromManifold(solid) {
  const m=solid.getMesh();
  return {positions:m.vertProperties,indices:m.triVerts};
}

export function binarySTL(meshes) {
  const n=meshes.reduce((s,m)=>s+m.indices.length/3,0), b=Buffer.alloc(84+50*n);
  b.writeUInt32LE(n,80);
  const a=new Vector3(), v=new Vector3(), w=new Vector3(), normal=new Vector3();
  let at=84;
  for(const {positions,indices} of meshes) for(let i=0;i<indices.length;i+=3) {
    a.fromArray(positions,3*indices[i]); v.fromArray(positions,3*indices[i+1]); w.fromArray(positions,3*indices[i+2]);
    normal.subVectors(v,a).cross(new Vector3().subVectors(w,a)).normalize();
    for(const point of [normal,a,v,w]) for(const key of ['x','y','z']) { b.writeFloatLE(point[key],at); at+=4; }
    at+=2;
  }
  return b;
}

export async function writeGlb(parts) {
  const d=new Document(), buf=d.createBuffer(), scene=d.createScene('Assembly');
  d.getRoot().setDefaultScene(scene);
  const mats=new Map();
  for(const {id,chunks} of parts) {
    const mesh=d.createMesh(id);
    for(const chunk of chunks) {
      const {positions,indices}=transformMesh(chunk,gltfFromCad);
      const key=[chunk.color,chunk.opacity].join('/');
      if(!mats.has(key)) {
        const mat=d.createMaterial().setBaseColorFactor([...new Color(chunk.color??materialDefault).toArray(),chunk.opacity??1]).setMetallicFactor(0);
        if(chunk.opacity<1) mat.setAlphaMode('BLEND');
        mats.set(key,mat);
      }
      const access=(type,arr)=>d.createAccessor().setBuffer(buf).setType(type).setArray(arr);
      mesh.addPrimitive(d.createPrimitive().setAttribute('POSITION',access('VEC3',positions)).setIndices(access('SCALAR',indices)).setMaterial(mats.get(key)));
    }
    scene.addChild(d.createNode(id).setMesh(mesh));
  }
  return new NodeIO().writeBinary(d);
}
