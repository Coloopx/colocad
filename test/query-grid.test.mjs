import test from 'node:test';
import {mkdtemp,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {CadClient} from '../src/index.mjs';
import {ModelingSession} from '../src/session.mjs';
import {checkGridQuery} from './validation/grid-query.mjs';

test('bundled face grid survives SDK query, dimension/count edits, relocation and CLI replay',async t=>{
  const temp=await mkdtemp(join(tmpdir(),'colocad-grid-'));
  t.after(()=>rm(temp,{recursive:true,force:true}));
  await checkGridQuery({CadClient,ModelingSession,pkg:fileURLToPath(new URL('..',import.meta.url)),out:join(temp,'checks')});
});
