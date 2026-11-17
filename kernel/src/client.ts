/**
 * [INPUT]: 公开模型、宿主配置及原生进程边界。
 * [OUTPUT]: 不可变STEP/reportBytes下的全命令/inspect/preview回执。
 * [POS]: SDK宿主边界；保持模型、路径及资源权限分离。
 * [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
 */
import { realpath } from 'node:fs/promises';
import {readReportFile} from './files.js';
import { join, relative, isAbsolute, sep } from 'node:path';
import { artifactJob } from './artifacts.js';
import { resolveBinary } from './binary.js';
import { execute, receipt, error, type CadInput } from './process.js';
import { previewKey, readPreviewCache, CacheError } from './preview-cache.js';
import type { Config, Model, Receipt, PreviewOptions, Sketch, BuildOptions, QueryRequest } from './types.js';
import { resourceLimits, ResourceLimitError, type ByteLimits } from './resource-limits.js';
export type * from './types.js';
export { attachStep } from './assets.js';

/** Local CAD API for the Harness and Agent sharing this sandbox. */
export class CadClient {
  private readonly byteLimits: Readonly<ByteLimits>;
  private readonly explicitByteLimits: boolean;
  private previousRevision?: string;
  private previousQueryable = false;
  private readonly previewCache = new Map<string, string>();
  constructor(private readonly config: Config) {
    const hostByteLimits = config.resourceLimits;
    this.byteLimits = resourceLimits(hostByteLimits);
    this.explicitByteLimits = hostByteLimits !== undefined;
    if (config.runner && config.runner.mode !== 'native') throw Error('CAD executes in the current sandbox. Use the sandbox test harness for Docker experiments.');
    for(const name of ['wallMs','idleMs'] as const){
      const value=config[name];if(value!==undefined&&(!Number.isSafeInteger(value)||value<1||value>2_147_483_647))throw Error(name+' must be a positive timer integer');
    }
  }
  /** Immutable host policy shared by trusted file codecs and native requests. */
  get resourceLimits(): Readonly<ByteLimits> { return this.byteLimits; }
  async capabilities(): Promise<Receipt> { return this.invoke(['capabilities']); }
  async schema(): Promise<Receipt> { return this.invoke(['schema']); }
  async querySchema(): Promise<Receipt> { return this.invoke(['query-schema']); }
  async query(directory: string, request: QueryRequest): Promise<Receipt> {
    try {
      const actual=await realpath(directory);let allowed=false;
      for(const root of this.config.queryRoots??[this.config.outputRoot]) {
        const base=await realpath(root),path=relative(base,actual);
        if(path===''||(!isAbsolute(path)&&path!=='..'&&!path.startsWith('..'+sep)))allowed=true;
      }
      if(!allowed)return error('QUERY_ARTIFACT_OUTSIDE_ROOT','Artifact is outside the host-configured query roots');
      if(Buffer.byteLength(JSON.stringify(request))>2*1024*1024)return error('QUERY_LIMIT','Query request exceeds 2 MiB');
      return this.invoke(['query','--snapshot',actual],request);
    }catch(cause){return error('QUERY_ARTIFACT_READ_FAILED',String(cause));}
  }
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
      const cached = await readPreviewCache(directory, options, this.byteLimits.reportBytes);
      return await artifactJob(this.config, 'observe-', async (_job, destination) => {
        const result = await this.invoke(['observe', '--out', destination, '--options', JSON.stringify(cached.options)], undefined, cached.raw);
        if (!result.ok) return result;
        if (result.data.source_hash !== cached.sourceHash) return error('INVALID_MESH_CACHE', 'cache source revision mismatch');
        return { ok: true, data: { ...result.data, source_artifact_directory: cached.source } };
      });
    } catch (cause) {
      if(cause instanceof ResourceLimitError)return error(cause.code,cause.message,cause.details);
      return error(cause instanceof CacheError ? cause.code : 'PREVIEW_CACHE_READ_FAILED', String(cause));
    }
  }
  async build(model: Model, optionsOrRevision?: BuildOptions | string): Promise<Receipt> {
    const options=typeof optionsOrRevision==='string'?{previousRevision:optionsOrRevision}:optionsOrRevision??{};
    if(options.queryable===false&&(model.datums?.length||model.interfaces?.length))return error('QUERY_SNAPSHOT_REQUIRED','Datum/interface declarations require queryable artifacts');
    const queryable=options.queryable??!!(model.datums?.length||model.interfaces?.length);
    const previousRevision=options.previousRevision??this.previousRevision;
    const checkpoints = previousRevision ? ['--reuse', previousRevision] : ['--checkpoints'];
    const result = await artifactJob(this.config, 'job-', (_job, destination) => this.invoke(['build', '--out', destination, ...checkpoints,...(queryable?['--queryable']:[])], model));
    if (result.ok) {this.previousRevision = result.data.artifact_directory as string;this.previousQueryable=queryable;}
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
    return this.build({ ...structuredClone(model), parameters: { ...model.parameters, ...parameters } },{queryable:this.previousQueryable});
  }
  async inspect(artifactDirectory: string): Promise<Record<string, unknown>> {
    return JSON.parse((await readReportFile(this,join(artifactDirectory, 'report.json'))).toString('utf8'));
  }
  private async invoke(args: string[], model?: unknown, rawInput?: CadInput): Promise<Receipt> {
    const input = rawInput ?? (model ? JSON.stringify(model) : '');
    const inputBytes = Buffer.byteLength(input);
    if (inputBytes > this.byteLimits.inputBytes) {
      const cause = new ResourceLimitError('inputBytes', inputBytes, this.byteLimits.inputBytes);
      return error(cause.code, cause.message, cause.details);
    }
    try {
      const binary = resolveBinary(this.config.binary ?? this.config.runner?.binary);
      const nativeArgs = this.explicitByteLimits ? [...args, '--resource-limits', JSON.stringify(this.byteLimits)] : [...args];
      if(this.config.idleMs!==undefined||this.config.onProgress)nativeArgs.push('--progress');
      return receipt(await execute(binary, nativeArgs, input, this.config.wallMs??(this.config.idleMs===undefined?30_000:null),this.byteLimits.reportBytes,
        {idleMs:this.config.idleMs,signal:this.config.signal,onProgress:this.config.onProgress}));
    } catch (cause) { return error('LOCAL_EXECUTION_FAILED', String(cause)); }
  }
}
