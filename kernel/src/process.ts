/** [INPUT]: 原生可执行文件、独立输入及宿主reportBytes。[OUTPUT]: UTF8有界进程与结构化回执。
 * [POS]: SDK执行隔离。[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md */
import {progressReader,type ExecutionProgress} from './progress.js';
import { spawn } from 'node:child_process';
import type { Receipt } from './types.js';
import {DEFAULT_RESOURCE_LIMITS,RESOURCE_LIMIT_CEILINGS,checkedLimit,ResourceLimitError} from './resource-limits.js';

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
export function error(code: string, message: string, details?: Record<string, unknown>): Receipt {
  return { ok: false, error: { code, message, ...(details ? { details } : {}) } };
}

// Command arguments and input are separate; agent data never enters a shell string.
export function execute(file: string, args: string[], input: CadInput = '', wallMs:number|null = 30_000, maxOutputBytes = DEFAULT_RESOURCE_LIMITS.reportBytes, options:{idleMs?:number;signal?:AbortSignal;onProgress?:(event:ExecutionProgress)=>void}={}): Promise<ProcessResult> {
  for(const value of [wallMs,options.idleMs])if(value!==null&&value!==undefined&&(!Number.isSafeInteger(value)||value<1||value>2147483647))throw new RangeError('execution budget must be a positive timer integer');
  checkedLimit(maxOutputBytes,RESOURCE_LIMIT_CEILINGS.reportBytes,'reportBytes');
  return new Promise((done, reject) => {
    const child = spawn(file, args, { stdio: ['pipe', 'pipe', 'pipe'] });
    let stderr = '', outputBytes = 0;
    let reason: 'wall' | 'idle' | 'cancel' | 'output' | undefined;
    let lastProgress: ExecutionProgress | undefined;
    let idleTimer: ReturnType<typeof setTimeout> | undefined;
    const stop = (cause: NonNullable<typeof reason>) => {
      if (reason) return;
      reason = cause;
      child.kill('SIGKILL');
    };
    const abort = () => stop('cancel');
    if (options.idleMs !== undefined) idleTimer = setTimeout(() => stop('idle'), options.idleMs);
    const timer = wallMs === null ? undefined : setTimeout(() => stop('wall'), wallMs);
    const observe = options.idleMs !== undefined || options.onProgress ? progressReader(event => {
      if (reason) return;
      lastProgress = event;
      idleTimer?.refresh();
      if (options.onProgress) {
        try { Promise.resolve(options.onProgress(event)).catch(() => {}); }
        catch { /* Observer failures do not change the native receipt. */ }
      }
    }) : undefined;
    const cleanup = () => {
      clearTimeout(timer);
      clearTimeout(idleTimer);
      options.signal?.removeEventListener('abort', abort);
    };
    options.signal?.addEventListener('abort', abort, {once:true});
    if (options.signal?.aborted) abort();
    child.on('error', e => { cleanup(); reject(e); });
    const chunks: Buffer[] = [];
    child.stdout.on('data', b => {
      if (reason) return;
      outputBytes += b.length;
      if (outputBytes > maxOutputBytes) { stop('output'); return; }
      chunks.push(b);
    });
    child.stderr.on('data', b => { stderr = (stderr + b).slice(-8192); observe?.(b); });
    child.stdin.on('error', () => {});
    child.on('close', code => {
      cleanup();
      done({code, stdout:Buffer.concat(chunks).toString('utf8'), stderr, lastProgress,
        timedOut:reason === 'wall', idleTimedOut:reason === 'idle', aborted:reason === 'cancel',
        overflow:reason === 'output', outputBytes, outputLimit:maxOutputBytes,
        budgetMs:reason === 'idle' ? options.idleMs : wallMs ?? undefined});
    });
    child.stdin.end(input);
  });
}

export function receipt(result: ProcessResult): Receipt {
  if (result.aborted) return error('CANCELLED','CAD command cancelled by its host');
  if (result.idleTimedOut || result.timedOut) {
    return {ok:false,error:{
      code:result.idleTimedOut ? 'IDLE_TIMEOUT' : 'TIMEOUT',
      message:result.idleTimedOut ? 'CAD command stopped after no effective progress' : 'CAD command exceeded wall-clock budget',
      feature_id:result.lastProgress?.feature_id ?? null,
      details:{budget_ms:result.budgetMs, last_progress:result.lastProgress ?? null}
    }};
  }
  if (result.overflow) {
    const limit=result.outputLimit??DEFAULT_RESOURCE_LIMITS.reportBytes;
    const cause=new ResourceLimitError('reportBytes',result.outputBytes??limit+1,limit,true);
    return error(cause.code,cause.message,cause.details);
  }
  try {
    const value = JSON.parse(result.stdout) as Receipt;
    if (typeof value?.ok !== 'boolean'
      || (value.ok && (!value.data || typeof value.data !== 'object' || Array.isArray(value.data) || result.code !== 0))
      || (!value.ok && !value.error?.code)) throw Error('invalid receipt');
    return value;
  } catch {
    return error('WORKER_FAILED', `exit=${result.code}; ${result.stderr.slice(-1000)}`);
  }
}
