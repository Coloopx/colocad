// Build the neutral sketch/operation graph separately from its solved B-rep.
// The probe explicitly represents an unbound fillet. It must never substitute
// final-body edge numbers for historical edges consumed by the operation.
import {readFileSync} from 'node:fs';
const [geometry,program]=process.argv.slice(2);
const ir=JSON.parse(readFileSync(geometry,'utf8')), job=JSON.parse(readFileSync(program,'utf8'));
if(job.format!=='colocad/native-job/1')throw Error('Expected a Colocad native job');
const m=ir.model;
if(m.features.length||m.sketches.length)throw Error('Expected geometry without an existing design history');
const fid=name=>'colocad:model:feature#'+name;
const add=(name,source_tag,definition,deps=[])=>{
  m.features.push({id:fid(name),ordinal:m.features.length,name,suppressed:false,source_tag,definition,dependencies:deps.map(fid)});
};
function sketch(f){
  const name='Sketch_'+f.id, key='colocad:model:sketch#'+name;
  const p=f.profile,uses=[];
  const uv=([u,v])=>({u,v});
  const entity=g=>{
    const id='colocad:model:sketch-entity#'+name+'_'+uses.length;
    m.sketch_entities.push({id,sketch:key,construction:false,geometry:g});uses.push({entity:id,reversed:false});
  };
  if(p.kind==='circle')entity({kind:'circle',center:uv(p.center),radius:p.radius});
  else{
    let points=p.points;
    if(p.kind==='rectangle'){
      const [x,y]=p.center,[w,h]=p.size.map(n=>n/2);
      points=[[x-w,y-h],[x+w,y-h],[x+w,y+h],[x-w,y+h]];
    }
    points.forEach((a,i)=>entity({kind:'line',start:uv(a),end:uv(points[(i+1)%points.length])}));
  }
  m.sketches.push({id:key,name,profiles:[uses],placement:{kind:'resolved',frame:{
    origin:{x:0,y:0,z:f.startZ},normal:{x:0,y:0,z:1},u_axis:{x:1,y:0,z:0}}}});
  add(name,'Sketch',{definition:'sketch',sketch:{space:'planar',sketch:key}});
  return {name,key};
}
for(const f of job.features){
  if(f.op==='extrude'||f.op==='cut'){
    const p=sketch(f);
    add(f.id,'Extrusion',{definition:'extrude',profile:{kind:'sketch',value:p.key},direction:{kind:'profile_normal'},
      start:{kind:'profile_plane'},extent:{kind:'one_sided',side:{termination:{kind:'blind',length:f.depth}}},
      op:f.op==='cut'?'cut':'join',solid:true},[p.name]);
  }else if(f.op==='fillet'){
    add(f.id,'Fillet',{definition:'fillet',groups:[{edges:{kind:'unresolved'},radius:{kind:'constant',radius:f.radius}}]});
  }else if(f.op==='linearPattern'){
    add(f.id,'Feature',{definition:'pattern',seeds:[{kind:'feature',value:fid(f.seed)}],pattern:{
      kind:'linear',direction:{x:Number(f.axis==='x'),y:Number(f.axis==='y'),z:0},spacing:f.pitch,count:f.count}},[f.seed]);
  }else throw Error('Unknown operation');
}
process.stdout.write(JSON.stringify(ir));
