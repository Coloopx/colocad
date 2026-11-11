import {open} from 'node:fs/promises';
import {createReadStream} from 'node:fs';
import {createInterface} from 'node:readline';
import {Vector3} from 'three';

// One shared Cartesian point per indexed position. Emit planar faces with polygonal
// loops directly so exporting a scan does not allocate one OCCT object per facet.
function real(n) {
  const [mantissa,exponent]=String(n).toUpperCase().split('E');
  return (mantissa.includes('.')?mantissa:mantissa+'.')+(exponent?'E'+exponent:'');
}
export async function writeFacetedStep(path,name,solid) {
  const file=await open(path,'wx'), pieces=solid.decompose();
  let buffer='',next=1;
  const entity=text=>{const id=next++;buffer+='#'+id+'='+text+';\n';return '#'+id;};
  async function flush(){await file.writeFile(buffer);buffer='';}
  try {
    buffer="ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('Faceted solid'),'2;1');\nFILE_NAME('"+name+"','',('Colocad'),('Colocad'),'Colocad','Colocad','');\nFILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF {1 0 10303 442 1 1 4 }'));\nENDSEC;\nDATA;\n";
    const app=entity("APPLICATION_CONTEXT('Managed model based 3d engineering')");
    entity("APPLICATION_PROTOCOL_DEFINITION('international standard','ap242_managed_model_based_3d_engineering',2013,"+app+")");
    const context=entity("PRODUCT_CONTEXT('',"+app+",'mechanical')");
    const product=entity("PRODUCT('"+name+"','"+name+"','',("+context+"))");
    const formation=entity("PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE('','',"+product+",.NOT_KNOWN.)");
    const design=entity("PRODUCT_DEFINITION_CONTEXT('',"+app+",'design')");
    const def=entity("PRODUCT_DEFINITION('design','',"+formation+","+design+")");
    const shape=entity("PRODUCT_DEFINITION_SHAPE('','',"+def+")");
    const mm=entity('(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.))');
    const rad=entity('(NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.))');
    const sr=entity('(NAMED_UNIT(*) SI_UNIT($,.STERADIAN.) SOLID_ANGLE_UNIT())');
    const tol=entity("UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.E-6),"+mm+",'distance_accuracy_value','')");
    const model=entity("(GEOMETRIC_REPRESENTATION_CONTEXT(3) GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT(("+tol+")) GLOBAL_UNIT_ASSIGNED_CONTEXT(("+[mm,rad,sr]+")) REPRESENTATION_CONTEXT('','3D'))");
    const bodies=[];
    for(const piece of pieces) {
      if(piece.volume()<=0) throw Error('Faceted STEP export requires independent outer shells; enclosed void shells need a B-rep with voids');
      const m=piece.getMesh(), pts=m.vertProperties, faces=m.triVerts, pointIds=[];
      for(let i=0;i<pts.length;i+=m.numProp) {
        pointIds.push(entity("CARTESIAN_POINT('',("+[pts[i],pts[i+1],pts[i+2]].map(real)+"))"));
        if(buffer.length>=262144) await flush();
      }
      const faceIds=[], a=new Vector3(), ab=new Vector3(), ac=new Vector3();
      for(let i=0;i<faces.length;i+=3) {
        const ia=faces[i],ib=faces[i+1],ic=faces[i+2];
        a.fromArray(pts,ia*m.numProp);ab.fromArray(pts,ib*m.numProp).sub(a).normalize();
        ac.fromArray(pts,ic*m.numProp).sub(a);
        const n=ab.clone().cross(ac).normalize();
        if(n.lengthSq()<.5) throw Error('Degenerate triangle in faceted STEP');
        const axis=entity("DIRECTION('',("+n.toArray().map(real)+"))");
        const ref=entity("DIRECTION('',("+ab.toArray().map(real)+"))");
        const plane=entity("PLANE('',"+entity("AXIS2_PLACEMENT_3D('',"+[pointIds[ia],axis,ref]+")")+")");
        const loop=entity("POLY_LOOP('',("+[pointIds[ia],pointIds[ib],pointIds[ic]]+"))");
        const bound=entity("FACE_OUTER_BOUND('',"+loop+",.T.)");
        faceIds.push(entity("FACE_SURFACE('',("+bound+"),"+plane+",.T.)"));
        if(buffer.length>=262144) await flush();
      }
      const shell=entity("CLOSED_SHELL('',("+faceIds.join(',')+"))");
      bodies.push(entity("FACETED_BREP('',"+shell+")"));
    }
    const rep=entity("FACETED_BREP_SHAPE_REPRESENTATION('',("+bodies.join(',')+"),"+model+")");
    entity("SHAPE_DEFINITION_REPRESENTATION("+shape+","+rep+")");
    buffer+='ENDSEC;\nEND-ISO-10303-21;\n';await flush();
  } finally {for(const piece of pieces)piece.delete();await file.close();}
}

// Join generated exchange roots in their common millimetre world coordinate system.
// Only our own faceted writer is renumbered. User-supplied STEP is parsed by OCCT.
export async function combineStep(path,exact,facets) {
  const file=await open(path,'wx');
  let maxId=0;
  try {
    if(exact) {
      const text=exact.toString('utf8'), end=text.lastIndexOf('ENDSEC;');
      if(end<0)throw Error('Invalid exact STEP');
      await file.writeFile(text.slice(0,end));
      for(const match of text.matchAll(/^#(\d+)\s*=/gm))maxId=Math.max(maxId,Number(match[1]));
    }
    for(let i=0;i<facets.length;i++) {
      const stream=createReadStream(facets[i]),lines=createInterface({input:stream,crlfDelay:Infinity});
      const offset=maxId;
      let data=false, batch='';
      try {
        for await(const line of lines) {
          if(!data) {
            if(!exact&&i===0)batch+=line+'\n';
            if(line==='DATA;')data=true;
            continue;
          }
          if(line==='ENDSEC;')break;
          batch+=line.replace(/#(\d+)/g,(_,n)=>{const shifted=Number(n)+offset;maxId=Math.max(maxId,shifted);return '#'+shifted;})+'\n';
          if(batch.length>=262144){await file.writeFile(batch);batch='';}
        }
        await file.writeFile(batch);
      } finally {lines.close();stream.destroy();}
    }
    await file.writeFile('ENDSEC;\nEND-ISO-10303-21;\n');
  } finally {await file.close();}
}
