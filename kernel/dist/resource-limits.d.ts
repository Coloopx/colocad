export interface ResourceLimits {
    inputBytes?: number;
    stepBytes?: number;
    totalStepBytes?: number;
    outputStepBytes?: number;
    reportBytes?: number;
}
export type ByteLimits = Required<ResourceLimits>;
export declare const DEFAULT_RESOURCE_LIMITS: Readonly<ByteLimits>;
export declare const RESOURCE_LIMIT_CEILINGS: Readonly<ByteLimits>;
export declare function checkedLimit(value: number, ceiling: number, resource?: keyof ByteLimits): number;
export declare function resourceLimits(value?: ResourceLimits): Readonly<ByteLimits>;
export declare class ResourceLimitError extends Error {
    readonly code = "RESOURCE_LIMIT";
    readonly details: Record<string, unknown>;
    constructor(resource: keyof ByteLimits | 'outputBytes', observed: number, limit: number, lowerBound?: boolean);
}
