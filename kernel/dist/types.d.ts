export type * from './generated.js';
export type { Request as QueryRequest, TopologyRef, Selection, Filter as QueryFilter, Derive as DatumDerivation } from './query-generated.js';
import type { Scalar } from './generated.js';
export type V2 = [Scalar, Scalar];
export type V3 = [Scalar, Scalar, Scalar];
export type Receipt = {
    ok: true;
    data: Record<string, unknown>;
} | {
    ok: false;
    error: {
        code: string;
        message: string;
        feature_id?: string | null;
        query_id?: string;
        details?: Record<string, unknown>;
    };
};
export type Runner = {
    mode: 'native';
    binary?: string;
};
export interface Config {
    outputRoot: string;
    binary?: string;
    wallMs?: number;
    /** Host-owned allowlist for querying reopened artifacts; defaults to outputRoot. */
    queryRoots?: string[];
    /** @deprecated 0.4 local binary configuration. */
    runner?: Runner;
}
export interface BuildOptions {
    queryable?: boolean;
    previousRevision?: string;
}
