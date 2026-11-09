import { CadClient, createCadTools } from '../../src/index.mjs';
import { readFile, stat } from 'node:fs/promises';
import { join } from 'node:path';
import {compactReceipt} from './compact-tools.mjs';
import { createSessionTools } from '../../src/session.mjs';

// Reuse one client for a sequential modeling session to retain checkpoint/mesh reuse.
export async function createPiCadTools(config, { session } = {}) {
  const definitions = session ? await createSessionTools(session) : await createCadTools(new CadClient(config));
  const enabled = new Set(['cad_capabilities', 'cad_validate', 'cad_build', 'cad_preview', 'cad_edit', 'cad_query']);
  return definitions.filter((tool) => enabled.has(tool.name)).map((tool) => ({
    name: tool.name,
    label: tool.name.replaceAll('_', ' '),
    description: tool.description,
    parameters: tool.inputSchema,
    executionMode: 'sequential',
    async execute(_toolCallId, args, signal) {
      signal?.throwIfAborted();
      const receipt = await tool.execute(args);
      if (!receipt.ok) {
        // Pi exposes thrown errors to the model as failed tool results.
        throw new Error(JSON.stringify(receipt.error));
      }
      const data = receipt.data;
      const summary = compactReceipt(receipt).data;
      if(summary.feature_summary)summary.degradations=summary.feature_summary.degradations;
      const content = [{ type: 'text', text: JSON.stringify(summary) }];
      if (data.artifact_sha256?.['preview.png']) {
        const file = join(data.artifact_directory, 'preview.png');
        // Return a usable image to vision models, not just a path they cannot see.
        if ((await stat(file)).size <= 4 * 1024 * 1024) {
          content.push({ type: 'image', data: (await readFile(file)).toString('base64'), mimeType: 'image/png' });
        }
      }
      return {
        content,
        details: data,
      };
    },
  }));
}
