import test from "node:test";
import assert from "node:assert/strict";
import { readFile, mkdtemp, rm, access } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { compileTree, build, writeSolidWorks } from "../src/index.mjs";
const sample = async (n) =>
  JSON.parse(
    await readFile(new URL("../examples/" + n + ".json", import.meta.url)),
  );
test("one feature compiler accepts two distinct histories and preserves the nonadjacent cut seed", async () => {
  const manifold = compileTree(await sample("manifold")),
    rail = compileTree(await sample("sensor-rail"));
  assert.deepEqual(
    manifold.map((f) => f.op),
    ["extrude", "cut", "fillet", "linearPattern", "cut"],
  );
  assert.equal(manifold[3].seed, manifold[1].id);
  assert.equal(rail[3].axis, "y");
  assert.notDeepEqual(
    rail.map((f) => f.op),
    manifold.map((f) => f.op),
  );
});
test("invalid operations, stale references and resource limits fail before geometry", async () => {
  const base = await sample("manifold");
  for (const change of [
    (s) => (s.features[3].seed = "Rim"),
    (s) => (s.features[3].seed = "Channel"),
    (s) => (s.features[3].count = 1),
    (s) => (s.features[1].id = "mount"),
    (s) => (s.parameters.pitch = Infinity),
    (s) => (s.features[2].radius = { param: "missing" }),
    (s) => (s.features[1].op = "script"),
    (s) => (s.features[3].count = 129),
  ]) {
    const tree = structuredClone(base);
    change(tree);
    assert.throws(() => compileTree(tree));
  }
});
test("manifold edits rebuild material; two independent builds never overwrite the same destination", async () => {
  const dir = await mkdtemp(join(tmpdir(), "colocad-"));
  try {
    const tree = await sample("manifold");
    const old = await build(tree, join(dir, "base"), { preview: true });
    assert.equal(old.geometry.solids, 1);
    assert.equal(old.geometry.faces, 27);
    const plan = JSON.parse(await readFile(old.nativeJob));
    assert.equal(plan.features[2].edges.count, 6);
    assert.equal(plan.features[3].count, 4);
    await access(join(old.directory, "preview.png"));
    await access(old.step);
    const rev = structuredClone(tree);
    rev.parameters.instances = 3;
    const edited = await build(rev, join(dir, "edit"));
    const delta = edited.geometry.volume_mm3 - old.geometry.volume_mm3;
    assert.ok(Math.abs(delta - Math.PI * 3 ** 2 * 8) < 0.001, delta);
    const conflicting = await Promise.allSettled([
      build(tree, join(dir, "same")),
      build(rev, join(dir, "same")),
    ]);
    assert.equal(conflicting.filter((r) => r.status === "fulfilled").length, 1);
    const loser = conflicting.find((r) => r.status === "rejected");
    assert.equal(loser.reason.code, "EEXIST");
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
});
test("Y-direction rectangular slots rebuild through the same compiler", async () => {
  const dir = await mkdtemp(join(tmpdir(), "colocad-"));
  try {
    const tree = await sample("sensor-rail");
    const a = await build(tree, join(dir, "rail"));
    tree.parameters.copies = 2;
    const b = await build(tree, join(dir, "changed"));
    assert.ok(
      Math.abs(b.geometry.volume_mm3 - a.geometry.volume_mm3 - 9 * 5 * 9) <
        0.001,
    );
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
});
test(
  "headless host returns an explicit native error and never emits a mislabeled file",
  { skip: process.platform === "win32" },
  async () => {
    const dir = await mkdtemp(join(tmpdir(), "colocad-"));
    try {
      const dest = join(dir, "result.SLDPRT");
      await assert.rejects(writeSolidWorks(await sample("manifold"), dest), {
        code: "NATIVE_AUTHOR_UNAVAILABLE",
      });
      await assert.rejects(access(dest), { code: "ENOENT" });
    } finally {
      await rm(dir, { recursive: true, force: true });
    }
  },
);
