/** Run in a fresh consumer directory after installing the release tarball. */
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFile, access, mkdtemp, writeFile } from 'node:fs/promises';
import { join, relative, resolve } from 'node:path';

assert.ok(!process.env.AGENT_CAD_BINARY, 'Test the installed binary with no override');
const cwd = process.cwd();
const pkg = join(cwd, 'node_modules/@coloopx/colocad');
const cli = join(cwd, 'node_modules/.bin/colocad');
const skill = join(pkg, 'skills/colocad');
const readJson = async (path) => JSON.parse(await readFile(path, 'utf8'));
const build = (...args) => JSON.parse(execFileSync(process.execPath, args, { encoding: 'utf8' }));
const out = await mkdtemp(join(cwd, 'packed-check-'));
const manifest = await readJson(join(pkg, 'package.json'));
const version = (await readJson(join(pkg, 'kernel/release.json'))).version;
assert.equal(version, manifest.version);
const manifold = build(cli, 'build', join(pkg, 'examples/manifold.json'), join(out, 'manifold'), '--preview');
assert.equal(manifold.geometry.faces, 27);
await access(manifold.step);
await access(join(manifold.directory, 'preview.png'));

const helper = join(skill, 'scripts/build.mjs');
const plate = build(helper, join(skill, 'assets/plate-grid.json'), join(out, 'plate'),
  '--set', 'length=100', '--set', 'xCount=3', '--set', 'xPitch=32');
assert.equal(plate.geometry.solids, 1);
assert.ok(Math.abs(plate.geometry.volume_mm3 - (100 * 50 - 6 * Math.PI * 9) * 6) < 0.01);
const sleeve = await readJson(join(skill, 'assets/sleeve.json'));
const input = sleeve.output;
delete sleeve.output;
sleeve.assembly = [0, 24].map((z, i) => ({ id: 'ring' + i, input, translation: [0, 0, z] }));
const source = join(out, 'pair.json');
await writeFile(source, JSON.stringify(sleeve));
const pair = build(helper, source, join(out, 'pair'));
const report = await readJson(join(pair.directory, 'report.json'));
assert.equal(pair.geometry.solids, 2);
assert.equal(report.validation.measured_instances, 1);
assert.equal(report.validation.translated_instance_reuses, 1);

const locations = { manifold, plate, pair };
if (process.argv[2]) {
  const hand = build(helper, resolve(process.argv[2]), join(out, 'hand'));
  const handReport = await readJson(join(hand.directory, 'report.json'));
  assert.equal(hand.geometry.solids, 43);
  assert.equal(handReport.validation.translated_instance_reuses, 22);
  locations.hand = hand;
}
console.log(JSON.stringify({ version, platform: process.platform, arch: process.arch,
  outputs: Object.fromEntries(Object.entries(locations).map(([k, v]) => [k, relative(cwd, v.directory)])) }));
