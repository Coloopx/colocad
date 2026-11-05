// Editable reconstruction in millimetres, based on the four reference images.
// No upstream mesh, joint definition, CAD file or transform is read by this builder.
import { mkdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

export const stages=['wrist','palm','proximal','middle','tips','thumb','drive'];
export function hand(stage='drive', { cableRadius=.65, guideDiameter=2.9 }={}) {
  if (!stages.includes(stage) || !(cableRadius>0 && cableRadius<1.1) || !(guideDiameter>=2.9 && guideDiameter<=3.3)) throw Error('Invalid stage or tendon radius');
  const features=[],parts=[],meta={}; const at=stages.indexOf(stage);
  const frame=(origin,x_axis=[1,0,0],y_axis=[0,1,0])=>({origin,x_axis,y_axis});
  const F=(id,op,args)=>(features.push({id,op,...args}),id);
  const ext=(id,profile,o,h,xa=[1,0,0],ya=[0,1,0])=>F(id,'extrude',{profile,frame:frame(o,xa,ya),height:h});
  const circle=r=>({kind:'circle',radius:r});
  const side=(id,x,y,z,r,w)=>ext(id,circle(r),[x,y,z],w,[0,1,0],[0,0,1]);
  const bore=(id,base,tool)=>F(id,'cut',{left:base,right:tool});
  const rect=(id,origin,size)=>F(id,'box',{origin,size});
  const channel=(id,body,x,y,lo,hi)=>bore(id,body,ext(id+'Tool',circle(id==='ProximalCableGuide'?{param:'guideRadius'}:1.45),[x,y,lo],hi-lo));
  function inst(stageN,id,input,pos=[0,0,0],rot=[0,0,0],group='frame'){
    if(stageN<=at){parts.push({id,input,translation:pos,rotation_deg:rot});meta[id]=group;}
  }
  const wrist=F('Yoke','pipe',{path:{kind:'arc',start:[-30,0,-4],via:[0,0,-36],end:[32,0,-4]},radius:4.8});
  inst(0,'Wrist_Yoke',wrist);
  const shaft=ext('WristAxle',circle(12.5),[0,15,-34],30,[1,0,0],[0,0,1]);
  inst(0,'Wrist_Axle',shaft,[0,0,0],[0,0,0],'pins');
  let palm=ext('PalmBlank',{kind:'polygon',points:[[-37,-8],[-16,-8],[-16,0],[8,0],[8,-8],[41,-8],[41,70],[21,70],[21,74],[-18,74],[-18,52],[-25,52],[-25,57],[-42,57],[-42,22],[-37,13]]},[0,10,0],23,[1,0,0],[0,0,1]);
  const fingers=[['Little',-33,85,57],['Ring',-11,94,74],['Middle',11,98,74],['Index',33,94,70]];
  for(const [name,x] of fingers)palm=channel('PalmGuide'+name,palm,x,-5,-15,110);
  inst(1,'Palm',palm);
  const unrot=[0,0,0], R=47*Math.PI/180;
  const thumbAt=(z,dy=0)=>[32+Math.sin(R)*z,-12+dy,26+Math.cos(R)*z];
  const thumbRot=[0,47,0];
  // Capsules are constructed on a YZ sketch plane. Overlapping envelopes at a
  // hinge become a central tongue and two fork ears with a transverse pin bore.
  function link(key,L,w,r,terminal=false){
    const profile={kind:'outline',start:[-r,0],segments:[
      {kind:'arc',via:[0,-r],to:[r,0]},
      {kind:'line',to:[r*(terminal?.79:1),L]},
      {kind:'arc',via:[0,L+(terminal?4.5:r)],to:[-r*(terminal?.79:1),L]},
      {kind:'line',to:[-r,0]}]};
    let body=ext(key+'Blank',profile,[-w/2,0,0],w,[0,1,0],[0,0,1]);
    for(const sign of [-1,1]){
      const s=sign<0?'Neg':'Pos',edge=sign<0?-w/2-1:3.65,ww=w/2-3.65+1;
      body=bore(key+'Tang'+s,body,rect(key+'Trim'+s,[edge,-r-2,-r-2],[ww,2*r+4,2*r+2.6]));
    }
    body=bore(key+'PinRoot',body,side(key+'RootDrill',-w/2-1,0,0,2.1,w+2));
    if(!terminal){
      body=bore(key+'Fork',body,rect(key+'ForkMill',[-3.95,-r-2,L-r-.65],[7.9,2*r+4,2*r+3]));
      body=bore(key+'PinEnd',body,side(key+'EndDrill',-w/2-1,0,L,2.1,w+2));
    }
    return channel(key+'CableGuide',body,0,-5,-r-2,L+r+2);
  }
  function mounting(key,h){
    const r=8.25,w=18;
    const prof={kind:'outline',start:[-r,0],segments:[{kind:'line',to:[r,0]},{kind:'line',to:[r,h]},{kind:'arc',via:[0,h+r],to:[-r,h]},{kind:'line',to:[-r,0]}]};
    let a=ext(key+'Base',prof,[-w/2,0,0],w,[0,1,0],[0,0,1]);
    a=bore(key+'Fork',a,rect(key+'Mill',[-3.95,-12,h-r-.65],[7.9,24,2*r+3]));
    a=bore(key+'Pivot',a,side(key+'Drill',-10,0,h,2.1,20));
    return channel(key+'Guide',a,0,-5,-2,h+r+2);
  }
  const proximal=link('Proximal',47,18.8,8.3), middle=link('Intermediate',27,17.5,7.7), tip=link('Tip',23,16.2,7.3,true);
  const pin=side('Pin',-9.55,0,0,1.85,19.1);
  for(const [name,x,z,base] of fingers){
    inst(1,name+'_Mount',mounting(name+'Socket',z-base),[x,0,base]);
    inst(2,name+'_Proximal',proximal,[x,0,z],unrot,'links');
    inst(3,name+'_Middle',middle,[x,0,z+47],unrot,'links');
    inst(4,name+'_Tip',tip,[x,0,z+74],unrot,'tips');
    for(const [i,off] of [0,47,74].entries())inst(6,name+'_Pin'+i,pin,[x,0,z+off],unrot,'pins');
    const cable=ext(name+'CordDef',circle({param:'cableRadius'}),[0,0,0],z+74+19+8);
    inst(6,name+'_Tendon',cable,[x,-5,-8],unrot,'cord');
  }
  const thumbL1=40,thumbL2=32;
  inst(5,'Thumb_Mount',mounting('ThumbSocket',12),thumbAt(-12),thumbRot);
  inst(5,'Thumb_Proximal',link('ThumbBaseLink',thumbL1,20,9),thumbAt(0),thumbRot,'links');
  inst(5,'Thumb_Middle',link('ThumbMidLink',thumbL2,19,8.25),thumbAt(thumbL1),thumbRot,'links');
  inst(5,'Thumb_Tip',link('ThumbEndLink',25,17,7.7,true),thumbAt(thumbL1+thumbL2),thumbRot,'tips');
  for(const [i,z] of [0,thumbL1,thumbL1+thumbL2].entries())inst(6,'Thumb_Pin'+i,pin,thumbAt(z),thumbRot,'pins');
  inst(6,'Thumb_Tendon',ext('ThumbCordDef',circle({param:'cableRadius'}),[0,0,0],105),thumbAt(-11,-5),thumbRot,'cord');
  // Only evaluate features referenced by this milestone.
  const used=new Set(parts.map(i=>i.input));
  for(const f of [...features].reverse())if(used.has(f.id))for(const key of ['input','left','right'])if(f[key])used.add(f[key]);
  return {model:{version:1,units:'mm',parameters:{cableRadius,guideRadius:guideDiameter/2},features:features.filter(f=>used.has(f.id)),assembly:parts},groups:meta};
}
if(process.argv[1] && import.meta.url===pathToFileURL(process.argv[1]).href){
  const dir=process.argv[2];if(!dir) throw Error('usage: node model.mjs NEW_DIR');
  await mkdir(dir,{recursive:false});
  for(const stage of stages)await writeFile(join(dir,stage+'.json'),JSON.stringify(hand(stage).model,null,2)+'\n');
}
