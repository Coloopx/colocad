import { CadClient } from './client.js';
import type { Model } from './types.js';
/** Paths belong to the Harness in the current sandbox. Foreign STEP geometry is imported as an asset. */
export declare function readModelFile(cad: CadClient, path: string): Promise<Model>;
/** Export a new file. A STEP with embedModel=true can be reopened and edited from that single file. */
export declare function writeModelFile(cad: CadClient, model: Model, path: string, options?: {
    embedModel?: boolean;
}): Promise<{
    path: string;
    bytes: number;
    sha256: string;
    model_hash: string;
}>;
export declare function editModelFile(cad: CadClient, source: string, parameters: Record<string, number>, destination: string): Promise<{
    path: string;
    bytes: number;
    sha256: string;
    model_hash: string;
}>;
