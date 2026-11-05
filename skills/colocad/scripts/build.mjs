#!/usr/bin/env node
import { mkdir, readFile, rmdir } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { createRequire } from 'node:module';

try {
  const [input, output, ...flags] = process.argv.slice(2);
  if (!input || !output)
    throw Error('usage: node build.mjs MODEL.json NEW_DIRECTORY [--set name=value]... [--preview]');
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
  if (model.version !== 1 || (!model.output && !model.assembly?.length))
    throw Error('Expected a kernel model with output or assembly. For colocad/1 feature trees use colocad build.');
  for (let i = 0; i < flags.length; i++) {
    if (flags[i] === '--preview') {
      model.preview ??= {};
      continue;
    }
    if (flags[i] !== '--set') throw Error('Unknown option: ' + flags[i]);
    const assignment = flags[++i] ?? '';
    const [key, value, extra] = assignment.split('=');
    if (extra !== undefined || !value?.trim() || !Number.isFinite(Number(value)))
      throw Error('Expected --set name=number, got: ' + assignment);
    if (!Object.hasOwn(model.parameters ?? {}, key)) throw Error('Unknown model parameter: ' + key);
    model.parameters[key] = Number(value);
  }
  const dest = resolve(output);
  await mkdir(dest, { recursive: false });
  try {
    const result = await new CadClient({ outputRoot: dest, wallMs: 120000 }).build(model);
    if (!result.ok) throw Error(JSON.stringify(result.error));
    console.log(JSON.stringify({ directory: result.data.artifact_directory, geometry: result.data.geometry }));
  } catch (error) {
    // Remove only the empty directory reserved by this invocation.
    await rmdir(dest).catch(() => {});
    throw error;
  }
} catch (error) {
  console.error(error.message); process.exitCode = 1;
}
