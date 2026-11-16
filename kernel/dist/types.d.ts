/** [INPUT]: 原生Schema及宿主字节合同。[OUTPUT]: 公开SDK类型。
 * [POS]: 配置与模型严格分离。[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md */
export type * from './generated.js';
export type { ResourceLimits } from './resource-limits.js';
import type { ResourceLimits } from './resource-limits.js';
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
    /** Host-only input, independent STEP output and reportBytes budgets. Never read from a model. */
    resourceLimits?: ResourceLimits;
    /** Host-owned allowlist for querying reopened artifacts; defaults to outputRoot. */
    queryRoots?: string[];
    /** @deprecated 0.4 local binary configuration. */
    runner?: Runner;
}
export interface BuildOptions {
    queryable?: boolean;
    previousRevision?: string;
}
