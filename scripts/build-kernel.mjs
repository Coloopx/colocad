import { spawnSync } from "node:child_process";
import { copyFile, mkdir, readFile, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";
import { join } from "node:path";
const root = fileURLToPath(new URL("..", import.meta.url));
for (const [cmd, args] of [
  [process.execPath, ["scripts/vendor.mjs"]],
  ["cargo", ["build", "--release", "--locked", "-p", "agent-cad"]],
]) {
  const result = spawnSync(cmd, args, { cwd: root, stdio: "inherit" });
  if (result.status !== 0) throw Error("Build failed: " + cmd);
}
const platform = process.platform + "-" + process.arch;
const dir = join(root, "kernel/native", platform);
const name = process.platform === "win32" ? "agent-cad.exe" : "agent-cad";
await mkdir(dir, { recursive: true });
await copyFile(join(root, "target/release", name), join(dir, name));
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
