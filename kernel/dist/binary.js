import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
/** Configured path, then packaged CAD executable for the current OS and architecture. */
export function resolveBinary(binary) {
    const configured = binary ?? process.env.AGENT_CAD_BINARY;
    if (configured)
        return resolve(configured);
    const name = process.platform === 'win32' ? 'agent-cad.exe' : 'agent-cad';
    const path = fileURLToPath(new URL(`../native/${process.platform}-${process.arch}/${name}`, import.meta.url));
    if (!existsSync(path))
        throw Error(`No packaged CAD executable for ${process.platform}-${process.arch}; configure binary with a local build`);
    return path;
}
