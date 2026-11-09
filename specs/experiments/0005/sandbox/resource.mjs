import {readFile,readdir,writeFile} from 'node:fs/promises';
import {join} from 'node:path';
const read=async p=>(await readFile('/sys/fs/cgroup/'+p,'utf8')).trim();
const pairs=s=>Object.fromEntries(s.split('\n').map(l=>{const [k,v]=l.split(/\s+/);return[k,Number(v)];}));
export async function resources(label){
 const processes=[];
 for(const id of await readdir('/proc'))if(/^\d+$/.test(id))try{
  const status=await readFile('/proc/'+id+'/status','utf8'),comm=await readFile('/proc/'+id+'/comm','utf8');processes.push({pid:Number(id),name:comm.trim(),rss_kib:Number(status.match(/^VmRSS:\s+(\d+)/m)?.[1]??0)});
 }catch{}
 return {label,cpu_max:await read('cpu.max'),memory_max:Number(await read('memory.max')),memory_swap_max:await read('memory.swap.max'),memory_peak_bytes:Number(await read('memory.peak')),memory_current_bytes:Number(await read('memory.current')),memory_events:pairs(await read('memory.events')),memory_stat:pairs(await read('memory.stat')),cpu_stat:pairs(await read('cpu.stat')),processes,accounting:'Unified cgroup v2 includes every descendant and charged anonymous/file/cache pages. Process RSS is supplementary. External inference provider/gateway and independent OCCT verification excluded.'};
}
export async function saveResources(out,label){const value=await resources(label);await writeFile(join(out,'resources.json'),JSON.stringify(value,null,2)+'\n');return value;}
