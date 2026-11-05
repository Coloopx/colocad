import type { Model } from './types.js';
/** Register STEP content under an opaque ID; agents never select filesystem paths. */
export declare function attachStep(model: Model, id: string, step: string): Model;
