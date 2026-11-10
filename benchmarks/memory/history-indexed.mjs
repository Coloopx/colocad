// Same Node/SDK import as history.mjs; only the metadata lookup changes.
import {ModelingSession} from '/pkg/src/session.mjs';
import {DatabaseSync} from 'node:sqlite';
import {writeFile} from 'node:fs/promises';
const n=Number(process.argv[2]);
const db=new DatabaseSync('/history/'+n+'/project.sqlite',{readOnly:true});
db.exec('PRAGMA cache_size=-2048; PRAGMA mmap_size=0; PRAGMA temp_store=FILE;');
const head=db.prepare('SELECT seq,record FROM events ORDER BY seq DESC LIMIT 1').get();
if(head.seq!==n||JSON.parse(head.record).id!=='s'+String(n).padStart(7,'0'))throw Error('wrong accepted head');
db.close();
await writeFile('/out/result.json',JSON.stringify({ok:true,entries:n,latest:head.seq,variant:'experimental indexed Node lookup; same SDK import',synthetic_metadata:true}));
