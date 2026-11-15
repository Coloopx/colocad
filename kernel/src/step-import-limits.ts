/**
 * [INPUT]: Host Config.stepImportLimits and bounded native CLI count policy.
 * [OUTPUT]: Validated single-read data snapshot and immutable native host arguments.
 * [POS]: SDK host boundary; limits are never copied into agent model JSON.
 * [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
 */
import type {StepImportLimits} from './types.js';
const ceilings={maxEntities:2_000_000,maxDefinitions:1024,maxOccurrences:4096,maxGroups:8192} as const;
export function stepImportArgs(value?:StepImportLimits):string[]{
  if(value===undefined)return [];
  if(value===null||typeof value!=='object'||Array.isArray(value))throw Error('stepImportLimits must be an object');
  const snapshot:Record<string,number>=Object.create(null);
  for(const [key,limit]of Object.entries(value)){
    const ceiling=Object.hasOwn(ceilings,key)?ceilings[key as keyof typeof ceilings]:undefined;
    if(!ceiling||!Number.isSafeInteger(limit)||limit<1||limit>ceiling)
      throw Error(`stepImportLimits.${key} must be an integer in 1..${ceiling??'supported ceiling'}`);
    snapshot[key]=limit;
  }
  return ['--step-import-limits',JSON.stringify(snapshot)];
}
