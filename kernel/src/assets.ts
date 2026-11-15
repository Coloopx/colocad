/**
 * [INPUT]: 宿主注册的STEP原文及可选有限单源预算。
 * [OUTPUT]: 不含路径的SHA绑定资产；默认24MiB，超限结构化拒绝。
 * [POS]: 公开资产注册边界，不修改模型几何或原生验证。
 * [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
 */
import { createHash } from 'node:crypto';
import type { Model } from './types.js';
import { DEFAULT_RESOURCE_LIMITS, RESOURCE_LIMIT_CEILINGS, checkedLimit, ResourceLimitError } from './resource-limits.js';
export interface StepRegistrationOptions { maxBytes?: number; }

/** Register STEP content under an opaque ID; agents never select filesystem paths. */
export function attachStep(model: Model, id: string, step: string, options: StepRegistrationOptions = {}): Model {
  if (!/^[A-Za-z0-9_-]{1,64}$/.test(id)) throw Error('invalid asset ID');
  if (Object.hasOwn(model.assets ?? {}, id)) throw Error('asset ID already exists');
  const limit = checkedLimit(options.maxBytes ?? DEFAULT_RESOURCE_LIMITS.stepBytes, RESOURCE_LIMIT_CEILINGS.stepBytes);
  const bytes = Buffer.byteLength(step);
  if (bytes > limit) throw new ResourceLimitError('stepBytes', bytes, limit);
  return { ...structuredClone(model), assets: { ...model.assets, [id]: { step, sha256: createHash('sha256').update(step).digest('hex') } } };
}
