import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { artifactJob } from './artifacts.js';
import { resolveBinary } from './binary.js';
import { execute, receipt, error, type CadInput } from './process.js';
import { previewKey, readPreviewCache, CacheError } from './preview-cache.js';
import type { Config, Model, Receipt, PreviewOptions, Sketch } from './types.js';
export type * from './types.js';
export { attachStep } from './assets.js';

/** Local CAD API for the Harness and Agent sharing this sandbox. */
export class CadClient {
  private previousRevision?: string;
  private readonly previewCache = new Map<string, string>();
  constructor(private readonly config: Config) {
    if (config.runner && config.runner.mode !== 'native') throw Error('CAD executes in the current sandbox. Use the sandbox test harness for Docker experiments.');
    if (config.wallMs !== undefined && (!Number.isFinite(config.wallMs) || config.wallMs < 1 || config.wallMs > 120_000)) {
      throw Error('wallMs must be 1..120000');
    }
  }
  async capabilities(): Promise<Receipt> { return this.invoke(['capabilities']); }
  async schema(): Promise<Receipt> { return this.invoke(['schema']); }
  async validate(model: Model): Promise<Receipt> { return this.invoke(['validate'], model); }
  async solveSketch(sketch: Sketch, parameters: Record<string, number> = {}): Promise<Receipt> {
    return this.invoke(['solve-sketch'], { sketch, parameters });
  }
  async inspectStep(model: Model, asset: string): Promise<Receipt> {
    const source = model.assets?.[asset];
    if (!source) return error('ASSET_NOT_FOUND', asset);
    const result = await this.invoke(['inspect-step'], source);
    if (result.ok) {
      const template = result.data.model_template as Model;
      for (const feature of template.features) if (feature.op === 'import_step') feature.asset = asset;
      result.data.required_asset_id = asset;
    }
    return result;
  }
  async preview(model: Model, options: Partial<PreviewOptions> = {}): Promise<Receipt> {
    const next = { ...structuredClone(model), preview: { ...model.preview, ...options } };
    const cached = this.previewCache.get(previewKey(next));
    return cached ? this.observeArtifact(cached, next.preview) : this.build(next);
  }
  /** Resume display from a trusted host artifact path, including in a new SDK process. */
  async observeArtifact(directory: string, options: Partial<PreviewOptions> = {}): Promise<Receipt> {
    try {
      const cached = await readPreviewCache(directory, options);
      return await artifactJob(this.config, 'observe-', async (_job, destination) => {
        const result = await this.invoke(['observe', '--out', destination, '--options', JSON.stringify(cached.options)], undefined, cached.raw);
        if (!result.ok) return result;
        if (result.data.source_hash !== cached.sourceHash) return error('INVALID_MESH_CACHE', 'cache source revision mismatch');
        return { ok: true, data: { ...result.data, source_artifact_directory: cached.source } };
      });
    } catch (cause) {
      return error(cause instanceof CacheError ? cause.code : 'PREVIEW_CACHE_READ_FAILED', String(cause));
    }
  }
  async build(model: Model, previousRevision = this.previousRevision): Promise<Receipt> {
    const checkpoints = previousRevision ? ['--reuse', previousRevision] : ['--checkpoints'];
    const result = await artifactJob(this.config, 'job-', (_job, destination) => this.invoke(['build', '--out', destination, ...checkpoints], model));
    if (result.ok) this.previousRevision = result.data.artifact_directory as string;
    if (result.ok && model.preview && (result.data.artifact_sha256 as Record<string, string> | undefined)?.['preview.mesh.json']) {
      this.previewCache.set(previewKey(model), result.data.artifact_directory as string);
      if (this.previewCache.size > 16) this.previewCache.delete(this.previewCache.keys().next().value!);
    }
    return result;
  }
  async edit(model: Model, parameters: Record<string, number>): Promise<Receipt> {
    for (const [key, value] of Object.entries(parameters)) {
      if (!Object.hasOwn(model.parameters, key)) return error('UNKNOWN_PARAMETER', key);
      if (!Number.isFinite(value)) return error('INVALID_PARAMETER', key);
    }
    return this.build({ ...structuredClone(model), parameters: { ...model.parameters, ...parameters } });
  }
  async inspect(artifactDirectory: string): Promise<Record<string, unknown>> {
    return JSON.parse(await readFile(join(artifactDirectory, 'report.json'), 'utf8'));
  }
  private async invoke(args: string[], model?: unknown, rawInput?: CadInput): Promise<Receipt> {
    const input = rawInput ?? (model ? JSON.stringify(model) : '');
    if (Buffer.byteLength(input) > 32 * 1024 * 1024) return error('RESOURCE_LIMIT', 'input exceeds 32 MiB');
    try {
      const binary = resolveBinary(this.config.binary ?? this.config.runner?.binary);
      return receipt(await execute(binary, args, input, this.config.wallMs ?? 30_000));
    } catch (cause) { return error('LOCAL_EXECUTION_FAILED', String(cause)); }
  }
}
