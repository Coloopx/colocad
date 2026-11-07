// Catalog interfaces: HIWIN K99TE16-1906 p.22. Internal sections are reconstruction assumptions.
// X = width, Y = vertical, Z = travel; mm. See sources.json before using for integration.
export const specification={manufacturer:'HIWIN',series:'KK60 standard',configuration:'5 mm lead / 300 mm rail / A1 / open / no motor',railLength:300,width:60,carriageHeight:33,screwAxisHeight:23,length:370,stroke:210,lead:5,screwDiameter:12};
export const stages=[
 ['rail-stock','Extrude the U-section rail','geometry'],['rail-races','Cut the four guide raceways','geometry'],['rail-holes','Drill and counterbore the six mounting holes','geometry'],
 ['carriage-stock','Extrude the carriage saddle','geometry'],['carriage-bore','Machine the nut seat and ball tracks','geometry'],['carriage-holes','Drill the M5 mounting pattern and datum slots','geometry'],
 ['screw-blank','Revolve the stepped ballscrew blank','geometry'],['screw-groove','Cut 56 helical turns at 5 mm lead','geometry'],
 ['end-plate','Machine the end bearing plate','geometry'],['motor-bracket','Machine the motor support and register','geometry'],['coupling','Bore and split the coupling hubs','geometry'],
 ['base-assembly','Install the rail and end supports','assembly'],['screw-assembly','Install the ballscrew and support bearings','assembly'],['carriage-installed','Install the carriage and end caps','assembly'],
 ['rolling-assembly','Add the ball nut, guide balls and wipers','assembly'],['complete','Complete the actuator and mounting hardware','assembly'],
].map(([id,label,kind])=>({id,label,kind}));
const p=name=>({param:name}),mul=(a,b)=>({op:'mul',a,b}),add=(a,b)=>({op:'add',a,b});
export function kk60(stage='complete',{travel=0}={}){
 if(!stages.some(s=>s.id===stage))throw Error('Unknown KK60 stage');
 if(!Number.isFinite(travel)||Math.abs(travel)>105)throw Error('travel must be within -105..105 mm');
 const features=[],all=[],f=(id,op,args)=>(features.push({id,op,...args}),id);
 const box=(id,origin,size)=>f(id,'box',{origin,size});
 const cut=(id,left,right)=>f(id,'cut',{left,right});
 const extrude=(id,points,z,height)=>f(id,'extrude',{profile:{kind:'polygon',points},frame:{origin:[0,0,z],x_axis:[1,0,0],y_axis:[0,1,0]},height});
 const ring=(id,ro,ri,z,h,y=0)=>f(id,'extrude',{profile:{kind:'circle',radius:ro},holes:[{kind:'circle',radius:ri}],frame:{origin:[0,y,z],x_axis:[1,0,0],y_axis:[0,1,0]},height:h});
 const holes=(id,input,positions,r,depth,origin=[0,0,0],axes=[[1,0,0],[0,1,0]],counterbore)=>f(id,'holes',{input,positions,radius:r,depth,frame:{origin,x_axis:axes[0],y_axis:axes[1]},...(counterbore?{counterbore}:{})});
 const faceholes=(id,input,face,positions,r,depth,counterbore)=>f(id,'face_holes',{input,face,positions,radius:r,depth,...(counterbore?{counterbore}:{})});
 const inst=(id,input,translation=[0,0,0],rotation_deg=[0,0,0],phase='complete')=>all.push({id,input,translation,rotation_deg,phase});
 const trackPositions=[[-20.5,12],[-20.5,20],[20.5,12],[20.5,20]];
 const railStock=extrude('railStock',[[-30,0],[30,0],[30,21],[29,22],[21,22],[21,5],[-21,5],[-21,22],[-29,22],[-30,21]],7,300);
 const railRaces=holes('railRaceways',railStock,trackPositions,1.65,302,[0,0,6]);
 // Recessed rail floor is Y=5; explicit inward plane avoids drilling from the top of the sidewalls.
 const railHoles=holes('railMounting',railRaces,[-15,15].flatMap(x=>[57,157,257].map(z=>[x,z])),2.75,6,[0,5,0],[[1,0,0],[0,0,1]],{radius:4.75,depth:4.7});
 const rail=holes('railCoverTaps',railHoles,[-26,26].flatMap(x=>[57,257].map(z=>[x,z])),1.3,4,[0,22,0],[[1,0,0],[0,0,1]]);
 const carriageSection=[[-19.7,8],[19.7,8],[19.7,23],[21,25],[21,32],[20,33],[-20,33],[-21,32],[-21,25],[-19.7,23]];
 const carStock=extrude('carriageStock',carriageSection,-25.5,51);
 const carSeat=holes('carriageNutSeat',carStock,[[0,23]],9.2,53,[0,0,-26.5]);
 const carBore=holes('carriageRaceways',carSeat,trackPositions,1.65,53,[0,0,-26.5]);
 const carHoles=faceholes('carriageMounting',carBore,'pos_y',[-15,15].flatMap(x=>[-15,15].map(z=>[x,z])),2.5,8);
 let car=carHoles;
 for(const z of [-23,23])car=f('datumSlot'+(z<0?'A':'B'),'face_cutout',{input:car,face:'pos_y',center:[0,z],profile:{kind:'rectangle',width:15,depth:5},depth:1.5});
 const capStock=extrude('endCapStock',[[-19.5,8],[19.5,8],[21,25],[21,31.5],[-21,31.5],[-21,25]],0,15.5);
 const capBore=holes('endCapBore',capStock,[[0,23]],6.5,15.5,[0,0,0],undefined,{radius:8.6,depth:1.1});
 const cap=faceholes('endCapWiperSeat',capBore,'pos_z',[[0,23]],8.6,1.1);
 const wiper=ring('wiper',8.5,6.2,0,1,23);
 const nut=ring('ballNut',9,6.05,-21,42,23);
 const shaftBlank=f('shaftBlank','revolve',{profile:{kind:'polygon',points:[[0,-7],[6,-7],[6,286],[4,286],[4,306],[3,306],[3,340],[0,340]]},frame:{origin:[0,0,0],x_axis:[1,0,0],y_axis:[0,0,1]},axis_origin:[0,0,0],axis:[0,0,1],angle_deg:360});
 let screw=shaftBlank;
 // Four cubic lofts approximate a circular helical cutter, then exact native booleans.
 // It is NOT an exact analytical helix, gothic-arch ball race or OEM ground profile.
 for(let q=0;q<4;q++){
  const tool=f('helixQuarter'+q,'profile_loft',{degree:3,sections:Array.from({length:4},(_,i)=>{const theta=(q+i/3)*Math.PI/2;return {profile:{kind:'circle',radius:1},frame:{origin:[6.2*Math.cos(theta),6.2*Math.sin(theta),5+(q+i/3)*1.25],x_axis:[Math.cos(theta),Math.sin(theta),0],y_axis:[0,0,1]}};})});
  const pattern=f('helicalTools'+q,'pattern',{input:tool,positions:Array.from({length:56},(_,i)=>[0,0,i*5])});
  screw=cut('screwGroove'+q,screw,pattern);
 }
 const endStock=box('endPlateStock',[-29.8,.5,0],[59.6,44.5,11]);
 const endBore=holes('endBearingSeat',endStock,[[0,23]],8,13,[0,0,-1]);
 const endPlate=holes('endPlateScrews',endBore,[[-23,9],[23,9],[-23,35],[23,35]],1.5,11,[0,0,0],undefined,{radius:3,depth:3});
 const motorStock=box('motorSupportStock',[-29.8,.5,311],[59.6,44.5,59]);
 const motorWindow=f('motorSupportWindow','face_cutout',{input:motorStock,face:'pos_y',center:[0,340.5],profile:{kind:'rectangle',width:36,depth:39,radius:2},depth:41});
 const motorBore=holes('motorBearingSeat',motorWindow,[[0,23]],10,61,[0,0,310]);
 const motorRegister=faceholes('motorRegister',motorBore,'pos_z',[[0,23]],14,4);
 // Nominal threaded holes: two four-hole patterns on PCD40; no cosmetic thread mesh.
 const motorHoles=faceholes('motorM4Pattern',motorRegister,'pos_z',[45,135,225,315].map(a=>[20*Math.cos(a*Math.PI/180),23+20*Math.sin(a*Math.PI/180)]).filter(([x,y])=>y>.5),2,8);
 const motorM3=faceholes('motorM3Pattern',motorHoles,'pos_z',[30,150,210,330].map(a=>[20*Math.cos(a*Math.PI/180),23+20*Math.sin(a*Math.PI/180)]),1.5,8);
 const motor=faceholes('motorSupportBolts',motorM3,'pos_z',[-23,23].flatMap(x=>[9,35].map(y=>[x,y])),1.5,11,{radius:3,depth:3});
 const couplingStock=ring('couplingHubStock',10,3,0,8);
 const coupling=faceholes('couplingSetScrew',couplingStock,'pos_x',[[0,4]],1.5,7);
 const couplingB=ring('couplingDrivenHub',10,3,0,8);
 const spiderStock=extrude('couplingSpiderStock',Array.from({length:24},(_,i)=>{const a=i*Math.PI/12,r=i%4<2?9.8:6;return [r*Math.cos(a),r*Math.sin(a)];}),0,3);
 const spider=holes('couplingSpider',spiderStock,[[0,0]],3.2,5,[0,0,-1]);
 const boltStock=f('boltStock','revolve',{profile:{kind:'polygon',points:[[0,0],[1.4,0],[1.4,8],[2.7,8],[2.7,10.5],[0,10.5]]},frame:{origin:[0,0,0],x_axis:[1,0,0],y_axis:[0,0,1]},axis_origin:[0,0,0],axis:[0,0,1],angle_deg:360});
 const bolt=f('socketBolt','face_cutout',{input:boltStock,face:'pos_z',center:[0,0],profile:{kind:'polygon',points:Array.from({length:6},(_,i)=>[1.3*Math.cos(i*Math.PI/3),1.3*Math.sin(i*Math.PI/3)])},depth:1.8});
 const smallInner=ring('tailInnerRace',6.4,6,0,5),smallOuter=ring('tailOuterRace',8,7.2,0,5);
 const driveInner=ring('driveInnerRace',5.6,4,0,7),driveOuter=ring('driveOuterRace',10,8.4,0,7);
 const guideBall=f('guideBall','sphere',{center:[0,0,0],radius:1.5});
 const bearingBall=f('bearingBall','sphere',{center:[0,0,0],radius:1.3});
 const tailBall=f('tailBall','sphere',{center:[0,0,0],radius:.35});
 inst('rail-machined',rail,undefined,undefined,'base-assembly');inst('end-bearing-plate',endPlate,undefined,undefined,'base-assembly');inst('motor-bearing-support',motor,undefined,undefined,'base-assembly');
 inst('ballscrew-5mm-lead',screw,[0,23,11],[0,0,mul(p('travel'),72)],'screw-assembly');
 inst('bearing-tail-inner',smallInner,[0,23,4],undefined,'screw-assembly');inst('bearing-tail-outer',smallOuter,[0,23,4],undefined,'screw-assembly');
 inst('bearing-drive-inner',driveInner,[0,23,311],undefined,'screw-assembly');inst('bearing-drive-outer',driveOuter,[0,23,311],undefined,'screw-assembly');
 for(let i=0;i<12;i++){const a=i*Math.PI/6;inst('bearing-drive-ball-'+i,bearingBall,[7*Math.cos(a),23+7*Math.sin(a),314.5],undefined,'screw-assembly');inst('bearing-tail-ball-'+i,tailBall,[6.8*Math.cos(a),23+6.8*Math.sin(a),6.5],undefined,'screw-assembly');}
 const carriageZ=add(157,p('travel'));
 inst('carriage-machined',car,[0,0,carriageZ],undefined,'carriage-installed');
 inst('carriage-end-cap-a',cap,[0,0,add(carriageZ,-41)],undefined,'carriage-installed');inst('carriage-end-cap-b',cap,[0,0,add(carriageZ,25.5)],undefined,'carriage-installed');
 inst('ball-nut',nut,[0,0,carriageZ],undefined,'rolling-assembly');inst('seal-wiper-a',wiper,[0,0,add(carriageZ,-41)],undefined,'rolling-assembly');inst('seal-wiper-b',wiper,[0,0,add(carriageZ,40)],undefined,'rolling-assembly');
 for(let row=0;row<4;row++)for(let i=0;i<8;i++)inst('guide-ball-'+row+'-'+i,guideBall,[...trackPositions[row],add(carriageZ,-21+i*6)],undefined,'rolling-assembly');
 inst('coupling-driven-hub',coupling,[0,23,330]);inst('coupling-spider',spider,[0,23,338]);inst('coupling-motor-hub',couplingB,[0,23,341]);
 for(const x of [-23,23])for(const y of [9,35]){inst('bolt-tail-'+x+'-'+y,bolt,[x,y,10.5],[180,0,0]);inst('bolt-drive-'+x+'-'+y,bolt,[x,y,359.5]);}
 const single={'rail-stock':railStock,'rail-races':railRaces,'rail-holes':rail,'carriage-stock':carStock,'carriage-bore':carBore,'carriage-holes':car,'screw-blank':shaftBlank,'screw-groove':screw,'end-plate':endPlate,'motor-bracket':motor,'coupling':coupling}[stage];
 const at=stages.findIndex(s=>s.id===stage);
 let assembly=all.filter(x=>stages.findIndex(s=>s.id===x.phase)<=at).map(({phase,...x})=>x);
 // Separate inspection revision: carriage envelope at travel endpoints, before installing into whole assembly.
 if(stage==='carriage-installed')assembly=assembly.filter(x=>x.id.startsWith('carriage-'));
 const needed=new Set(single?[single]:assembly.map(x=>x.input));
 for(const feat of [...features].reverse())if(needed.has(feat.id))for(const key of ['input','left','right'])if(feat[key])needed.add(feat[key]);
 return {version:1,units:'mm',parameters:{travel},features:features.filter(x=>needed.has(x.id)),...(single?{output:single}:{assembly})};
}
