#!/usr/bin/env node
import { readFile, writeFile } from "node:fs/promises";
import { compileTree, build, kernelModel, CadClient } from "../src/index.mjs";
import { ModelingSession, exportSessionHtml } from "../src/session.mjs";
import { startViewer, parseViewOptions } from "../src/view-server.mjs";
import { renderSession, parseRenderOptions } from "../src/render.mjs";
import { writeSolidWorks } from "../src/solidworks.mjs";
const [cmd, input, output, ...flags] = process.argv.slice(2);
const help = "colocad migrate SESSION_DIRECTORY\ncolocad history SESSION_DIRECTORY [--after CURSOR --limit COUNT | --all]\n"+
  "colocad query REVISION_DIR REQUEST.json [--out NEW_RECEIPT.json]\n"+
  "colocad build TREE.json NEW_DIRECTORY [--preview] [--queryable]\ncolocad plan TREE.json\ncolocad record MODEL.json SESSION_DIRECTORY [--label TEXT] [--queryable]\ncolocad replay SESSION_DIRECTORY OUTPUT.html\ncolocad view SESSION_DIRECTORY [--no-open] [--port PORT] [--export OUTPUT.html] [--cutaway] [--view iso|front|side|top]\ncolocad render SESSION_DIRECTORY OUTPUT.mp4|OUTPUT.gif [--width N] [--height N] [--fps N] [--seconds-per-step N]\ncolocad solidworks NATIVE-JOB.json NEW_FILE.SLDPRT";
try {
  if (cmd === "--help" || cmd === "-h" || !cmd) console.log(help);
  else if(cmd==='migrate' && input && !output) {
    console.log(JSON.stringify(await ModelingSession.migrate(input)));
  }
  else if(cmd==='history' && input) {
    const session=new ModelingSession(input);
    const args=process.argv.slice(4);
    if(args.length===1 && args[0]==='--all') console.log(JSON.stringify(await session.read()));
    else {
      const options={};
      for(let i=0;i<args.length;i+=2) {
        const key={'--after':'after','--limit':'limit'}[args[i]];
        if(!key || Object.hasOwn(options,key) || !/^\d+$/.test(args[i+1]??'')) throw Error(help);
        options[key]=Number(args[i+1]);
      }
      console.log(JSON.stringify(await session.history(options)));
    }
  }
  else if(cmd==='query'&&input&&output&&(flags.length===0||(flags.length===2&&flags[0]==='--out'))){
    const bytes=await readFile(output);if(bytes.length>2*1024*1024)throw Error('Query input exceeds 2 MiB');
    const client=new CadClient({outputRoot:input,queryRoots:[input],wallMs:120000});
    const receipt=await client.query(input,JSON.parse(bytes));
    if(flags.length)await writeFile(flags[1],JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});
    if(receipt.ok)console.log(JSON.stringify(receipt));else{console.error(JSON.stringify(receipt));process.exitCode=1;}
  }
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
    if (cmd === "record" && output) {
      let label=input,queryable;for(let i=0;i<flags.length;i++){if(flags[i]==='--queryable'&&queryable===undefined)queryable=true;else if(flags[i]==='--label'&&flags[i+1]!==undefined)label=flags[++i];else throw Error(help);}
      let session;
      try { session=await ModelingSession.create(output,{wallMs:120000}); }
      catch(error) { if(error.code!=="EEXIST") throw error; session=await ModelingSession.open(output,{wallMs:120000}); }
      res=await session.build(spec.format==="colocad/1"?kernelModel(spec):spec,{label,queryable});
      if(!res.ok) process.exitCode=1;
    }
    else if (cmd === "plan" && !output) res = compileTree(spec);
    else if (
      cmd === "build" &&
      output &&
      flags.every(flag=>['--preview','--queryable'].includes(flag))&&new Set(flags).size===flags.length
    )
      res = await build(spec, output, { preview: flags.includes('--preview'),queryable:flags.includes('--queryable') });
    else if (cmd === "solidworks" && output && !flags.length)
      res = await writeSolidWorks(spec, output);
    else throw Error(help);
    console.log(JSON.stringify(res));
  }
} catch (e) {
  console.error(JSON.stringify(cmd==='query'?{ok:false,error:{code:'CLI_QUERY_ERROR',message:e.message}}:{ error: e.message }));
  process.exitCode = 1;
}
