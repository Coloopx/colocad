// Executable tool-contract walkthrough; the decision sequence is scripted, not LLM inference.
import {readFile,writeFile} from 'node:fs/promises';
import {join,resolve} from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {ModelingSession} from '../../src/session.mjs';
import {createCompactSessionTools} from './compact-tools.mjs';
const exec=promisify(execFile);
export async function runGuidedWorkflow(directory){
 const root=resolve(directory),session=await ModelingSession.create(join(root,'session'),{title:'Guided workflow: build, check, edit, recover, inspect'});
 const tools=await createCompactSessionTools(session);const calls=[];
 const call=async(name,args)=>{const result=await tools.find(t=>t.name===name).execute(args);calls.push({tool:name,result});return result;};
 const model=JSON.parse(await readFile(new URL('../../skills/colocad/assets/plate-grid.json',import.meta.url)));
 const caps=await call('cad_capabilities',{});if(!caps.ok||!caps.data.operations.includes('face_hole_grid'))throw Error('Required operation unavailable');
 const validation=await call('cad_validate',{model});if(!validation.ok)throw Error(JSON.stringify(validation.error));
 const first=await call('cad_build',{model});if(!first.ok)throw Error(JSON.stringify(first.error));
 const bored=await call('cad_edit',{parameters:{holeDiameter:8}});if(!bored.ok)throw Error(JSON.stringify(bored.error));
 // Quantitative feedback comes from the analytic feature intent, not just the renderer.
 const expectedRemoved=4*Math.PI*(4**2-3**2)*6;
 if(Math.abs(first.data.geometry.volume_mm3-bored.data.geometry.volume_mm3-expectedRemoved)>.01)throw Error('Hole-size edit failed volume check');
 const invalid=await call('cad_edit',{parameters:{nonexistent:1}});if(invalid.ok||invalid.error.code!=='UNKNOWN_PARAMETER')throw Error('Expected visible failed edit');
 const accepted=await session.source();if(accepted.parameters.holeDiameter!==8)throw Error('Failure replaced accepted source');
 const preview=await call('cad_preview',{model:accepted,options:{view:'top'}});if(!preview.ok)throw Error(JSON.stringify(preview.error));
 const longer=await call('cad_edit',{parameters:{length:100}});if(!longer.ok)throw Error(JSON.stringify(longer.error));
 const [lo,hi]=longer.data.geometry.bbox_mm;if(Math.abs(hi[0]-lo[0]-100)>1e-6)throw Error('Length edit did not update the envelope');
 const html=join(root,'review.html');await exec(process.execPath,[fileURLToPath(new URL('../../bin/colocad.mjs',import.meta.url)),'replay',session.root,html]);
 const result={scope:'Scripted tool-contract workflow, not a live model evaluation',passed:true,html,steps:(await session.read()).steps.length,calls,measurements:{holeEditRemovedMm3:expectedRemoved,finalLengthMm:hi[0]-lo[0]}};
 await writeFile(join(root,'workflow.json'),JSON.stringify(result,null,2)+'\n');return result;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){const r=await runGuidedWorkflow(process.argv[2]??'./out/guided-workflow');console.log(JSON.stringify({passed:r.passed,scope:r.scope,steps:r.steps,html:r.html,measurements:r.measurements}));}
