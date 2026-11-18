import { spawnSync } from "node:child_process";
import { copyFile, mkdir, readFile, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";
import { join } from "node:path";
const root = fileURLToPath(new URL("..", import.meta.url));
for (const [cmd, args] of [
  [process.execPath, ["scripts/vendor.mjs"]],
  ["cargo", ["build", "--release", "--locked", "-p", "colocad-catalog"]],
  ["cargo", ["build", "--profile", "distribution", "--locked", "-p", "agent-cad"]],
]) {
  const result = spawnSync(cmd, args, { cwd: root, stdio: "inherit" });
  if (result.status !== 0) throw Error("Build failed: " + cmd);
}
const platform = process.platform + "-" + process.arch;
const dir = join(root, "kernel/native", platform);
const name = process.platform === "win32" ? "agent-cad.exe" : "agent-cad";
await mkdir(dir, { recursive: true });
await copyFile(join(root, "target/distribution", name), join(dir, name));
if (process.platform === "darwin") {
  const executable = join(dir, name);
  for (const [cmd, args] of [
    ["xcrun", ["strip", executable]],
    ["codesign", ["--force", "--sign", "-", "--identifier", name, executable]],
    ["codesign", ["--verify", "--strict", executable]],
  ]) {
    if (spawnSync(cmd, args, {stdio: "inherit"}).status !== 0)
      throw Error("Native packaging failed: " + cmd);
  }
}
await copyFile(join(root, "target/release/colocad-catalog"), join(dir, "colocad-catalog"));
const bytes = await readFile(join(dir, name));
await writeFile(
  join(dir, "manifest.json"),
  JSON.stringify(
    {
      platform,
      bytes: bytes.length,
      sha256: createHash("sha256").update(bytes).digest("hex"),
    },
    null,
    2,
  ) + "\n",
);
