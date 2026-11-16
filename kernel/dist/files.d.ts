import type { CadClient } from './client.js';
import type { Model } from './types.js';
/** 当前宿主可传已验证回执SHA；默认读取原子发布的固定旁文件，不授予source权限。 */
export declare function readStepArtifact(cad: CadClient, artifactDirectory: string, options?: {
    sha256?: string;
}): Promise<Buffer>;
/** 当前宿主报告读取；producer策略无授权作用，SHA由可信调用方提供。 */
export declare function readReportFile(cad: {
    readonly resourceLimits: {
        readonly reportBytes: number;
    };
}, path: string, options?: {
    sha256?: string;
}): Promise<Buffer>;
/** Paths belong to the Harness in the current sandbox. Foreign STEP geometry is imported as an asset. */
export declare function readModelFile(cad: CadClient, path: string): Promise<Model>;
/** Export a new file. A STEP with embedModel=true can be reopened and edited from that single file. */
export declare function writeModelFile(cad: CadClient, model: Model, path: string, options?: {
    embedModel?: boolean;
}): Promise<{
    path: string;
    bytes: number;
    sha256: string;
    model_hash: string;
}>;
export declare function editModelFile(cad: CadClient, source: string, parameters: Record<string, number>, destination: string): Promise<{
    path: string;
    bytes: number;
    sha256: string;
    model_hash: string;
}>;
