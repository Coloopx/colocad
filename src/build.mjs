import {
  mkdtemp,
  readFile,
  writeFile,
  mkdir,
  rename,
  rm,
} from "node:fs/promises";
import { join, resolve, dirname } from "node:path";
import { compileTree, kernelModel } from "./tree.mjs";
import { CadClient } from "../kernel/dist/index.js";

// Stage beside the destination. Acquiring an empty output directory with mkdir
// prevents overwriting someone else's receipt or racing a concurrent build.
export async function build(
  tree,
  destination,
  { preview = false, queryable = false, binary } = {},
) {
  const plan = compileTree(tree);
  const model = kernelModel(tree);
  if (preview) model.preview = {};
  const out = resolve(destination);
  await mkdir(dirname(out), { recursive: true });
  const stage = await mkdtemp(join(dirname(out), ".colocad-"));
  let owned = false;
  try {
    const client = new CadClient({ outputRoot: stage, binary, wallMs: 120000 });
    const result = await client.build(model,{queryable});
    if (!result.ok)
      throw Error(result.error.code + ": " + result.error.message);
    const folder = result.data.artifact_directory;
    const receipt = JSON.parse(
      await readFile(join(folder, "report.json"), "utf8"),
    );
    if (receipt.geometry.solids !== 1 || !(receipt.geometry.volume_mm3 > 0))
      throw Error("Expected a single positive-volume solid");
    const job = {
      format: "colocad/native-job/1",
      features: plan,
      expected: {
        volumeMM3: receipt.geometry.volume_mm3,
        bboxMM: receipt.geometry.bbox_mm,
      },
    };
    await writeFile(
      join(folder, "feature-tree.json"),
      JSON.stringify(tree, null, 2) + "\n",
    );
    await writeFile(
      join(folder, "native-job.json"),
      JSON.stringify(job, null, 2) + "\n",
    );
    await mkdir(out);
    owned = true;
    await rename(folder, join(out, "part"));
    return {
      directory: join(out, "part"),
      step: join(out, "part", "model.step"),
      nativeJob: join(out, "part", "native-job.json"),
      geometry: receipt.geometry,
      ...(receipt.query_revision_id?{query_revision_id:receipt.query_revision_id}:{}),
    };
  } catch (error) {
    if (owned) await rm(out, { recursive: true, force: true });
    throw error;
  } finally {
    await rm(stage, { recursive: true, force: true });
  }
}
