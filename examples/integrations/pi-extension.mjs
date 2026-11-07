import { resolve } from 'node:path';
import { createPiCadTools } from './pi-tools.mjs';
import { ModelingSession } from '../../src/session.mjs';

// Run Pi in the project directory. An explicit path may be set by the host.
export default async function colocadExtension(pi) {
  const outputRoot = resolve(process.env.COLOCAD_OUTPUT_DIR ?? './cad-output');
  let session;
  if (process.env.COLOCAD_SESSION_DIR) {
    const directory=resolve(process.env.COLOCAD_SESSION_DIR);
    try { session=await ModelingSession.create(directory); }
    catch(error) { if(error.code!=='EEXIST')throw error; session=await ModelingSession.open(directory); }
  }
  for (const tool of await createPiCadTools({ outputRoot, wallMs: 30_000 }, { session })) {
    pi.registerTool(tool);
  }
}
