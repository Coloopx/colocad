// Mount the packed npm archive at /pkg.tgz in the Abelien runner image.
import { createBashTool } from "/usr/local/lib/node_modules/@earendil-works/pi-coding-agent/dist/index.js";
import { mkdir, readFile } from "node:fs/promises";
const cwd = "/work/colocad-check";
await mkdir(cwd);
const shell = createBashTool(cwd);
const steps = [];
for (const [name, command] of [
  [
    "install",
    "npm install --offline --ignore-scripts --no-fund --no-audit /pkg.tgz",
  ],
  [
    "manifold",
    "./node_modules/.bin/colocad build node_modules/colocad/examples/manifold.json original --preview",
  ],
  [
    "rail",
    "./node_modules/.bin/colocad build node_modules/colocad/examples/sensor-rail.json rail",
  ],
  [
    "edit",
    `node --input-type=module -e "import fs from 'node:fs'; import {build} from 'colocad'; const t=JSON.parse(fs.readFileSync('original/part/feature-tree.json'));t.parameters.instances=3;console.log(JSON.stringify(await build(t,'edited')));"`,
  ],
]) {
  const t = performance.now();
  const r = await shell.execute(name, { command, timeout: 100 });
  const msg = r.content
    .filter((v) => v.type === "text")
    .map((v) => v.text)
    .join("");
  const row = { name, elapsedMs: Math.round(performance.now() - t) };
  if (name !== "install") {
    const { geometry } = JSON.parse(msg);
    if (geometry.solids !== 1) throw Error("Expected one solid");
    Object.assign(row, { geometry });
  }
  steps.push(row);
}
const delta = steps[3].geometry.volume_mm3 - steps[1].geometry.volume_mm3;
if (Math.abs(delta - 72 * Math.PI) > 0.001)
  throw Error("Edited feature pattern did not rebuild as expected");
const cg = async (k) => (await readFile("/sys/fs/cgroup/" + k, "utf8")).trim();
console.log(
  JSON.stringify(
    {
      platform: process.platform,
      arch: process.arch,
      node: process.version,
      uid: process.getuid(),
      cpuMax: await cg("cpu.max"),
      memoryLimit: Number(await cg("memory.max")),
      containerPeakBytes: Number(await cg("memory.peak")),
      dispatch: "Pi createBashTool in Abelien runner; deterministic tool calls",
      steps,
      editVolumeDeltaMM3: delta,
    },
    null,
    2,
  ),
);
