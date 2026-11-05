import type { Model, PreviewOptions } from './types.js';
export declare class CacheError extends Error {
    readonly code: string;
    constructor(code: string, message: string);
}
export declare function previewKey(model: Model): string;
export declare function readPreviewCache(directory: string, options: Partial<PreviewOptions>): Promise<{
    raw: NonSharedBuffer;
    source: string;
    sourceHash: string;
    options: {
        chord_mm: any;
        height?: number | undefined;
        pose: any;
        section?: ((import("./generated.js").SectionPlane) | (null)) | undefined;
        view?: import("./generated.js").View | undefined;
        width?: number | undefined;
    };
}>;
