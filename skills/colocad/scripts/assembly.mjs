#!/usr/bin/env node
import { mkdir, readFile } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { createRequire } from 'node:module';

try {
  const [input, output, ...flags] = process.argv.slice(2);
  if (!input || !output || flags.length > 1 || (flags.length && flags[0] !== '--preview'))
    throw Error('usage: node assembly.mjs MODEL.json NEW_DIRECTORY [--preview]');
  // The skill lives outside node_modules. Use the consumer project’s pinned SDK.
  const project = createRequire(join(process.cwd(), 'package.json'));
  const name = '@coloopx/colocad';
  let entry;
  for (const root of project.resolve.paths(name)) {
    const pkg = join(root, name);
    try {
      const manifest = JSON.parse(await readFile(join(pkg, 'package.json'), 'utf8'));
      entry = join(pkg, manifest.exports['./kernel'].import);
      break;
    } catch (e) { if (e.code !== 'ENOENT') throw e; }
  }
  if (!entry) throw Error('Install @coloopx/colocad in the current project first.');
  const { CadClient } = await import(pathToFileURL(entry).href);
  const model = JSON.parse(await readFile(input, 'utf8'));
  if (!Array.isArray(model.assembly) || !model.assembly.length) throw Error('Expected a kernel assembly model.');
  if (flags.length) model.preview ??= {};
  const dest = resolve(output);
  await mkdir(dest, { recursive: false });
  const result = await new CadClient({ outputRoot: dest, wallMs: 120000 }).build(model);
  if (!result.ok) throw Error(JSON.stringify(result.error));
  console.log(JSON.stringify({ directory: result.data.artifact_directory, geometry: result.data.geometry }));
} catch (error) {
  console.error(error.message); process.exitCode = 1;
}
