import type { Config, Model, Receipt, PreviewOptions, Sketch } from './types.js';
export type * from './types.js';
export { attachStep } from './assets.js';
/** Local CAD API for the Harness and Agent sharing this sandbox. */
export declare class CadClient {
    private readonly config;
    private previousRevision?;
    private readonly previewCache;
    constructor(config: Config);
    capabilities(): Promise<Receipt>;
    schema(): Promise<Receipt>;
    validate(model: Model): Promise<Receipt>;
    solveSketch(sketch: Sketch, parameters?: Record<string, number>): Promise<Receipt>;
    inspectStep(model: Model, asset: string): Promise<Receipt>;
    preview(model: Model, options?: Partial<PreviewOptions>): Promise<Receipt>;
    /** Resume display from a trusted host artifact path, including in a new SDK process. */
    observeArtifact(directory: string, options?: Partial<PreviewOptions>): Promise<Receipt>;
    build(model: Model, previousRevision?: string | undefined): Promise<Receipt>;
    edit(model: Model, parameters: Record<string, number>): Promise<Receipt>;
    inspect(artifactDirectory: string): Promise<Record<string, unknown>>;
    private invoke;
}
