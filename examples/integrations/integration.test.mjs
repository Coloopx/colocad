import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm, cp, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { CadClient } from '@coloopx/colocad';
import { createAssistantMessageEventStream } from '@earendil-works/pi-ai';
import { discoverAndLoadExtensions, loadSkillsFromDir } from '@earendil-works/pi-coding-agent';
import { createPiCadTools } from './pi-tools.mjs';
import { createCadAgent } from './pi-core.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const skill = join(here, 'node_modules/@coloopx/colocad/skills/colocad');
const plate = JSON.parse(await readFile(join(skill, 'assets/plate-grid.json'), 'utf8'));
function data(receipt) {
  assert.equal(receipt.ok, true, JSON.stringify(receipt));
  return receipt.data;
}
async function temp(t) {
  const dir = await mkdtemp(join(tmpdir(), 'colocad-integration-'));
  t.after(() => rm(dir, { recursive: true, force: true }));
  return dir;
}

test('published SDK: build, cached camera change, parameter edit, persisted source', async (t) => {
  const cad = new CadClient({ outputRoot: await temp(t), wallMs: 30_000 });
  data(await cad.validate(plate));
  const original = data(await cad.preview(plate, { view: 'iso' }));
  const top = data(await cad.preview(plate, { view: 'top' }));
  assert.equal(top.source_artifact_directory, original.artifact_directory);
  assert.ok(top.artifact_sha256['preview.png']);
  const edited = data(await cad.edit(plate, { holeDiameter: 8 }));
  const expectedRemoval = 4 * Math.PI * (4 ** 2 - 3 ** 2) * 6;
  assert.ok(Math.abs(original.geometry.volume_mm3 - edited.geometry.volume_mm3 - expectedRemoval) < 0.01);
  assert.equal(plate.parameters.holeDiameter, 6);
  const saved = JSON.parse(await readFile(join(edited.artifact_directory, 'model.json'), 'utf8'));
  assert.equal(saved.parameters.holeDiameter, 8);
  assert.match(await readFile(join(edited.artifact_directory, 'model.step'), 'utf8'), /ISO-10303-21/);
});

test('Pi adapter: complete schema, native preview image, failed receipt, pre-dispatch cancellation', async (t) => {
  const tools = await createPiCadTools({ outputRoot: await temp(t) });
  const preview = tools.find((tool) => tool.name === 'cad_preview');
  assert.ok(preview.parameters.$defs.PreviewOptions);
  const result = await preview.execute('preview', { model: plate, options: { view: 'front' } });
  const image = result.content.find((item) => item.type === 'image');
  assert.equal(image.mimeType, 'image/png');
  assert.equal(Buffer.from(image.data, 'base64').subarray(1, 4).toString(), 'PNG');
  const edit = tools.find((tool) => tool.name === 'cad_edit');
  await assert.rejects(edit.execute('bad', { model: plate, parameters: { missing: 2 } }), /UNKNOWN_PARAMETER/);
  await assert.rejects(preview.execute('cancel', { model: plate }, AbortSignal.abort()), /abort/i);
});

test('Pi adapter exposes explicit degradation to the model', async (t) => {
  const tools = await createPiCadTools({ outputRoot: await temp(t) });
  const build = tools.find((tool) => tool.name === 'cad_build');
  const model = { version: 1, units: 'mm', parameters: {}, features: [
    { id: 'stock', op: 'box', origin: [0, 0, 0], size: [10, 10, 10] },
    { id: 'round', op: 'fillet', input: 'stock', radius: 50, select: { kind: 'all' },
      on_fail: { kind: 'record', reason: 'Explicit optional treatment', omitted: ['50 mm fillet'], affects: ['appearance'] } },
  ], output: 'round' };
  const result = await build.execute('degraded', { model });
  const summary = JSON.parse(result.content[0].text);
  assert.equal(summary.degradations[0].id, 'round');
  assert.deepEqual(summary.degradations[0].omitted, ['50 mm fillet']);
  assert.ok(summary.degradations[0].error);
});

test('real Pi core loop: failed CAD call is delivered to model; correction creates STEP', async (t) => {
  const outputRoot = await temp(t);
  const model = { id: 'scripted', name: 'Scripted test', api: 'openai-completions', provider: 'test',
    baseUrl: 'https://unused.invalid', reasoning: false, input: ['text', 'image'],
    cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 }, contextWindow: 100000, maxTokens: 1000 };
  let turns = 0;
  const streamFn = (_model, context) => {
    const previous = context.messages.filter((m) => m.role === 'toolResult');
    let content;
    if (turns === 0) content = [{ type: 'toolCall', id: 'bad', name: 'cad_edit', arguments: { model: plate, parameters: { missing: 4 } } }];
    else if (turns === 1) {
      assert.equal(previous.at(-1).isError, true);
      assert.match(JSON.stringify(previous.at(-1).content), /UNKNOWN_PARAMETER/);
      content = [{ type: 'toolCall', id: 'fixed', name: 'cad_build', arguments: { model: plate } }];
    } else {
      assert.equal(previous.at(-1).isError, false);
      content = [{ type: 'text', text: 'STEP ready.' }];
    }
    const stopReason = turns++ < 2 ? 'toolUse' : 'stop';
    const message = { role: 'assistant', content, api: model.api, provider: model.provider, model: model.id,
      usage: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, totalTokens: 0, cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 } },
      stopReason, timestamp: Date.now() };
    const stream = createAssistantMessageEventStream();
    stream.push({ type: 'done', reason: stopReason, message });
    return stream;
  };
  const agent = await createCadAgent({ model, streamFn, outputRoot });
  const results = [];
  agent.subscribe((event) => { if (event.type === 'tool_execution_end') results.push(event); });
  await agent.prompt('Make the plate.');
  assert.equal(turns, 3);
  assert.equal(results.length, 2);
  assert.equal(results[0].isError, true);
  assert.equal(results[1].isError, false);
  const built = results[1].result.details;
  assert.equal(built.geometry.solids, 1);
  assert.match(await readFile(join(built.artifact_directory, 'model.step'), 'utf8'), /ISO-10303-21/);
});

test('Pi coding agent discovers full skill and loads the extension with five executable tools', async (t) => {
  const dir = await temp(t);
  const skillRoot = join(dir, '.agents/skills');
  await mkdir(skillRoot, { recursive: true });
  await cp(skill, join(skillRoot, 'colocad'), { recursive: true });
  const discovered = loadSkillsFromDir({ dir: skillRoot, source: 'test' });
  assert.deepEqual(discovered.diagnostics, []);
  assert.equal(discovered.skills[0].name, 'colocad');
  await readFile(join(discovered.skills[0].baseDir, 'scripts/build.mjs'));
  const loaded = await discoverAndLoadExtensions([join(here, 'pi-extension.mjs')], dir, join(dir, 'empty-agent-dir'));
  assert.deepEqual(loaded.errors, []);
  const extension = loaded.extensions.find((item) => item.resolvedPath === join(here, 'pi-extension.mjs'));
  assert.equal(extension.tools.size, 5);
  const result = await extension.tools.get('cad_capabilities').definition.execute('caps', {});
  assert.ok(result.content[0].text.length > 10);
});
