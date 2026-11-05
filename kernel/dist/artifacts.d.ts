import type { Config, Receipt } from './types.js';
/** Publish a new revision on success; remove failed jobs even if validation throws. */
export declare function artifactJob(config: Config, prefix: string, action: (job: string, destination: string) => Promise<Receipt>): Promise<Receipt>;
