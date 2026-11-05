// Resolve a fillet's operands against the INPUT state, before filleting removes them.
// IDs in this report name B-rep records in a single regenerated state. They are
// deliberately distinguished from SOLIDWORKS persistent selection vectors.
export function inputEdges(model,rule,tolerance=0.001){
  const index=new Map(['edges','vertices','points','coedges','loops','faces','curves'].flatMap(key=>model[key].map(x=>[x.id,x])));
  const point=v=>index.get(index.get(v).point).position;
  const rows=[];
  for(const edge of model.edges){
    const g=index.get(edge.carrier.curve).geometry;
    const a=point(edge.start),b=point(edge.end);
    if(Math.abs(a.z-rule.z)>tolerance || Math.abs(b.z-rule.z)>tolerance)continue;
    let planar=false;
    if(g.kind==='line')planar=Math.abs(g.direction.z)<1e-8;
    else if(g.kind==='circle')planar=Math.abs(Math.abs(g.axis.z)-1)<1e-8 && Math.abs(g.center.z-rule.z)<=tolerance;
    else if(g.kind==='nurbs')planar=g.poles.points.every(p=>Math.abs(p.z-rule.z)<=tolerance);
    else throw Error('Unsupported candidate edge '+edge.id);
    if(!planar)continue;
    const faces=model.coedges.filter(c=>c.edge===edge.id).map(c=>index.get(c.owner_loop).face);
    if(faces.length!==2)throw Error('Edge does not have two incident coedges');
    rows.push({edge:edge.id,faces:faces.sort(),curveKind:g.kind,start:a,end:b});
  }
  if(rows.length!==rule.count)throw Error('Fillet selected '+rows.length+' edges, expected '+rule.count);
  return rows;
}
