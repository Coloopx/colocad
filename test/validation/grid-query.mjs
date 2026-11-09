import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir,cp} from 'node:fs/promises';
import {join,resolve,relative} from 'node:path';
import {pathToFileURL} from 'node:url';
import {execFileSync} from 'node:child_process';

export async function checkGridQuery({CadClient,ModelingSession,pkg,out}) {
  await mkdir(out);
  const model=JSON.parse(await readFile(join(pkg,'skills/colocad/assets/plate-grid.json'),'utf8'));
  const cad=new CadClient({outputRoot:out,wallMs:120000});
  const checked=r=>{assert.equal(r.ok,true,JSON.stringify(r));return r.data;};
  const plain=checked(await cad.build(model));
  model.preview={width:160,height:120,chord_mm:1};
  const session=await ModelingSession.create(join(out,'session'),{wallMs:120000});
  const initial=checked(await session.build(model,{queryable:true}));
  const request=rev=>({schema:'colocad.query/1',query_revision_id:rev,
    target:{definition_id:'mounts'},space:'definition',tolerance:{linear_mm:1e-5,angular_rad:1e-7},
    queries:[{id:'bores',op:'holes',where:{termination:'through'}}]});
  const observations=[];
  const observe=async(data,centers,diameter)=>{
    const receipt=checked(await session.query(request(data.query_revision_id))),r=receipt.results[0];
    assert.equal(r.status,'complete');assert.equal(r.items.length,centers.length);
    for(const hole of r.items){
      assert.ok(hole.segments.length>0);
      for(const s of hole.segments)assert.ok(Math.abs(s.diameter_mm-diameter)<1e-5);
    }
    const actual=r.items.map(h=>h.axis.point_mm.slice(0,2)).sort((a,b)=>a[0]-b[0]||a[1]-b[1]);
    const expected=centers.toSorted((a,b)=>a[0]-b[0]||a[1]-b[1]);
    actual.forEach((p,i)=>p.forEach((v,j)=>assert.ok(Math.abs(v-expected[i][j])<1e-5)));
    for(const k of ['geometry_build_calls','step_export_calls','tessellation_calls'])assert.equal(receipt.execution[k],0);
    observations.push(receipt);return receipt;
  };
  const centers4=[[15,10],[15,40],[65,10],[65,40]];
  const before=await observe(initial,centers4,6);
  const moved=join(out,'relocated');await cp(initial.artifact_directory,moved,{recursive:true});
  const reopened=checked(await new CadClient({outputRoot:out,wallMs:120000}).query(moved,request(initial.query_revision_id)));
  assert.deepEqual(reopened.results,before.results);
  const edited=checked(await session.edit({length:100,xCount:3,xPitch:32,holeDiameter:8}));
  assert.equal((await session.query(request(initial.query_revision_id))).error.code,'QUERY_REVISION_MISMATCH');
  const centers6=[18,50,82].flatMap(x=>[10,40].map(y=>[x,y]));
  await observe(edited,centers6,8);
  assert.ok(Math.abs(edited.geometry.volume_mm3-(100*50-6*Math.PI*16)*6)<1e-5);
  const cli=join(pkg,'bin/colocad.mjs'),review=join(out,'review.html');
  const replay=JSON.parse(execFileSync(process.execPath,[cli,'replay',session.root,review],{encoding:'utf8'}));
  assert.equal(replay.steps,5);assert.equal(replay.meshDefinitions,2);
  const rows=[['plain',plain,80,centers4,3],['queryable',initial,80,centers4,3],['edited',edited,100,centers6,4]]
    .map(([id,data,length,centers,radius])=>({id,artifact_directory:relative(out,data.artifact_directory),length,centers,radius,receipt:data}));
  await writeFile(join(out,'geometry-inputs.json'),JSON.stringify(rows,null,2)+'\n');
  await writeFile(join(out,'observations.json'),JSON.stringify(observations,null,2)+'\n');
  const summary={passed:true,platform:process.platform+'-'+process.arch,
    checks:['bundled_grid_queryable_preview','four_hole_centers_diameters','six_hole_parameter_edit','stale_revision_rejection','relocation_preserves_refs','query_no_build_export_tessellation','cli_html_replay'],
    revision:edited.query_revision_id,html_bytes:(await readFile(review)).length};
  await writeFile(join(out,'summary.json'),JSON.stringify(summary,null,2)+'\n');return summary;
}

if(process.argv[1]&&import.meta.url===pathToFileURL(resolve(process.argv[1])).href){
  assert.ok(!process.env.AGENT_CAD_BINARY,'Archive checks must use the installed native');
  const pkg=resolve('node_modules/@coloopx/colocad');
  const {CadClient}=await import(pathToFileURL(join(pkg,'src/index.mjs')));
  const {ModelingSession}=await import(pathToFileURL(join(pkg,'src/session.mjs')));
  console.log(JSON.stringify(await checkGridQuery({CadClient,ModelingSession,pkg,out:resolve(process.argv[2]??'grid-release-check')})));
}
