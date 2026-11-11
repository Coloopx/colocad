import { createHash } from 'node:crypto';
import { readFile, stat } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const readJson = async (name) => JSON.parse(await readFile(join(root, name), 'utf8'));
const pkg = await readJson('package.json');
const release = await readJson('kernel/release.json');
const catalog = await readJson('kernel/catalog.json');
if (release.version !== pkg.version) throw Error('Update the tested kernel hashes for ' + pkg.version);

for (const platform of ['darwin-arm64', 'linux-arm64', 'linux-x64']) {
  const dir = join('kernel', 'native', platform);
  const manifest = await readJson(join(dir, 'manifest.json'));
  const helper = join(root,dir,'colocad-catalog');
  const helperHash = createHash('sha256').update(await readFile(helper)).digest('hex');
  if (helperHash !== catalog.sha256[platform] || !((await stat(helper)).mode & 0o111))
    throw Error('Missing or incorrect session catalog for ' + platform);
  const binary = join(root, dir, 'agent-cad');
  const bytes = await readFile(binary);
  const hash = createHash('sha256').update(bytes).digest('hex');
  if (manifest.platform !== platform || manifest.bytes !== bytes.length ||
      manifest.sha256 !== hash || release.sha256[platform] !== hash ||
      !((await stat(binary)).mode & 0o111)) {
    throw Error('Missing or incorrect tested kernel for ' + platform);
  }
}
await readFile(join(root, 'skills/colocad/SKILL.md'));
