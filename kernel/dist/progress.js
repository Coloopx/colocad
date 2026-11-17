/** [INPUT]: 原生stderr中的有序结构化执行事件。
 * [OUTPUT]: 有界进度解析器和公开事件类型；普通日志及重放事件不续期。
 * [POS]: SDK的观测边界，不将CPU心跳或模型文本当作有效进展。
 * [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md */
import { StringDecoder } from 'node:string_decoder';
const phases = new Set(['command', 'feature', 'validation', 'check', 'export']);
export function progressReader(accept) {
    let partial = '', sequence = 0, discard = false;
    const decoder = new StringDecoder('utf8');
    return (chunk) => {
        for (const piece of decoder.write(chunk).split(/(?<=\n)/)) {
            if (!discard)
                partial += piece;
            if (partial.length > 4096) {
                partial = '';
                discard = true;
            }
            if (!piece.endsWith('\n'))
                continue;
            const line = partial;
            partial = '';
            if (discard) {
                discard = false;
                continue;
            }
            if (!line.startsWith('COLOCAD_PROGRESS '))
                continue;
            let row;
            try {
                row = JSON.parse(line.slice(17));
            }
            catch {
                continue;
            }
            if (row?.schema !== 'colocad.progress/1' || !Number.isSafeInteger(row.seq) || row.seq <= sequence
                || !phases.has(row.phase) || !['started', 'completed'].includes(row.state)
                || row.feature_id !== undefined && (typeof row.feature_id !== 'string' || !row.feature_id.length || row.feature_id.length > 128))
                continue;
            sequence = row.seq;
            accept(row);
        }
    };
}
