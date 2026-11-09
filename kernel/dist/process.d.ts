import type { Receipt } from './types.js';
export type CadInput = string | Uint8Array;
export interface ProcessResult {
    code: number | null;
    stdout: string;
    stderr: string;
    timedOut: boolean;
    overflow: boolean;
}
export declare function error(code: string, message: string): Receipt;
export declare function execute(file: string, args: string[], input?: CadInput, wallMs?: number, maxOutputBytes?: number): Promise<ProcessResult>;
export declare function receipt(result: ProcessResult): Receipt;
