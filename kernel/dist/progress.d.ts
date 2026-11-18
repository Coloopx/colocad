export type ExecutionProgress = {
    schema: 'colocad.progress/1';
    seq: number;
    phase: 'command' | 'feature' | 'validation' | 'check' | 'export';
    state: 'started' | 'completed';
    feature_id?: string;
};
export declare function progressReader(accept: (event: ExecutionProgress) => void): (chunk: Buffer) => void;
/** Host defaults for GUI/CLI adapters launching a native command in their sandbox. */
export declare function executionPolicyFromEnv(env: NodeJS.ProcessEnv): {
    idleMs: number;
    wallMs: number | undefined;
    onProgress: ((event: ExecutionProgress) => void) | undefined;
};
