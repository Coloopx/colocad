export type Vec3 = [number,number,number];
type Feature = {id:string} & (
  {op:'profile_loft'; sections:Vec3[][]} |
  {op:'shell'; input:string; thickness:number; openFace:{plane:'XY'|'XZ'|'YZ'; at:number}} |
  {op:'hollow'; input:string; thickness:number} |
  {op:'box'; min:Vec3; max:Vec3} | {op:'sphere'; center?:Vec3; radius:number} |
  {op:'cone'|'cylinder'; base?:Vec3; axis?:Vec3; height:number; radius:number} |
  {op:'cut'|'union'|'intersect'|'mesh_cut'|'mesh_union'|'mesh_intersect'; input:string;tool:string} |
  {op:'import_step';path:string} | {op:'import_mesh';path:string;units?:'mm'|'cm'|'m'|'in'} |
  {op:'faceted_brep';input:string}
);
export interface HybridModel {
  format:'colocad/hybrid/1';units:'mm';title?:string;chordToleranceMm?:number;facetedStep?:boolean;
  features:Feature[];
  instances:{id:string;input:string;translate?:Vec3;rotate?:{axis:Vec3;center?:Vec3;degrees:number};
    material?:{color?:string;opacity?:number}}[];
  checks?:{a:string;b:string;maxOverlapMm3?:number}[];
  inspections?:({instance:string}&(
    {op:'wall_probe';from:Vec3;to:Vec3;minMm:number;maxMm:number} |
    {op:'slice';zMm:number} | {op:'overhang';bedZ:number;angleDeg:number}
  ))[];
}
export interface HybridReceipt {
  ok:boolean;format:'colocad/hybrid/1';units:'mm';chordToleranceMm:number;assemblyStep:string|null;
  features:{id:string;op:string;representation:'brep'|'mesh';volumeMm3:number}[];
  instances:{id:string;input:string;representation:'brep'|'mesh';triangles:number;stl:string;step:string|null}[];
  checks:{a:string;b:string;method:'brep-common'|'triangle-solid-common';overlapMm3:number;maxOverlapMm3:number;passed:boolean}[];
  inspections:Array<Record<string,unknown>>;
}
export function buildHybrid(spec:HybridModel,destination:string,options?:{assetRoot?:string}):Promise<HybridReceipt>;
