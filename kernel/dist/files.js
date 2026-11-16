/** [INPUT]: 宿主文件路径、产物SHA及CadClient不可变输入/输出预算。
 * [OUTPUT]: 模型/当前reportBytes报告读取与固定SHA的独立STEP读取。
 * [POS]: 路径留在宿主；输出不继承producer授权，外来原文仍按输入预算登记。
 * [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md */
import { open, readFile, writeFile, link, unlink, realpath } from 'node:fs/promises';
import { randomUUID, createHash } from 'node:crypto';
import { dirname, basename, extname, join } from 'node:path';
import { attachStep } from './assets.js';
import { ResourceLimitError, checkedLimit, RESOURCE_LIMIT_CEILINGS } from './resource-limits.js';
const otherArtifactLimit = 32 * 1024 * 1024;
const marker = '/* AGENT_CAD_IR_1 ';
const hash = (s) => createHash('sha256').update(s).digest('hex');
function requireData(r) { if (!r.ok)
    throw Object.assign(Error(r.error.message), r.error); return r.data; }
const unsupported = (path) => Object.assign(Error('No local CAD file codec for ' + extname(path)), { code: 'UNSUPPORTED_FILE_FORMAT' });
async function bounded(path, limit, resource = 'inputBytes') {
    const fd = await open(path, 'r');
    try {
        const s = await fd.stat();
        if (!s.isFile())
            throw Error('CAD file must be a regular file');
        if (s.size > limit)
            throw new ResourceLimitError(resource, s.size, limit);
        // ---------- 同一描述符有界读取，文件增长最多观察limit+1字节 ----------
        const chunks = [];
        let total = 0;
        for (;;) {
            const chunk = Buffer.allocUnsafe(Math.min(64 * 1024, limit + 1 - total));
            const { bytesRead } = await fd.read(chunk, 0, chunk.length, null);
            if (!bytesRead)
                break;
            total += bytesRead;
            if (total > limit)
                throw new ResourceLimitError(resource, total, limit, true);
            chunks.push(chunk.subarray(0, bytesRead));
        }
        return Buffer.concat(chunks, total);
    }
    finally {
        await fd.close();
    }
}
async function artifactPath(root, name) {
    const path = await realpath(join(root, name));
    if (dirname(path) !== root)
        throw Object.assign(Error('STEP artifact path leaves its directory'), { code: 'STEP_ARTIFACT_INTEGRITY' });
    return path;
}
/** SHA-256固定64位小写十六进制加换行，不随装配规模增长。 */
const stepDigestBytes = 64 + 1;
/** 当前宿主可传已验证回执SHA；默认读取原子发布的固定旁文件，不授予source权限。 */
export async function readStepArtifact(cad, artifactDirectory, options = {}) {
    const limit = cad.resourceLimits.outputStepBytes, receiptDigest = options.sha256;
    const root = await realpath(artifactDirectory);
    const bytes = await bounded(await artifactPath(root, 'model.step'), limit, 'outputStepBytes');
    const expected = receiptDigest === undefined ?
        (await bounded(await artifactPath(root, 'model.step.sha256'), stepDigestBytes, 'outputBytes')).toString('utf8') : receiptDigest + '\n';
    if (!/^[a-f0-9]{64}\n$/.test(expected) || hash(bytes) !== expected.slice(0, -1))
        throw Object.assign(Error('STEP artifact SHA256 digest mismatch'), { code: 'STEP_ARTIFACT_INTEGRITY' });
    return bytes;
}
/** 当前宿主报告读取；producer策略无授权作用，SHA由可信调用方提供。 */
export async function readReportFile(cad, path, options = {}) {
    const limit = checkedLimit(cad.resourceLimits.reportBytes, RESOURCE_LIMIT_CEILINGS.reportBytes, 'reportBytes'), expected = options.sha256;
    const bytes = await bounded(path, limit, 'reportBytes');
    if (expected !== undefined && (!/^[a-f0-9]{64}$/.test(expected) || hash(bytes) !== expected))
        throw Object.assign(Error('SESSION_INTEGRITY: report SHA256 mismatch'), { code: 'REPORT_INTEGRITY' });
    return bytes;
}
/** Paths belong to the Harness in the current sandbox. Foreign STEP geometry is imported as an asset. */
export async function readModelFile(cad, path) {
    const ext = extname(path).toLowerCase();
    if (!['.json', '.step', '.stp'].includes(ext))
        throw unsupported(path);
    const raw = (await bounded(path, cad.resourceLimits.inputBytes)).toString('utf8');
    let model;
    if (ext === '.json') {
        model = JSON.parse(raw);
    }
    else {
        if (!/^\s*ISO-10303-21;/i.test(raw))
            throw Error('invalid STEP exchange header');
        const start = raw.indexOf(marker);
        if (start < 0) {
            const id = 'source_part';
            const seed = attachStep({ version: 1, units: 'mm', parameters: {}, features: [], output: '' }, id, raw, { maxBytes: cad.resourceLimits.stepBytes });
            const inspected = requireData(await cad.inspectStep(seed, id));
            model = { ...inspected.model_template, assets: seed.assets };
        }
        else {
            const end = raw.indexOf(' */\n', start);
            if (end < 0 || raw.indexOf(marker, start + marker.length) >= 0)
                throw Error('invalid embedded model record');
            const fields = raw.slice(start + marker.length, end).split(' ');
            if (fields.length !== 3)
                throw Error('invalid embedded model fields');
            const [stepHash, sourceHash, encoded] = fields;
            const step = raw.slice(0, start) + raw.slice(end + 4);
            const source = Buffer.from(encoded, 'base64');
            if (hash(step) !== stepHash || hash(source) !== sourceHash || source.toString('base64') !== encoded)
                throw Error('embedded model or STEP digest mismatch');
            model = JSON.parse(source.toString('utf8'));
        }
    }
    requireData(await cad.validate(model));
    return model;
}
async function publish(path, bytes) {
    const temp = join(dirname(path), '.' + basename(path) + '.' + randomUUID() + '.tmp');
    try {
        await writeFile(temp, bytes, { flag: 'wx', mode: 0o600 });
        await link(temp, path);
    }
    finally {
        await unlink(temp).catch(e => { if (e.code !== 'ENOENT')
            throw e; });
    }
}
/** Export a new file. A STEP with embedModel=true can be reopened and edited from that single file. */
export async function writeModelFile(cad, model, path, options = {}) {
    const ext = extname(path).toLowerCase();
    const names = { '.json': 'model.json', '.step': 'model.step', '.stp': 'model.step', '.glb': 'model.glb', '.stl': 'model.stl', '.png': 'preview.png' };
    if (!Object.hasOwn(names, ext))
        throw unsupported(path);
    const preview = ['.png', '.stl', '.glb'].includes(ext);
    const stepOutput = ['.step', '.stp'].includes(ext);
    const receipt = requireData(await cad.build(preview ? { ...model, preview: model.preview ?? {} } : model));
    let bytes = stepOutput ? await readStepArtifact(cad, String(receipt.artifact_directory), { sha256: receipt.artifact_sha256?.['model.step'] }) :
        await readFile(join(String(receipt.artifact_directory), names[ext]));
    if (options.embedModel && stepOutput) {
        const step = bytes.toString('utf8'), source = Buffer.from(JSON.stringify(model), 'utf8');
        const first = step.indexOf('\n') + 1;
        if (!first)
            throw Error('invalid STEP output header');
        const comment = marker + hash(bytes) + ' ' + hash(source) + ' ' + source.toString('base64') + ' */\n';
        bytes = Buffer.from(step.slice(0, first) + comment + step.slice(first));
    }
    if (stepOutput && bytes.length > cad.resourceLimits.outputStepBytes)
        throw new ResourceLimitError('outputStepBytes', bytes.length, cad.resourceLimits.outputStepBytes);
    const container = ext === '.json' || options.embedModel && stepOutput;
    const fileLimit = container ? cad.resourceLimits.inputBytes : stepOutput ? cad.resourceLimits.outputStepBytes : otherArtifactLimit;
    if (bytes.length > fileLimit)
        throw new ResourceLimitError(container ? 'inputBytes' : 'outputBytes', bytes.length, fileLimit);
    await publish(path, bytes);
    return { path, bytes: bytes.length, sha256: hash(bytes), model_hash: String(receipt.model_hash) };
}
export async function editModelFile(cad, source, parameters, destination) {
    const model = await readModelFile(cad, source);
    for (const [key, value] of Object.entries(parameters))
        if (!Object.hasOwn(model.parameters, key) || !Number.isFinite(value))
            throw Error('unknown or nonfinite parameter: ' + key);
    return writeModelFile(cad, { ...model, parameters: { ...model.parameters, ...parameters } }, destination, { embedModel: true });
}
