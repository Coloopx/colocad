// Original illustrative geometry, not dimensions or airfoils from a production engine.
// Z is the engine axis; all dimensions are millimetres. See README for assumptions.
export const stages=[
  ['fan-stock','Turn the fan hub blank','geometry'],
  ['fan-bore','Bore the fan hub','geometry'],
  ['fan-blade','Loft the twisted fan blade','geometry'],
  ['fan-row','Place the fan blade row','assembly'],
  ['shaft','Add concentric shafts and spinner','assembly'],
  ['compressor-1','Build the first compressor stage','assembly'],
  ['compressor-2','Build the second compressor stage','assembly'],
  ['compressor-3','Build the third compressor stage','assembly'],
  ['combustor-blank','Revolve the combustor liner','geometry'],
  ['combustor-ports','Cut the radial dilution ports','geometry'],
  ['core-combustor','Install the perforated combustor','assembly'],
  ['turbine-1','Build the high-pressure turbine row','assembly'],
  ['turbine-2','Build the low-pressure turbine row','assembly'],
  ['nacelle-profile','Revolve the outer nacelle profile','geometry'],
  ['casing','Install casing sections and bypass struts','assembly'],
  ['complete','Add the exhaust nozzle','assembly'],
].map(([id,label,kind])=>({id,label,kind}));
const ref=param=>({param});
const add=(a,b)=>({op:'add',a,b});

export function turbofan(stage='complete',{fanRadius=140,combustorPortRadius=5,shaftRadius=8}={}){
  if(!stages.some(s=>s.id===stage)||!(fanRadius>=120&&fanRadius<=150)||!(combustorPortRadius>=2&&combustorPortRadius<=7)||!(shaftRadius>=6&&shaftRadius<=8.5))throw Error('Invalid turbofan stage or parameters');
  const features=[],assembly=[];const at=stages.findIndex(s=>s.id===stage);
  const f=(id,op,args)=>(features.push({id,op,...args}),id);
  const cyl=(id,r,z,h)=>f(id,'cylinder',{origin:[0,0,z],radius:r,height:h});
  const cut=(id,left,right)=>f(id,'cut',{left,right});
  const pose=(id,input,translation=[0,0,0],rotation_deg=[0,0,0])=>f(id,'place',{input,translation,rotation_deg});
  const ring=(id,r0,r1,z,h)=>f(id,'revolve',{profile:{kind:'polygon',points:[[r0,z],[r1,z],[r1,z+h],[r0,z+h]]},frame:{origin:[0,0,0],x_axis:[1,0,0],y_axis:[0,0,1]},axis_origin:[0,0,0],axis:[0,0,1],angle_deg:360});
  const revolved=(id,points)=>f(id,'revolve',{profile:{kind:'polygon',points},frame:{origin:[0,0,0],x_axis:[1,0,0],y_axis:[0,0,1]},axis_origin:[0,0,0],axis:[0,0,1],angle_deg:360});
  const instance=(from,id,input,translation=[0,0,0],rotation_deg=[0,0,0])=>{if(at>=stages.findIndex(s=>s.id===from))assembly.push({id,input,translation,rotation_deg});};
  const disk=(id,r,z)=>cut(id,cyl(id+'Stock',r,z,10),cyl(id+'Bore',9,z-1,12));
  function blade(id,rootR,tipR,chord,thickness,z,twist=28){
    const profile=(c,t)=>({kind:'polygon',points:[[-.5*c,0],[-.32*c,-.5*t],[.22*c,-.42*t],[.5*c,0],[.18*c,.5*t],[-.3*c,.5*t]]});
    return f(id,'profile_loft',{degree:2,sections:[0,.5,1].map((u)=>{
      const angle=(twist*(1-u)-8)*Math.PI/180;
      const r=u===1?tipR:u===0?rootR:typeof tipR==='number'?(rootR+tipR)/2:{op:'div',a:add(rootR,tipR),b:2};
      return {profile:profile(chord*(1-.2*u),thickness*(1-.45*u)),frame:{origin:[r,10*u,z+8*u],x_axis:[0,Math.cos(angle),Math.sin(angle)],y_axis:[0,-Math.sin(angle),Math.cos(angle)]}};
    })});
  }
  const fanStock=cyl('fanHubStock',51,0,20),fanHole=cyl('fanHubTool',9,-1,22),fanHub=cut('fanHub',fanStock,fanHole);
  const fanBlade=blade('fanBlade',52,ref('fanRadius'),35,6,8,46);
  instance('fan-row','fan-hub',fanHub);
  for(let i=0;i<16;i++)instance('fan-row','fan-blade-'+String(i+1).padStart(2,'0'),fanBlade,[0,0,0],[0,0,i*22.5]);
  const lowShaft=ring('lowShaft',3,ref('shaftRadius'),-12,420),highShaft=ring('highShaft',10,13,80,268);
  instance('shaft','shaft-low-pressure',lowShaft);instance('shaft','shaft-high-pressure',highShaft);
  const spinner=revolved('spinner',[[9,-40],[11,-40],[36,-10],[49,0],[9,0]]);
  instance('shaft','fan-spinner',spinner);
  for(let row=0;row<3;row++){
    const z=80+row*48,rootR=26+row*5,tipR=58-row*2;
    const hub=disk('compressorHub'+row,rootR,z),airfoil=blade('compressorBlade'+row,rootR+1,tipR,16,3.6,z+4,32-row*5);
    instance('compressor-'+(row+1),'compressor-'+(row+1)+'-disk',hub);
    for(let i=0;i<12;i++)instance('compressor-'+(row+1),'compressor-'+(row+1)+'-blade-'+(i+1),airfoil,[0,0,0],[0,0,i*30+row*12]);
  }
  const liner=ring('combustorLiner',65,70,230,70);
  const port=f('combustorPortTool','extrude',{profile:{kind:'circle',radius:ref('combustorPortRadius')},frame:{origin:[72,0,267],x_axis:[0,1,0],y_axis:[0,0,-1]},height:10});
  let combustor=liner;
  for(let i=0;i<8;i++)combustor=cut('combustorPort'+i,combustor,pose('portPlacement'+i,port,[0,0,0],[0,0,i*45]));
  instance('core-combustor','combustor-perforated-liner',combustor);
  for(let row=0;row<2;row++){
    const z=326+row*44,rootR=30-row*2,hub=disk('turbineHub'+row,rootR,z),airfoil=blade('turbineBlade'+row,rootR+1,56+row*3,18,4,z+5,-20);
    instance('turbine-'+(row+1),'turbine-'+(row+1)+'-disk',hub);
    for(let i=0;i<12;i++)instance('turbine-'+(row+1),'turbine-'+(row+1)+'-blade-'+(i+1),airfoil,[0,0,0],[0,0,i*30+row*15]);
  }
  const nacelle=revolved('nacelle',[[147,-18],[154,-22],[160,-8],[160,38],[151,150],[137,310],[128,400],[122,400],[132,310],[145,150],[153,35],[153,-5],[147,-12]]);
  const coreCase=revolved('coreCasing',[[61,58],[66,58],[72,210],[77,225],[77,307],[65,390],[61,390],[72,307],[72,225],[67,210]]);
  const aft=ring('aftSupportRing',61,67,395,12);
  instance('casing','nacelle-outer-shell',nacelle);instance('casing','core-casing',coreCase);instance('casing','aft-casing-ring',aft);
  const strut=blade('bypassStrut',70,141,12,7,170,0);
  for(let i=0;i<8;i++)instance('casing','bypass-strut-'+(i+1),strut,[0,0,0],[0,0,i*45]);
  const nozzle=revolved('exhaustNozzle',[[59,407],[64,407],[50,460],[43,480],[39,480],[46,458]]);
  instance('complete','nozzle-core-exhaust',nozzle);
  const single={'fan-stock':fanStock,'fan-bore':fanHub,'fan-blade':fanBlade,'combustor-blank':liner,'combustor-ports':combustor,'nacelle-profile':nacelle}[stage];
  const used=new Set(single?[single]:assembly.map(x=>x.input));
  for(const feature of [...features].reverse())if(used.has(feature.id))for(const key of ['input','left','right'])if(feature[key])used.add(feature[key]);
  return {version:1,units:'mm',parameters:{fanRadius,combustorPortRadius,shaftRadius},features:features.filter(f=>used.has(f.id)),...(single?{output:single}:{assembly})};
}
