/** [INPUT]: 原生可执行文件、独立输入及宿主reportBytes。[OUTPUT]: UTF8有界进程与结构化回执。
 * [POS]: SDK执行隔离。[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md */
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
  outputBytes?: number;
  outputLimit?: number;
}
export function error(code: string, message: string, details?: Record<string, unknown>): Receipt {
  return { ok: false, error: { code, message, ...(details ? { details } : {}) } };
}

// Command arguments and input are separate; agent data never enters a shell string.
export function execute(file: string, args: string[], input: CadInput = '', wallMs = 30_000, maxOutputBytes = DEFAULT_RESOURCE_LIMITS.reportBytes): Promise<ProcessResult> {
  checkedLimit(maxOutputBytes,RESOURCE_LIMIT_CEILINGS.reportBytes,'reportBytes');
  return new Promise((done, reject) => {
    const child = spawn(file, args, { stdio: ['pipe', 'pipe', 'pipe'] });
    let stderr = '', timedOut = false, overflow = false, outputBytes = 0;
    const chunks:Buffer[]=[];
    const timer = setTimeout(() => { timedOut = true; child.kill('SIGKILL'); }, wallMs);
    child.on('error', e => { clearTimeout(timer); reject(e); });
    child.stdout.on('data', b => {
      if(overflow)return;
      outputBytes+=b.length;
      if(outputBytes>maxOutputBytes){overflow=true;child.kill('SIGKILL');return;}
      chunks.push(Buffer.from(b));
    });
    child.stderr.on('data', b => { stderr = (stderr + b).slice(-8192); });
    child.stdin.on('error', () => {});
    child.on('close', code => { clearTimeout(timer); done({ code, stdout:Buffer.concat(chunks).toString('utf8'), stderr, timedOut, overflow, outputBytes, outputLimit:maxOutputBytes }); });
    child.stdin.end(input);
  });
}

export function receipt(result: ProcessResult): Receipt {
  if (result.timedOut) return error('TIMEOUT', 'CAD command exceeded wall-clock budget');
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
