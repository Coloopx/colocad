import type { Model } from './types.js';
export interface StepRegistrationOptions {
    maxBytes?: number;
}
/** Register STEP content under an opaque ID; agents never select filesystem paths. */
export declare function attachStep(model: Model, id: string, step: string, options?: StepRegistrationOptions): Model;
