// Generic installation intent over independently authored source geometry.
// The unique blind key hole identifies the mounting side. No face indices,
// original source coordinates, or inferred thread compatibility are used.
import {attachStep} from '../../kernel/dist/index.js';
export const tolerance={linear_mm:1e-5,angular_rad:1e-7};
const filter=(input,where)=>({kind:'filter',input,where});
const entities=entity=>({kind:'entities',entity});
const children=(input,entity)=>({kind:'children',input,entity});
const ancestors=(input,entity)=>({kind:'ancestors',input,entity});
const range=value=>({min:value-1e-5,max:value+1e-5});
export function installationIntent(definition,diameter){
 const keyWall=filter(entities('face'),{surface_type:'cylinder',radius_mm:range(1.2)});
 const support=filter(ancestors(children(keyWall,'edge'),'face'),{surface_type:'plane',area_mm2:{min:1000}});
 const plane=filter(entities('face'),{planar_coplanar_with:{datum:'mountPlane'}});
 const circles=children(filter(children(plane,'loop'),{loop_role:'inner'}),'edge');
 const datum=(id,selection,kind)=>({id,definition,tolerance,derive:{kind:'selected',selection,derive:{kind}}});
 return {datums:[
  datum('mountPlane',support,'plane'),
  datum('registerCenter',filter(circles,{curve_type:'circle',radius_mm:range(5)}),'point'),
  datum('keyCenter',filter(circles,{curve_type:'circle',radius_mm:range(1.2)}),'point'),
  {id:'keyDirection',definition,tolerance,derive:{kind:'point_difference',from:'registerCenter',to:'keyCenter'}}
 ],interface:{id:'mount',definition,profile:'planar_hole_mount/v1',
  selectors:{plane:{where:{surface_type:'plane',planar_coplanar_with:{datum:'mountPlane'}},cardinality:{min:1,max:1}},hole_group:{opening_on:'plane',where:{pattern:'rectangular',member_count:4,equal_diameter:true},cardinality:{min:1,max:1}}},
  datum:{origin:{kind:'hole_group_center'},normal:'outward',x_axis:{kind:'orientation_datum',name:'key'}},
  orientation_datums:[{kind:'geometric_datum',name:'key',datum:'keyDirection'}],
  requirements:{verification_tolerance:tolerance,pattern:{kind:'rectangular',count:[2,2],pitch_mm:[24,20]},bore_diameter_mm:diameter,termination:'through'}
 }};
}
export function model(motorStep,supportStep){
 const motor=installationIntent('motor',4),support=installationIntent('support',4.4);
 let m={version:1,units:'mm',parameters:{},features:[{id:'motor',op:'import_step',asset:'motorSource',body_index:0},{id:'support',op:'import_step',asset:'supportSource',body_index:0}],datums:[...motor.datums,...support.datums],interfaces:[motor.interface,support.interface],
  assembly:[{id:'supportPart',input:'support'},{id:'motorPart',input:'motor',rotation_deg:[180,0,0]}],
  mates:{anchors:['supportPart'],constraints:[{id:'mount',kind:'fixed',alignment:'opposed',a:{instance:'supportPart',interface:'mount'},b:{instance:'motorPart',interface:'mount'}}]},
  checks:[{kind:'interface_compatibility',a:{instance:'motorPart',interface:'mount'},b:{instance:'supportPart',interface:'mount'},plane_relation:'opposed',correspondence:{kind:'pattern',require_all:true,symmetry:'require_unique'},tolerance,requirements:{max_plane_gap_mm:0.02,max_hole_axis_offset_mm:0.05,hole_size:{kind:'independent_ranges',a_diameter_mm:{min:3.99,max:4.01},b_diameter_mm:{min:4.39,max:4.41}}}},
   {kind:'geometric_relationship',a:{instance:'motorPart'},b:{instance:'supportPart'},tolerance,max_overlap_mm3:1e-7}]
 };
 m=attachStep(m,'motorSource',motorStep);return attachStep(m,'supportSource',supportStep);
}
