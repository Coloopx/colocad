import {readFile,writeFile} from 'node:fs/promises';
import {CadClient} from '@coloopx/colocad';
const model=JSON.parse(await readFile('node_modules/@coloopx/colocad/skills/colocad/assets/plate-grid.json'));
const rows=[];
for(const [id,queryable,preview] of [['plain',false,null],['query',true,null],['preview',false,{width:320,height:240,chord_mm:1}],['query-preview',true,{width:320,height:240,chord_mm:1}],['query-fine',true,{width:320,height:240,chord_mm:.3}]]){
 const m=structuredClone(model);if(preview)m.preview=preview;
 const cad=new CadClient({outputRoot:'probe/'+id});const r=await cad.build(m,{queryable});rows.push({id,input:m,queryable,result:r});console.log(JSON.stringify({id,ok:r.ok,error:r.error}));
}
await writeFile('probe.json',JSON.stringify(rows,null,2));
