/** [INPUT]: 当前reportBytes、显示网格和SHA报告。[OUTPUT]: 有界报告/SHA校验的显示缓存。
 * [POS]: 仅报告读取改用宿主预算，网格原24MiB显示规则独立。
 * [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md */
import { createHash } from 'node:crypto';
import { readFile, stat } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import type { Model, PreviewOptions } from './types.js';
import {readReportFile} from './files.js';
import {DEFAULT_RESOURCE_LIMITS} from './resource-limits.js';

export class CacheError extends Error {
  constructor(readonly code: string, message: string) { super(message); }
}
export function previewKey(model: Model): string {
  const geometry = { ...model, preview: { pose: model.preview?.pose ?? 'normal', chord_mm: model.preview?.chord_mm ?? 0.2 } };
  return createHash('sha256').update(JSON.stringify(geometry)).digest('hex');
}
export async function readPreviewCache(directory: string, options: Partial<PreviewOptions>, reportBytes=DEFAULT_RESOURCE_LIMITS.reportBytes) {
  const source = resolve(directory);
  const path = join(source, 'preview.mesh.json');
  if ((await stat(path)).size > 24 * 1024 * 1024) throw new CacheError('PREVIEW_BUDGET', 'mesh cache exceeds 24 MiB');
  const raw = await readFile(path);
  const report = JSON.parse((await readReportFile({resourceLimits:{reportBytes}},join(source, 'report.json'))).toString('utf8'));
  const expected = report.artifact_sha256?.['preview.mesh.json'];
  if (!expected || createHash('sha256').update(raw).digest('hex') !== expected
    || typeof report.model_hash !== 'string' || !report.preview) {
    throw new CacheError('INVALID_MESH_CACHE', 'cache does not match its source revision');
  }
  return { raw, source, sourceHash: report.model_hash as string,
    options: { pose: report.preview.pose, chord_mm: report.preview.chord_mm, ...options } };
}
