import {ModelingSession} from '/pkg/src/session.mjs';
import {writeFile} from 'node:fs/promises';
const n=Number(process.argv[2]);
const session=new ModelingSession('/history/'+n);
const head=await session.latestGeometry();
if(head.id!=='s'+String(n).padStart(7,'0'))throw Error('wrong accepted head');
await writeFile('/out/result.json',JSON.stringify({ok:true,entries:n,latest:head.id,variant:'published SDK full JSON ledger read',synthetic_metadata:true}));
