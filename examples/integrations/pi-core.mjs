import { Agent } from '@earendil-works/pi-agent-core';
import { createPiCadTools } from './pi-tools.mjs';

// Supply your harness's existing model and stream function.
export async function createCadAgent({ model, streamFn, getApiKey, outputRoot }) {
  return new Agent({
    initialState: {
      model,
      systemPrompt: 'Build parametric mechanical CAD from the requested dimensions in mm. '
        + 'Use cad_capabilities to inspect available operations. Validate the model, '
        + 'then build and inspect its geometry. Return the saved model and STEP paths. '
        + 'Treat reported failures as failed builds and correct the indicated feature.',
      tools: await createPiCadTools({ outputRoot, wallMs: 30_000 }),
    },
    streamFn,
    getApiKey,
    toolExecution: 'sequential',
  });
}
