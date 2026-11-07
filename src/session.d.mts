import type { Config, Model, Receipt, PreviewOptions, Sketch } from '../kernel/dist/types.js';
export interface StepMetadata { label?: string; kind?: string; }
export interface SessionStep {
  id: string; label: string; kind: string; operation: 'build'|'edit'|'observe';
  status: 'running'|'succeeded'|'failed'|'interrupted'; startedAt: string;
  finishedAt?: string; durationMs?: number; inputFile: string; inputSha256: string;
  receiptFile?: string; receiptSha256?: string; artifactDirectory?: string;
  geometry?: Record<string, unknown>; error?: { code: string; message: string };
  nativeTimingMs?: Record<string, unknown>; degradations?: Record<string, unknown>[];
  requestedParameters?: Record<string, number>;
}
export interface SessionLedger { format: 'colocad/session/1'; id: string; title: string; createdAt: string; steps: SessionStep[]; }
export type SessionConfig = Omit<Config, 'outputRoot'>;
export type RecordedReceipt = Receipt & { session_step: string };
export class ModelingSession {
  readonly root: string;
  static create(directory: string, options?: SessionConfig & {title?: string}): Promise<ModelingSession>;
  static open(directory: string, options?: SessionConfig): Promise<ModelingSession>;
  read(): Promise<SessionLedger>;
  source(): Promise<Model>;
  build(model: Model, metadata?: StepMetadata): Promise<RecordedReceipt>;
  edit(parameters: Record<string, number>, metadata?: StepMetadata): Promise<RecordedReceipt>;
  preview(model: Model, options?: Partial<PreviewOptions>, metadata?: StepMetadata): Promise<RecordedReceipt>;
  observe(options?: Partial<PreviewOptions>, metadata?: StepMetadata): Promise<RecordedReceipt>;
  capabilities(): Promise<Receipt>; schema(): Promise<Receipt>; validate(model: Model): Promise<Receipt>;
  solveSketch(sketch: Sketch, parameters?: Record<string,number>): Promise<Receipt>;
  inspectStep(model: Model, asset: string): Promise<Receipt>;
}
export function createSessionTools(session: ModelingSession): Promise<Array<{
  name: string; description: string; inputSchema: Record<string,unknown>;
  execute(args: any): Promise<Receipt>;
}>>;
export function readSessionBundle(directory: string, options?: {cutaway?: boolean; view?: 'iso'|'front'|'side'|'top'}): Promise<Record<string, any>>;
export function exportSessionHtml(directory: string, destination: string, options?: {cutaway?: boolean; view?: 'iso'|'front'|'side'|'top'}): Promise<{file: string; steps: number; meshDefinitions: number; bytes: number}>;
