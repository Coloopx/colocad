import type { Config, Model, Receipt, PreviewOptions, Sketch, BuildOptions, QueryRequest } from './types.js';
import { type ByteLimits } from './resource-limits.js';
export type * from './types.js';
export { attachStep } from './assets.js';
/** Local CAD API for the Harness and Agent sharing this sandbox. */
export declare class CadClient {
    private readonly config;
    private readonly byteLimits;
    private readonly explicitByteLimits;
    private previousRevision?;
    private previousQueryable;
    private readonly previewCache;
    constructor(config: Config);
    /** Immutable host policy shared by trusted file codecs and native requests. */
    get resourceLimits(): Readonly<ByteLimits>;
    capabilities(): Promise<Receipt>;
    schema(): Promise<Receipt>;
    querySchema(): Promise<Receipt>;
    query(directory: string, request: QueryRequest): Promise<Receipt>;
    validate(model: Model): Promise<Receipt>;
    solveSketch(sketch: Sketch, parameters?: Record<string, number>): Promise<Receipt>;
    inspectStep(model: Model, asset: string): Promise<Receipt>;
    preview(model: Model, options?: Partial<PreviewOptions>): Promise<Receipt>;
    /** Resume display from a trusted host artifact path, including in a new SDK process. */
    observeArtifact(directory: string, options?: Partial<PreviewOptions>): Promise<Receipt>;
    build(model: Model, optionsOrRevision?: BuildOptions | string): Promise<Receipt>;
    edit(model: Model, parameters: Record<string, number>): Promise<Receipt>;
    inspect(artifactDirectory: string): Promise<Record<string, unknown>>;
    private invoke;
}
