import { createHash } from 'node:crypto';
import { readFile, stat } from 'node:fs/promises';
import { resolve, join } from 'node:path';
export class CacheError extends Error {
    code;
    constructor(code, message) {
        super(message);
        this.code = code;
    }
}
export function previewKey(model) {
    const geometry = { ...model, preview: { pose: model.preview?.pose ?? 'normal', chord_mm: model.preview?.chord_mm ?? 0.2 } };
    return createHash('sha256').update(JSON.stringify(geometry)).digest('hex');
}
export async function readPreviewCache(directory, options) {
    const source = resolve(directory);
    const path = join(source, 'preview.mesh.json');
    if ((await stat(path)).size > 24 * 1024 * 1024)
        throw new CacheError('PREVIEW_BUDGET', 'mesh cache exceeds 24 MiB');
    const raw = await readFile(path);
    const report = JSON.parse(await readFile(join(source, 'report.json'), 'utf8'));
    const expected = report.artifact_sha256?.['preview.mesh.json'];
    if (!expected || createHash('sha256').update(raw).digest('hex') !== expected
        || typeof report.model_hash !== 'string' || !report.preview) {
        throw new CacheError('INVALID_MESH_CACHE', 'cache does not match its source revision');
    }
    return { raw, source, sourceHash: report.model_hash,
        options: { pose: report.preview.pose, chord_mm: report.preview.chord_mm, ...options } };
}
