/**
 * [INPUT]: 实际vendor脚本、公开合成归档及两个同文件顺序补丁。
 * [OUTPUT]: 重复准备幂等、目标/归档/补丁篡改拒绝的真实文件回归。
 * [POS]: 固定依赖构建边界，不运行几何或访问网络。
 * [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
 */
import test from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {readFile,writeFile,mkdir,mkdtemp,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
const hash=b=>createHash('sha256').update(b).digest('hex');
const target='crates/solidcore/src/example.rs';
const value=n=>`pub const VALUE: u8 = ${n};\n`;
const patch=(a,b)=>`--- a/${target}\n+++ b/${target}\n@@ -1 +1 @@\n-${value(a)}+${value(b)}`;

test('hash-locked same-target patches are idempotent and reject tampering',async t=>{
 const root=await mkdtemp(join(tmpdir(),'colocad-vendor-chain-'));
 t.after(()=>rm(root,{recursive:true,force:true}));
 const src=join(root,'raw/pkg',target),third=join(root,'third_party');
 await mkdir(join(root,'raw/pkg/crates/solidcore/src'),{recursive:true});
 await mkdir(join(root,'scripts'));await mkdir(third);
 const script=join(root,'scripts/vendor.mjs');
 await writeFile(script,await readFile(new URL('../scripts/vendor.mjs',import.meta.url)));
 await writeFile(src,value(1));
 const archive=join(third,'fixture.tar.gz');
 const archived=spawnSync('tar',['czf',archive,'-C',join(root,'raw'),'pkg'],{encoding:'utf8'});
 assert.equal(archived.status,0,archived.stderr);
 for(const [a,b]of [[1,2],[2,3]])await writeFile(join(third,`p${a}.patch`),patch(a,b));
 const lock={revision:'public-synthetic-patch-chain',archive:'third_party/fixture.tar.gz',
  sha256:hash(await readFile(archive)),patch:{path:'third_party/p1.patch',sha256:hash(patch(1,2)),
   target,before:hash(value(1)),after:hash(value(2))},
  additional_patches:[{path:'third_party/p2.patch',sha256:hash(patch(2,3)),
   targets:[{target,before:hash(value(2)),after:hash(value(3))}]}]};
 await writeFile(join(third,'solidcore.lock.json'),JSON.stringify(lock));
 const run=()=>spawnSync(process.execPath,[script],{cwd:root,encoding:'utf8'});
 const first=run();assert.equal(first.status,0,first.stderr);
 const delivered=join(root,'vendor/solidcore',target);
 assert.equal(await readFile(delivered,'utf8'),value(3));
 const repeat=run();assert.equal(repeat.status,0,repeat.stderr);
 assert.equal(await readFile(delivered,'utf8'),value(3));

 // ---- 即使伪造本地manifest整树摘要，也不能绕过锁中的最终目标SHA ----
 await writeFile(delivered,value(4));
 const manifest=join(root,'vendor/solidcore.manifest.json');
 const saved=JSON.parse(await readFile(manifest,'utf8'));
 saved.tree_sha256=createHash('sha256').update(target).update('\0')
  .update(createHash('sha256').update(value(4)).digest()).digest('hex');
 await writeFile(manifest,JSON.stringify(saved));
 const wrongTarget=run();assert.notEqual(wrongTarget.status,0);
 assert.match(wrongTarget.stderr,/unexpected source|incorrect patched file/);
 assert.equal(await readFile(delivered,'utf8'),value(4));

 await writeFile(join(third,'p2.patch'),patch(2,3)+'\n');
 const wrongPatch=run();assert.notEqual(wrongPatch.status,0);
 assert.match(wrongPatch.stderr,/patch checksum mismatch/);
 await writeFile(join(third,'p2.patch'),patch(2,3));
 await writeFile(archive,Buffer.concat([await readFile(archive),Buffer.from('x')]));
 const wrongArchive=run();assert.notEqual(wrongArchive.status,0);
 assert.match(wrongArchive.stderr,/archive checksum mismatch/);
});
