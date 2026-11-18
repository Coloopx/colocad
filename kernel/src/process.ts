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
    let stderr = '', timedOut = false, idleTimedOut=false, aborted=false, overflow = false, outputBytes = 0;
    let lastProgress:ExecutionProgress|undefined,idleTimer:ReturnType<typeof setTimeout>|undefined;
    const stop=()=>child.kill('SIGKILL');
    const abort=()=>{aborted=true;stop();};
    const renew=()=>{clearTimeout(idleTimer);if(options.idleMs!==undefined)idleTimer=setTimeout(()=>{idleTimedOut=true;stop();},options.idleMs);};
    const observe=progressReader(event=>{lastProgress=event;renew();try{Promise.resolve(options.onProgress?.(event)).catch(()=>{});}catch{/* 观察者失败不改变计算结果。 */}});
    const chunks:Buffer[]=[];
    const timer = wallMs===null?undefined:setTimeout(() => { timedOut = true; stop(); }, wallMs);
    const cleanup=()=>{clearTimeout(timer);clearTimeout(idleTimer);options.signal?.removeEventListener('abort',abort);};
    renew();options.signal?.addEventListener('abort',abort,{once:true});if(options.signal?.aborted)abort();
    child.on('error', e => { cleanup(); reject(e); });
    child.stdout.on('data', b => {
      if(overflow)return;
      outputBytes+=b.length;
      if(outputBytes>maxOutputBytes){overflow=true;child.kill('SIGKILL');return;}
      chunks.push(Buffer.from(b));
    });
    child.stderr.on('data', b => { stderr = (stderr + b).slice(-8192);observe(b); });
    child.stdin.on('error', () => {});
    child.on('close', code => { cleanup(); done({ code, idleTimedOut, aborted, lastProgress, stdout:Buffer.concat(chunks).toString('utf8'), stderr, timedOut, overflow, outputBytes, outputLimit:maxOutputBytes }); });
    child.stdin.end(input);
  });
}

export function receipt(result: ProcessResult): Receipt {
  if (result.aborted) return error('CANCELLED','CAD command cancelled by its host');
  if (result.idleTimedOut) return {ok:false,error:{code:'IDLE_TIMEOUT',message:'CAD command stopped after no effective progress',feature_id:result.lastProgress?.feature_id??null,details:{last_progress:result.lastProgress??null}}};
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
