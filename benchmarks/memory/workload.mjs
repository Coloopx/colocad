import {CadClient} from '/pkg/src/index.mjs';
import {mkdir,writeFile,appendFile} from 'node:fs/promises';
import {join} from 'node:path';
const [family,ns,ss,out,query]=process.argv.slice(2),n=Number(ns),start=Number(ss),t=performance.now();
await mkdir(out,{recursive:true});
const event=async phase=>appendFile(join(out,'events.jsonl'),JSON.stringify({phase,elapsed_s:(performance.now()-t)/1000})+'\n');
await event('imported');
const model={version:1,units:'mm',parameters:{},features:[]};let expected=0;
function part(id,holes,index=0,unique=false){
 const nx=Math.sqrt(holes),x=nx*4+4+(unique?index*.0001:0),y=nx*4+4;
 model.features.push({id:id+'-stock',op:'box',origin:[0,0,0],size:[x,y,6]},{id,op:'hole_grid',input:id+'-stock',origin:[4,4,-1],radius:1,depth:8,count:[nx,nx],spacing:[4,4]});
 return (x*y-holes*Math.PI)*6;
}
if(family==='part'){expected=part('plate',n);model.output='plate';}
else if(family==='chain'){
 const nx=Math.ceil(Math.sqrt(n)),l=nx*4+4;model.features.push({id:'stock',op:'box',origin:[0,0,0],size:[l,l,6]});
 for(let i=0;i<n;i++)model.features.push({id:'h'+i,op:'holes',input:i?'h'+(i-1):'stock',frame:{origin:[4+4*(i%nx),4+4*Math.floor(i/nx),-1],x_axis:[1,0,0],y_axis:[0,1,0]},positions:[[0,0]],radius:1,depth:8});
 model.output='h'+(n-1);expected=(l*l-n*Math.PI)*6;
}else{
 model.assembly=[];const shared=family==='repeated'?part('plate',16):null;
 for(let i=start;i<start+n;i++){const id=shared===null?'p'+i:'plate';expected+=shared??part(id,16,i,true);model.assembly.push({id:'i'+i,input:id,translation:[(i%64)*25,Math.floor(i/64)*25,0]});}
}
await writeFile(join(out,'input.json'),JSON.stringify(model));
const cad=new CadClient({outputRoot:out,wallMs:120000});
const r=await cad.build(model,{queryable:query==='true'});
await event(r.ok?'exported':'rejected');
if(r.ok){const g=r.data.geometry;if(Math.abs(g.volume_mm3-expected)>Math.max(1e-5,expected*1e-7))throw Error('analytic volume mismatch');}
await writeFile(join(out,'result.json'),JSON.stringify({...r,expected_volume_mm3:expected}));
if(!r.ok)process.exitCode=2;
