import { resolve } from 'node:path';
import { createPiCadTools } from './pi-tools.mjs';

// Run Pi in the project directory. An explicit path may be set by the host.
export default async function colocadExtension(pi) {
  const outputRoot = resolve(process.env.COLOCAD_OUTPUT_DIR ?? './cad-output');
  for (const tool of await createPiCadTools({ outputRoot, wallMs: 30_000 })) {
    pi.registerTool(tool);
  }
}
