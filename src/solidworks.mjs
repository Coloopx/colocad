import { mkdtemp, writeFile, link, rm, mkdir, stat } from "node:fs/promises";
import { dirname, join, resolve, extname } from "node:path";
import { fileURLToPath } from "node:url";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { compileTree } from "./tree.mjs";
import { build } from "./build.mjs";
const run = promisify(execFile);

// A failed or unavailable author never publishes an output part.
export async function writeSolidWorks(input, destination) {
  if (process.platform !== "win32") {
    const error = new Error(
      "NATIVE_AUTHOR_UNAVAILABLE: direct SLDPRT feature serialization on Linux/macOS is experimental; this release only activates the native author on a Windows host with local SOLIDWORKS.",
    );
    error.code = "NATIVE_AUTHOR_UNAVAILABLE";
    throw error;
  }
  const out = resolve(destination);
  if (extname(out).toLowerCase() !== ".sldprt")
    throw Error("Expected a new .SLDPRT file");
  await mkdir(dirname(out), { recursive: true });
  const stage = await mkdtemp(join(dirname(out), ".colocad-native-"));
  try {
    let jobPath;
    if (input?.format === "colocad/1") {
      jobPath = (await build(input, join(stage, "geometry"))).nativeJob;
    } else {
      if (
        input?.format !== "colocad/native-job/1" ||
        !Number.isFinite(input.expected?.volumeMM3) ||
        input.expected.volumeMM3 <= 0
      )
        throw Error("Invalid native job");
      const features = compileTree({
        format: "colocad/1",
        units: "mm",
        parameters: {},
        features: input.features,
      });
      jobPath = join(stage, "job.json");
      await writeFile(jobPath, JSON.stringify({ ...input, features }));
    }
    const temp = join(stage, "part.SLDPRT");
    const assembly = fileURLToPath(
      new URL("../solidworks/managed/Colocad.SolidWorks.dll", import.meta.url),
    );
    const { stdout } = await run("dotnet", [assembly, jobPath, temp], {
      timeout: 120000,
      maxBuffer: 1024 * 1024,
      windowsHide: true,
    });
    const receipt = JSON.parse(stdout);
    if (
      !receipt.reopened ||
      !receipt.editRebuildVerified ||
      !(await stat(temp)).size
    )
      throw Error("Missing native edit/reopen evidence");
    await link(temp, out);
    return { ...receipt, file: out };
  } finally {
    await rm(stage, { recursive: true, force: true });
  }
}
