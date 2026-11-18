export type ExecutionProgress = {
    schema: 'colocad.progress/1';
    seq: number;
    phase: 'command' | 'feature' | 'validation' | 'check' | 'export';
    state: 'started' | 'completed';
    feature_id?: string;
};
export declare function progressReader(accept: (event: ExecutionProgress) => void): (chunk: Buffer) => void;
