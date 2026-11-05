import { CadClient, type Model, type Receipt } from './client.js';
export declare function createCadTools(client: CadClient): Promise<({
    name: string;
    description: string;
    inputSchema: {
        type: string;
        properties: {};
        additionalProperties: boolean;
    };
    execute: () => Promise<Receipt>;
} | {
    name: string;
    description: string;
    inputSchema: {
        type: string;
        properties: Record<string, unknown>;
        required: string[];
        additionalProperties: boolean;
        $defs: unknown;
    };
    execute: (a: {
        model: Model;
        options?: import("./types.js").PreviewOptions;
    }) => Promise<Receipt>;
} | {
    name: string;
    description: string;
    inputSchema: {
        type: string;
        properties: Record<string, unknown>;
        required: string[];
        additionalProperties: boolean;
        $defs: unknown;
    };
    execute: (a: {
        sketch: import("./types.js").Sketch;
        parameters?: Record<string, number>;
    }) => Promise<Receipt>;
} | {
    name: string;
    description: string;
    inputSchema: {
        type: string;
        properties: Record<string, unknown>;
        required: string[];
        additionalProperties: boolean;
        $defs: unknown;
    };
    execute: (a: {
        model: Model;
        asset: string;
    }) => Promise<Receipt>;
} | {
    name: string;
    description: string;
    inputSchema: {
        type: string;
        properties: Record<string, unknown>;
        required: string[];
        additionalProperties: boolean;
        $defs: unknown;
    };
    execute: (a: {
        model: Model;
        parameters: Record<string, number>;
    }) => Promise<Receipt>;
})[]>;
