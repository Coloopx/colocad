/**
 * [INPUT]: 公开SDK、ResourceLimits及session读取类型。
 * [OUTPUT]: 输出预算与受控产物reader正反类型合同，仅Windows编译。
 * [POS]: PR6输出预算回归；独立于输入登记选项。
 * [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
 */
import {CadClient,readStepArtifact,attachStep} from '../../kernel/dist/index.js';
import type {ResourceLimits,Model} from '../../kernel/dist/index.js';
import {ModelingSession,readSessionBundle,exportSessionHtml} from '../../src/session.mjs';
const resourceLimits:ResourceLimits={outputStepBytes:67108864};
const cad=new CadClient({outputRoot:'output',resourceLimits});
const output:number=cad.resourceLimits.outputStepBytes;
const bytes:Promise<Buffer>=readStepArtifact(cad,'output/revision');
const legacyBytes:Promise<Buffer>=readStepArtifact(cad,'output/revision',{sha256:'a'.repeat(64)});
// @ts-expect-error STEP output permission cannot authorize arbitrary JSON metadata.
void readStepArtifact(cad,'output/revision',{metadataBytes:134217728});
void ModelingSession.create('session',{resourceLimits});
void readSessionBundle('session',{resourceLimits});
void exportSessionHtml('session','history.html',{resourceLimits});
// @ts-expect-error Active host permissions are immutable.
cad.resourceLimits.outputStepBytes=1;
// @ts-expect-error Output authority cannot be supplied by model IR.
const model:Model={version:1,units:'mm',parameters:{},features:[],outputStepBytes:67108864};
// @ts-expect-error STEP registration still accepts only its own input limit.
void attachStep(model,'source','STEP',{outputStepBytes:67108864});
// @ts-expect-error Numeric byte policy only.
const invalid:ResourceLimits={outputStepBytes:'67108864'};
void [output,bytes,legacyBytes,invalid];
