import { mkdir, mkdtemp, rm } from 'node:fs/promises';
import { resolve, join } from 'node:path';
/** Publish a new revision on success; remove failed jobs even if validation throws. */
export async function artifactJob(config, prefix, action) {
    await mkdir(config.outputRoot, { recursive: true });
    const job = await mkdtemp(join(resolve(config.outputRoot), prefix));
    let keep = false;
    try {
        const result = await action(job, join(job, 'revision'));
        if (!result.ok)
            return result;
        keep = true;
        return { ok: true, data: { ...result.data, artifact_directory: join(job, 'revision') } };
    }
    finally {
        if (!keep)
            await rm(job, { recursive: true, force: true });
    }
}
