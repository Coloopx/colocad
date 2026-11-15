/**
 * [INPUT]: Host Config.stepImportLimits and bounded native CLI count policy.
 * [OUTPUT]: Validated immutable STEP import settings and native host arguments.
 * [POS]: SDK host boundary; limits are never copied into agent model JSON.
 * [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
 */
import type { StepImportLimits } from './types.js';
export declare function stepImportArgs(value?: StepImportLimits): string[];
