import { spawn } from 'node:child_process';
import type { Receipt } from './types.js';

export type CadInput = string | Uint8Array;
export interface ProcessResult {
  code: number | null;
  stdout: string;
  stderr: string;
  timedOut: boolean;
  overflow: boolean;
}
export function error(code: string, message: string): Receipt {
  return { ok: false, error: { code, message } };
}

// Command arguments and input are separate; agent data never enters a shell string.
export function execute(file: string, args: string[], input: CadInput = '', wallMs = 30_000): Promise<ProcessResult> {
  return new Promise((done, reject) => {
    const child = spawn(file, args, { stdio: ['pipe', 'pipe', 'pipe'] });
    let stdout = '', stderr = '', timedOut = false, overflow = false;
    const timer = setTimeout(() => { timedOut = true; child.kill('SIGKILL'); }, wallMs);
    child.on('error', e => { clearTimeout(timer); reject(e); });
    child.stdout.on('data', b => {
      stdout += b;
      if (Buffer.byteLength(stdout) > 2_097_152) { overflow = true; child.kill('SIGKILL'); }
    });
    child.stderr.on('data', b => { stderr = (stderr + b).slice(-8192); });
    child.stdin.on('error', () => {});
    child.on('close', code => { clearTimeout(timer); done({ code, stdout, stderr, timedOut, overflow }); });
    child.stdin.end(input);
  });
}

export function receipt(result: ProcessResult): Receipt {
  if (result.timedOut) return error('TIMEOUT', 'CAD command exceeded wall-clock budget');
  if (result.overflow) return error('OUTPUT_LIMIT', 'CAD response exceeded 2 MiB');
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
