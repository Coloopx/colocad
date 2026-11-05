// Run inside the benchmark container after the STEP timings.
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { join } from 'node:path';
import { performance } from 'node:perf_hooks';
import { build, kernelModel, CadClient } from '/opt/package/src/index.mjs';

const rows = [];
for (const name of ['sensor-rail', 'manifold']) {
  const tree = JSON.parse(await readFile('/opt/package/examples/' + name + '.json'));
  const first = await build(tree, '/tmp/' + name, { preview: true });
  for (let rep = 0; rep <= 5; rep++) {
    const hashes = {};
    for (const mode of (rep % 2 ? ['cache', 'rebuild'] : ['rebuild', 'cache'])) {
      // Fresh clients keep a full rebuild independent of revision reuse.
      const client = new CadClient({ outputRoot: '/tmp/out', wallMs: 120000 });
      const model = { ...kernelModel(tree), preview: { view: 'top' } };
      const start = performance.now();
      const result = mode === 'cache'
        ? await client.observeArtifact(first.directory, model.preview)
        : await client.build(model);
      const wall_ms = performance.now() - start;
      if (!result.ok) throw Error(JSON.stringify(result.error));
      hashes[mode] = {};
      for (const file of ['preview.png', 'model.glb', 'model.stl']) {
        const bytes = await readFile(join(result.data.artifact_directory, file));
        hashes[mode][file] = createHash('sha256').update(bytes).digest('hex');
      }
      rows.push({ case: name, rep, mode, wall_ms, hashes: hashes[mode] });
    }
    if (JSON.stringify(hashes.cache) !== JSON.stringify(hashes.rebuild))
      throw Error('Different display artifacts: ' + name);
  }
}
console.log(JSON.stringify(rows, null, 2));
