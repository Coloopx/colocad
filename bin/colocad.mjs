#!/usr/bin/env node
import { readFile } from "node:fs/promises";
import { compileTree, build } from "../src/index.mjs";
import { writeSolidWorks } from "../src/solidworks.mjs";
const [cmd, input, output, ...flags] = process.argv.slice(2);
const help =
  "colocad build TREE.json NEW_DIRECTORY [--preview]\ncolocad plan TREE.json\ncolocad solidworks NATIVE-JOB.json NEW_FILE.SLDPRT";
try {
  if (cmd === "--help" || cmd === "-h" || !cmd) console.log(help);
  else {
    if (!input || !["plan", "build", "solidworks"].includes(cmd))
      throw Error(help);
    const raw = await readFile(input);
    if (raw.length > 2 * 1024 * 1024) throw Error("Input exceeds 2 MiB");
    const spec = JSON.parse(raw.toString("utf8"));
    let res;
    if (cmd === "plan" && !output) res = compileTree(spec);
    else if (
      cmd === "build" &&
      output &&
      (flags.length === 0 || (flags.length === 1 && flags[0] === "--preview"))
    )
      res = await build(spec, output, { preview: flags.length > 0 });
    else if (cmd === "solidworks" && output && !flags.length)
      res = await writeSolidWorks(spec, output);
    else throw Error(help);
    console.log(JSON.stringify(res));
  }
} catch (e) {
  console.error(JSON.stringify({ error: e.message }));
  process.exitCode = 1;
}
