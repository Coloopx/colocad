import { createHash } from 'node:crypto';
import type { Model } from './types.js';

/** Register STEP content under an opaque ID; agents never select filesystem paths. */
export function attachStep(model: Model, id: string, step: string): Model {
  if (!/^[A-Za-z0-9_-]{1,64}$/.test(id)) throw Error('invalid asset ID');
  if (Object.hasOwn(model.assets ?? {}, id)) throw Error('asset ID already exists');
  if (Buffer.byteLength(step) > 24 * 1024 * 1024) throw Error('STEP exceeds 24 MiB');
  return { ...structuredClone(model), assets: { ...model.assets, [id]: { step, sha256: createHash('sha256').update(step).digest('hex') } } };
}
