/** [INPUT]: package公开SDK类型。[OUTPUT]: 宿主字节与注册选项的编译契约。
 * [POS]: 不允许模型/未知配置字段替代预算。[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md */
import {CadClient,attachStep,ResourceLimitError} from '@coloopx/colocad/kernel';
import type {Config,ResourceLimits,StepRegistrationOptions} from '@coloopx/colocad/kernel';
const limits:ResourceLimits={inputBytes:67108864,stepBytes:41943040,totalStepBytes:50331648};
const config:Config={outputRoot:'./out',resourceLimits:limits};
const cad=new CadClient(config);
const bytes:number=cad.resourceLimits.stepBytes;
const options:StepRegistrationOptions={maxBytes:bytes};
const model=attachStep({version:1,units:'mm',parameters:{},features:[]},'source','STEP',options);
const reason=new ResourceLimitError('stepBytes',33,32);
// @ts-expect-error Memory quota is a sandbox capability, not a STEP byte setting.
const invalid:ResourceLimits={memoryGiB:3};
// @ts-expect-error The active host view is immutable.
cad.resourceLimits.stepBytes=12;
void [model,reason,invalid];

const lifecycle:Config={outputRoot:'./out',idleMs:600000,signal:new AbortController().signal,onProgress:event=>{const n:number=event.seq;void n;}};
// @ts-expect-error Progress callbacks observe a closed native phase vocabulary.
const badLifecycle:Config={outputRoot:'./out',onProgress:event=>{const invalid:'heartbeat'=event.phase;void invalid;}};
void [lifecycle,badLifecycle];
