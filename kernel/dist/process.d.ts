/** [INPUT]: 原生可执行文件、独立输入及宿主reportBytes。[OUTPUT]: UTF8有界进程与结构化回执。
 * [POS]: SDK执行隔离。[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md */
import { type ExecutionProgress } from './progress.js';
import type { Receipt } from './types.js';
export type CadInput = string | Uint8Array;
export interface ProcessResult {
    code: number | null;
    stdout: string;
    stderr: string;
    timedOut: boolean;
    overflow: boolean;
    idleTimedOut?: boolean;
    aborted?: boolean;
    lastProgress?: ExecutionProgress;
    budgetMs?: number;
    outputBytes?: number;
    outputLimit?: number;
}
export declare function error(code: string, message: string, details?: Record<string, unknown>): Receipt;
export declare function execute(file: string, args: string[], input?: CadInput, wallMs?: number | null, maxOutputBytes?: number, options?: {
    idleMs?: number;
    signal?: AbortSignal;
    onProgress?: (event: ExecutionProgress) => void;
}): Promise<ProcessResult>;
export declare function receipt(result: ProcessResult): Receipt;
