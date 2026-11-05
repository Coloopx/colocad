export type * from './generated.js';
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
    /** @deprecated 0.4 local binary configuration. */
    runner?: Runner;
}
