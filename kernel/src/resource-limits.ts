/**
 * [INPUT]: 宿主显式字节预算；模型不能提供或覆盖。
 * [OUTPUT]: 默认/硬上限、独立outputStepBytes及不可变输入/输出策略和结构化RESOURCE_LIMIT。
 * [POS]: 注册/原生传输/产物reader的宿主字节合同，对应Rust resources模块。
 * [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
 */
const MiB = 1024 * 1024;
export interface ResourceLimits {
  inputBytes?: number;
  stepBytes?: number;
  totalStepBytes?: number;
  outputStepBytes?: number;
}
export type ByteLimits = Required<ResourceLimits>;
export const DEFAULT_RESOURCE_LIMITS: Readonly<ByteLimits> = Object.freeze({
  inputBytes: 32 * MiB, stepBytes: 24 * MiB, totalStepBytes: 24 * MiB,
  outputStepBytes: 32 * MiB,
});
export const RESOURCE_LIMIT_CEILINGS: Readonly<ByteLimits> = Object.freeze({
  inputBytes: 512 * MiB, stepBytes: 256 * MiB, totalStepBytes: 512 * MiB,
  outputStepBytes: 512 * MiB,
});
const invalid = (message: string, details: Record<string, unknown> = {}) =>
  Object.assign(Error(message), { code: 'INVALID_RESOURCE_LIMITS', details });
export function checkedLimit(value: number, ceiling: number, resource: keyof ByteLimits = 'stepBytes'): number {
  if (!Number.isSafeInteger(value) || value < 1 || value > ceiling)
    throw invalid(`byte budget must be an integer in 1..${ceiling}`, { resource, requested_bytes: String(value), ceiling_bytes: ceiling });
  return value;
}
export function resourceLimits(value: ResourceLimits = {}): Readonly<ByteLimits> {
  if (!value || typeof value !== 'object' || Array.isArray(value) || Object.keys(value).some(k => !Object.hasOwn(DEFAULT_RESOURCE_LIMITS, k)))
    throw invalid('unknown or invalid resourceLimits');
  const result = { ...DEFAULT_RESOURCE_LIMITS, ...value };
  for (const key of Object.keys(result) as (keyof ByteLimits)[]) checkedLimit(result[key], RESOURCE_LIMIT_CEILINGS[key], key);
  if (result.stepBytes > result.totalStepBytes || result.totalStepBytes > result.inputBytes)
    throw invalid('resourceLimits require stepBytes <= totalStepBytes <= inputBytes');
  return Object.freeze(result);
}
export class ResourceLimitError extends Error {
  readonly code = 'RESOURCE_LIMIT';
  readonly details: Record<string, unknown>;
  constructor(resource: keyof ByteLimits | 'outputBytes', observed: number, limit: number, lowerBound = false) {
    const value = limit % MiB === 0 ? `${limit / MiB} MiB` : `${limit} bytes`;
    super(`${resource === 'stepBytes' ? 'STEP' : resource} exceeds ${value}`);
    this.details = { resource, observed_bytes: observed, limit_bytes: limit, observed_is_lower_bound: lowerBound };
  }
}
