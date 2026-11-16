/** [INPUT]: SDK模块。[OUTPUT]: 公开模型、宿主配置、资产登记与校验STEP产物读取API。
 * [POS]: package kernel入口。[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md */
export { CadClient } from './client.js';
export { attachStep } from './assets.js';
export type { StepRegistrationOptions } from './assets.js';
export { ResourceLimitError, DEFAULT_RESOURCE_LIMITS, RESOURCE_LIMIT_CEILINGS } from './resource-limits.js';
export { createCadTools } from './tools.js';
export { resolveBinary, resolveBinary as resolveWorker } from './binary.js';
export { pose, occurrence, hardwareEnvelope, envelope, simplified, blindBoss } from './helpers.js';
export type { HardwareKind } from './helpers.js';
export type * from './types.js';
export {readModelFile,writeModelFile,editModelFile,readStepArtifact} from './files.js';
