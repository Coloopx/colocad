#!/usr/bin/env node
import { readFile } from "node:fs/promises";
import { compileTree, build, kernelModel } from "../src/index.mjs";
import { ModelingSession, exportSessionHtml } from "../src/session.mjs";
import { startViewer, parseViewOptions } from "../src/view-server.mjs";
import { renderSession, parseRenderOptions } from "../src/render.mjs";
import { writeSolidWorks } from "../src/solidworks.mjs";
const [cmd, input, output, ...flags] = process.argv.slice(2);
const help =
  "colocad build TREE.json NEW_DIRECTORY [--preview]\ncolocad plan TREE.json\ncolocad record MODEL.json SESSION_DIRECTORY [--label TEXT]\ncolocad replay SESSION_DIRECTORY OUTPUT.html\ncolocad view SESSION_DIRECTORY [--no-open] [--port PORT] [--export OUTPUT.html] [--cutaway] [--view iso|front|side|top]\ncolocad render SESSION_DIRECTORY OUTPUT.mp4|OUTPUT.gif [--width N] [--height N] [--fps N] [--seconds-per-step N]\ncolocad solidworks NATIVE-JOB.json NEW_FILE.SLDPRT";
try {
  if (cmd === "--help" || cmd === "-h" || !cmd) console.log(help);
  else if (cmd === "render" && input && output) {
    console.log(JSON.stringify(await renderSession(input,output,parseRenderOptions(flags))));
  }
  else if (cmd === "view" && input) {
    const options=parseViewOptions(process.argv.slice(4));
    const viewer=await startViewer(input,options);
    console.log(JSON.stringify({event:'viewer-ready',url:viewer.url,exportedHtml:viewer.exportedHtml}));
    let closing=false;
    const close=async()=>{if(closing)return;closing=true;await viewer.close();process.exitCode=0;};
    process.once('SIGINT',close);process.once('SIGTERM',close);
  }
  else if (cmd === "replay" && input && output && !flags.length) {
    console.log(JSON.stringify(await exportSessionHtml(input, output)));
  }
  else {
    if (!input || !["plan", "build", "solidworks", "record"].includes(cmd))
      throw Error(help);
    const raw = await readFile(input);
    if (raw.length > 2 * 1024 * 1024) throw Error("Input exceeds 2 MiB");
    const spec = JSON.parse(raw.toString("utf8"));
    let res;
    if (cmd === "record" && output && (!flags.length || (flags.length===2 && flags[0]==="--label"))) {
      let session;
      try { session=await ModelingSession.create(output,{wallMs:120000}); }
      catch(error) { if(error.code!=="EEXIST") throw error; session=await ModelingSession.open(output,{wallMs:120000}); }
      res=await session.build(spec.format==="colocad/1"?kernelModel(spec):spec,{label:flags[1]??input});
      if(!res.ok) process.exitCode=1;
    }
    else if (cmd === "plan" && !output) res = compileTree(spec);
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
