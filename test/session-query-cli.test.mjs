import test from 'node:test';
import {mkdtemp,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {checkSessionCli} from './validation/session-query-cli.mjs';

test('CLI records exact inspections, edits the accepted part and rebinds after relocation',async t=>{
  const root=await mkdtemp(join(tmpdir(),'colocad-query-cli-'));
  t.after(()=>rm(root,{recursive:true,force:true}));
  await checkSessionCli(fileURLToPath(new URL('..',import.meta.url)),root);
});
