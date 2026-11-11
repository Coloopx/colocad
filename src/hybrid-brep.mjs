import initOC from 'replicad-opencascadejs';
import * as cad from 'replicad';
import {readFile} from 'node:fs/promises';
import {Vector3} from 'three';

let ready;
export function exactKernel() {
  ready??=readFile(new URL(import.meta.resolve('replicad-opencascadejs/wasm')))
    .then(wasmBinary=>initOC({wasmBinary,print:()=>{}})).then(oc=>{cad.setOC(oc);return oc;});
  return ready;
}

export function vec(p,label) {
  if(!Array.isArray(p)||p.length!==3||!p.every(Number.isFinite)) throw Error('Expected finite xyz for '+label);
  return p;
}

function splineWire(points) {
  if(!Array.isArray(points)||points.length<4) throw Error('A spline section needs at least four ordered landmarks');
  const ps=points.map(p=>new Vector3(...vec(p,'landmark')));
  const centroid=ps.reduce((s,p)=>s.add(p),new Vector3()).divideScalar(ps.length);
  const normal=new Vector3();
  ps.forEach((p,i)=>normal.add(new Vector3().crossVectors(p.clone().sub(centroid),ps[(i+1)%ps.length].clone().sub(centroid))));
  if(normal.length()<1e-9 || new Set(points.map(p=>p.join(','))).size!==points.length) throw Error('Degenerate spline section');
  normal.normalize();
  if(ps.some(p=>Math.abs(normal.dot(p.clone().sub(centroid)))>1e-6)) throw Error('Spline landmarks must be coplanar');
  const at=i=>ps[(i+ps.length)%ps.length];
  const edges=ps.map((p,i)=>cad.makeBezierCurve([
    p.toArray(), p.clone().addScaledVector(at(i+1).clone().sub(at(i-1)),1/6).toArray(),
    at(i+1).clone().addScaledVector(at(i+2).clone().sub(p),-1/6).toArray(), at(i+1).toArray()
  ]));
  try { return cad.assembleWire(edges); } finally { for(const edge of edges) edge.delete(); }
}

export async function exactFeature(f,get,asset) {
  const oc=await exactKernel();
  switch(f.op) {
    case 'box': return cad.makeBox(vec(f.min,'min'),vec(f.max,'max'));
    case 'cylinder':
    case 'cone': {
      const radius=f.radius,height=f.height,base=vec(f.base??[0,0,0],'base'),axis=new Vector3(...vec(f.axis??[0,0,1],'axis'));
      if(!Number.isFinite(radius)||radius<=0||!Number.isFinite(height)||height<=0||axis.lengthSq()<1e-18) throw Error('Expected positive radius, height and nonzero axis');
      axis.normalize();
      if(f.op==='cylinder')return cad.makeCylinder(radius,height,base,axis.toArray());
      const ref=axis.clone().cross(Math.abs(axis.z)<.8?new Vector3(0,0,1):new Vector3(1,0,0)).normalize();
      const start=new Vector3(...base),end=start.clone().addScaledVector(axis,height);
      const face=cad.makePolygon([base,end.toArray(),end.clone().addScaledVector(ref,radius).toArray()]);
      try{return cad.revolution(face,base,axis.toArray());}finally{face.delete();}
    }
    case 'hollow': {
      if(!Number.isFinite(f.thickness)||f.thickness<=0)throw Error('Expected positive inward offset');
      const source=get(f.input),filter=new cad.FaceFinder().when(()=>false);
      try {
        const core=source.shell({filter,thickness:f.thickness},1e-4);
        try {return source.cut(core);}finally{core.delete();}
      } finally {filter.delete();}
    }
    case 'sphere': {
      if(!(Number.isFinite(f.radius)&&f.radius>0)) throw Error('Expected positive sphere radius');
      return cad.makeSphere(f.radius).translate(vec(f.center??[0,0,0],'center'));
    }
    case 'import_step': return cad.importSTEP(new Blob([await asset(f.path)]));
    case 'profile_loft': {
      if(!Array.isArray(f.sections)||f.sections.length<2||!f.sections.every(s=>s.length===f.sections[0].length))
        throw Error('At least two sections with matching ordered landmarks are required');
      const loft=new oc.BRepOffsetAPI_ThruSections(true,false,1e-6), wires=[];
      try {
        loft.CheckCompatibility(false); loft.SetMaxDegree(3);
        for(const section of f.sections){const wire=splineWire(section);wires.push(wire);loft.AddWire(wire.wrapped);}
        loft.Build();
        if(!loft.IsDone()) throw Error('Smooth loft failed');
        return cad.cast(loft.Shape());
      } finally { loft.delete();for(const w of wires)w.delete(); }
    }
    case 'shell': {
      if(!Number.isFinite(f.thickness)||f.thickness<=0) throw Error('Expected positive inward wall thickness in mm');
      const filter=new cad.FaceFinder(), source=get(f.input);
      try {
        const {plane,at}=f.openFace??{};
        if(!['XY','XZ','YZ'].includes(plane)||!Number.isFinite(at)) throw Error('openFace needs plane XY/XZ/YZ and signed normal offset at');
        const axis={XY:2,XZ:1,YZ:0}[plane];
        filter.when(({element,normal})=>element.geomType==='PLANE' && Math.abs(Math.abs(normal.toTuple()[axis])-1)<1e-8 && Math.abs(element.center.toTuple()[axis]-at)<1e-6);
        const found=filter.find(source), count=found.length;
        found.forEach(face=>face.delete());
        if(count!==1) throw Error('Opening selector matched '+count+' faces');
        return source.shell({filter,thickness:f.thickness},1e-4);
      } finally {filter.delete();}
    }
    case 'cut': return get(f.input).cut(get(f.tool));
    case 'union': return get(f.input).fuse(get(f.tool));
    case 'intersect': return get(f.input).intersect(get(f.tool));
    default: throw Error('Unknown exact feature '+f.op);
  }
}

function volume(shape){
  const oc=cad.getOC(), props=new oc.GProp_GProps();
  try {oc.BRepGProp.VolumeProperties(shape.wrapped,props,1e-9,true,false);return props.Mass();}
  finally {props.delete();}
}
export function checkBrep(shape) {
  const oc=cad.getOC(), check=new oc.BRepCheck_Analyzer(shape.wrapped,true);
  try {
    if(!check.IsValid(shape.wrapped)) throw Error('OCCT BRepCheck rejected result');
    const solids=shape.solids;
    try {if(!solids.length) throw Error('No B-rep solid');} finally {solids.forEach(s=>s.delete());}
    const measured=volume(shape);
    if(!(measured>0&&Number.isFinite(measured))) throw Error('Invalid signed B-rep volume');
    return measured;
  } finally {check.delete();}
}

export function exactChunks(shape,tolerance) {
  const m=shape.mesh({tolerance,angularTolerance:.12});
  return [{positions:new Float32Array(m.vertices),indices:new Uint32Array(m.triangles)}];
}
export function placeBrep(shape,rotation,translation) {
  let result=shape.clone();
  if(rotation) result=result.rotate(rotation.degrees,rotation.center,rotation.axis);
  return result.translate(translation);
}
export function overlapVolume(a,b) {
  const common=a.intersect(b);
  try {return volume(common);} finally {common.delete();}
}
export async function stepBytes(shapes) {
  return Buffer.from(await cad.exportSTEP(shapes,{unit:'MM',modelUnit:'MM'}).arrayBuffer());
}
