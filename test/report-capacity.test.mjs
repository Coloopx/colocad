/**
 * [INPUT]: 真实公开native/SDK、小盒体及合法实例metadata；不替换build/transport。
 * [OUTPUT]: 报告容量、checkpoint冷建及reportBytes发布/读取合同的公开回归。
 * [POS]: 公开容量合同回归；真实native/SDK、Windows执行，保留诊断目录。
 * [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
 */
import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,readFile,writeFile,readdir,mkdir,stat} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {CadClient,resolveBinary,DEFAULT_RESOURCE_LIMITS,RESOURCE_LIMIT_CEILINGS} from '../kernel/dist/index.js';

const MiB=1024*1024,sha=b=>createHash('sha256').update(b).digest('hex');
const box=()=>({version:1,units:'mm',parameters:{},features:[{id:'box',op:'box',origin:[0,0,0],size:[2,3,4]}],output:'box'});
function assembly(){
  const m=box();delete m.output;
  m.assembly=Array.from({length:96},(_,n)=>({id:'instance-'+n,input:'box',translation:[4*n,0,0],metadata:{public_note:'x'.repeat(24*1024)}}));
  return m;
}
async function root(t){
  const path=await mkdtemp(join(process.env.COLOCAD_REPORT_RED_ROOT??tmpdir(),'colocad-report-red-'));
  t.diagnostic('retained report capacity evidence: '+path);return path;
}
async function direct(t,path,model,resourceLimits){
  const binary=resolveBinary(),out=join(path,'native-proof');
  const args=['build','--out',out,'--checkpoints'];
  if(resourceLimits)args.push('--resource-limits',JSON.stringify(resourceLimits));
  const r=spawnSync(binary,args,{input:JSON.stringify(model),maxBuffer:16*MiB,timeout:120000});
  assert.ifError(r.error);
  await writeFile(join(path,'native-stdout.json'),r.stdout);await writeFile(join(path,'native-stderr.txt'),r.stderr);
  const receipt=JSON.parse(r.stdout.toString('utf8'));
  assert.equal(r.status,receipt.ok?0:1,JSON.stringify(receipt));
  t.diagnostic(JSON.stringify({binary_sha256:sha(await readFile(binary)),stdout_bytes:r.stdout.length,native_ok:receipt.ok}));
  return {out,receipt,stdoutBytes:r.stdout.length};
}
const accepted=r=>{assert.equal(r.ok,true,JSON.stringify(r));return r.data;};
const denied=limit=>e=>{
  assert.equal(e.code,'RESOURCE_LIMIT');assert.equal(e.details?.resource,'reportBytes');
  assert.equal(e.details.limit_bytes,limit);assert.ok(e.details.observed_bytes>limit);return true;
};

test('R1 real >2MiB native success survives SDK delivery with the same accepted STEP',async t=>{
  const path=await root(t),m=assembly(),proof=await direct(t,path,m);
  accepted(proof.receipt);assert.ok(proof.stdoutBytes>2*MiB&&proof.stdoutBytes<8*MiB);
  const original=await readFile(join(proof.out,'model.step'));
  assert.ok(original.length<32*MiB);assert.equal(sha(original),proof.receipt.data.artifact_sha256['model.step']);
  const jobs=join(path,'sdk'),cad=new CadClient({outputRoot:jobs,wallMs:120000});
  const r=await cad.build(m);
  await writeFile(join(path,'sdk-result.json'),JSON.stringify(r,null,2));
  t.diagnostic(JSON.stringify({sdk_ok:r.ok,error:r.error,remaining_jobs:await readdir(jobs)}));
  const report=accepted(r);
  assert.equal(report.geometry.solids,96);assert.ok(Math.abs(report.geometry.volume_mm3-2304)<1e-6);
  assert.equal(report.assembly.instances.length,96);
  assert.equal(report.assembly.instances[95].metadata.public_note.length,24*1024);
  const delivered=await readFile(join(report.artifact_directory,'model.step'));
  assert.deepEqual(delivered,original);assert.equal(sha(delivered),report.artifact_sha256['model.step']);
});

test('R2 oversized previous report causes safe cold rebuild, never CHECKPOINT_IO or old-file mutation',async t=>{
  const path=await root(t),m=box(),proof=await direct(t,path,m);accepted(proof.receipt);
  const file=join(proof.out,'report.json'),previous=JSON.parse(await readFile(file,'utf8'));
  previous.public_padding='x'.repeat(2*MiB+1);const raw=Buffer.from(JSON.stringify(previous));
  await writeFile(file,raw);
  const r=await new CadClient({outputRoot:join(path,'sdk'),wallMs:120000}).build(m,{previousRevision:proof.out});
  await writeFile(join(path,'sdk-result.json'),JSON.stringify(r,null,2));
  const report=accepted(r);
  assert.equal(report.validation.valid,true);assert.ok(Math.abs(report.geometry.volume_mm3-24)<1e-6);
  assert.ok(report.features.every(f=>f.cache_hit!==true),'oversized legacy manifest must not authorize cached geometry');
  assert.deepEqual(report.checkpoint_recovery,{mode:'cold',reason:'oversized_legacy_manifest'});
  assert.equal(sha(await readFile(file)),sha(raw),'original report remains immutable');
  assert.equal(sha(await readFile(join(report.artifact_directory,'model.step'))),proof.receipt.data.artifact_sha256['model.step']);
});

test('reportBytes defaults/ceilings and snapshot are independent from input and STEP permission',async t=>{
  const path=await root(t),ordinary=new CadClient({outputRoot:path});
  assert.equal(DEFAULT_RESOURCE_LIMITS.reportBytes,8*MiB);assert.equal(RESOURCE_LIMIT_CEILINGS.reportBytes,128*MiB);
  const limits={reportBytes:4*MiB};let reads=0;
  const cad=new CadClient({outputRoot:path,get resourceLimits(){reads++;return limits;}});
  limits.reportBytes=1;assert.equal(reads,1);assert.equal(cad.resourceLimits.reportBytes,4*MiB);
  assert.equal(cad.resourceLimits.inputBytes,ordinary.resourceLimits.inputBytes);
  assert.equal(cad.resourceLimits.outputStepBytes,ordinary.resourceLimits.outputStepBytes);
  for(const reportBytes of [0,1,1023,-1,1.5,Infinity,NaN,128*MiB+1,'4096'])
    assert.throws(()=>new CadClient({outputRoot:path,resourceLimits:{reportBytes}}),e=>e.code==='INVALID_RESOURCE_LIMITS');
});

test('native explicit reportBytes refuses oversized receipt before atomically publishing artifacts',async t=>{
  const path=await root(t),proof=await direct(t,path,assembly(),{reportBytes:64*1024});
  assert.equal(proof.receipt.ok,false);denied(64*1024)(proof.receipt.error);
  await assert.rejects(stat(proof.out),{code:'ENOENT'});
});

test('SDK explicit reportBytes admits a real report above 2MiB without granting new STEP input permission',async t=>{
  const path=await root(t),cad=new CadClient({outputRoot:path,wallMs:120000,resourceLimits:{reportBytes:4*MiB}});
  const report=accepted(await cad.build(assembly()));
  assert.equal(report.geometry.solids,96);assert.equal(report.resource_limits.reportBytes,4*MiB);
  assert.equal(cad.resourceLimits.stepBytes,24*MiB);
  assert.ok((await stat(join(report.artifact_directory,'report.json'))).size>2*MiB);
});

test('SDK inspect enforces exact UTF8 reportBytes rather than producer permission',async t=>{
  const path=await root(t),directory=join(path,'artifact');await mkdir(directory);
  const raw=Buffer.from(JSON.stringify({public_note:'日'.repeat(2000),resource_limits:{reportBytes:128*MiB}}));
  await writeFile(join(directory,'report.json'),raw);
  const exact=new CadClient({outputRoot:path,resourceLimits:{reportBytes:raw.length}});
  assert.equal((await exact.inspect(directory)).public_note.length,2000);
  const strict=new CadClient({outputRoot:path,resourceLimits:{reportBytes:raw.length-1}});
  await assert.rejects(strict.inspect(directory),denied(raw.length-1));
});

test('report host option is rejected in model IR rather than granting authority',async t=>{
  const path=await root(t),cad=new CadClient({outputRoot:path});
  for(const field of ['reportBytes','resourceLimits']){
    const m=box();m[field]=field==='reportBytes'?128*MiB:{reportBytes:128*MiB};
    assert.equal((await cad.validate(m)).ok,false);
  }
});
