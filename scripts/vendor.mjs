// Verify the pinned archive and extract it atomically. Apply and verify the single reviewed downstream patch.
import {createHash} from 'node:crypto';
import {readFile,writeFile,mkdir,readdir,rename,rm,access,readlink} from 'node:fs/promises';
import {spawnSync} from 'node:child_process';
import {resolve,dirname,join,relative,isAbsolute,sep} from 'node:path';
import {fileURLToPath} from 'node:url';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const lock=JSON.parse(await readFile(join(root,'third_party/solidcore.lock.json'),'utf8'));
const archive=join(root,lock.archive), destination=join(root,'vendor/solidcore');
try {await access(archive);} catch {
  const response=await fetch(lock.url);if(!response.ok)throw Error(`dependency download: ${response.status}`);
  await writeFile(archive,new Uint8Array(await response.arrayBuffer()));
}
const digest=createHash('sha256').update(await readFile(archive)).digest('hex');
if(digest!==lock.sha256)throw Error('solidcore archive checksum mismatch');
async function treeHash(dir){
 const hash=createHash('sha256');
 async function walk(at){
  const entries=(await readdir(at,{withFileTypes:true})).sort((a,b)=>a.name.localeCompare(b.name,'en'));
  for(const item of entries){
   const path=join(at,item.name);
   if(item.isDirectory())await walk(path);
   else if(item.isFile()){hash.update(relative(dir,path).replaceAll('\\','/'));hash.update('\0');hash.update(createHash('sha256').update(await readFile(path)).digest());}
   else if(item.isSymbolicLink()){
    const target=await readlink(path);
    if(isAbsolute(target)||!resolve(dirname(path),target).startsWith(resolve(dir)+sep))throw Error('dependency link escapes its source tree');
    hash.update(relative(dir,path).replaceAll('\\','/'));hash.update('\0link\0');hash.update(target);
   }else throw Error('unsupported file in pinned dependency');
  }
 }
 await walk(dir);return hash.digest('hex');
}
const patch=lock.patch, patchFile=join(root,patch.path);
const hashFile=async path=>createHash('sha256').update(await readFile(path)).digest('hex');
if(await hashFile(patchFile)!==patch.sha256)throw Error('solidcore patch checksum mismatch');
async function patchTree(directory){
 const target=join(directory,patch.target);
 if(await hashFile(target)!==patch.before)throw Error('unexpected upstream source for patch');
 const result=spawnSync('patch',['-s','-p1','-i',patchFile],{cwd:directory,stdio:'inherit'});
 if(result.status!==0||await hashFile(target)!==patch.after)throw Error('solidcore patch did not reproduce the pinned source');
}
let exists=true;try{await access(destination);}catch{exists=false;}
const manifestPath=join(root,'vendor/solidcore.manifest.json');
if(exists){
 const saved=JSON.parse(await readFile(manifestPath,'utf8'));
 if(saved.archive_sha256!==digest||saved.tree_sha256!==await treeHash(destination))throw Error('vendor tree differs from its verified archive; use a clean vendor directory');
 if(saved.patch_sha256!==patch.sha256){
  await patchTree(destination);
  await writeFile(manifestPath,JSON.stringify({...saved,patch_sha256:patch.sha256,tree_sha256:await treeHash(destination)},null,2)+'\n');
 }else if(await hashFile(join(destination,patch.target))!==patch.after)throw Error('unexpected patched source');
 console.log('solidcore: verified source and patch');
}else{
 const stage=join(root,'vendor',`.solidcore-${process.pid}`);await mkdir(stage,{recursive:true});
 try{
  const result=spawnSync('tar',['xzf',archive,'--strip-components=1','-C',stage],{stdio:'inherit'});
  if(result.status!==0)throw Error('extracting solidcore failed');
  await patchTree(stage);
  const tree_sha256=await treeHash(stage);await rename(stage,destination);
  await writeFile(manifestPath,JSON.stringify({revision:lock.revision,archive_sha256:digest,patch_sha256:patch.sha256,tree_sha256},null,2)+'\n');
 }catch(error){await rm(stage,{recursive:true,force:true});throw error;}
 console.log('solidcore: extracted and verified pinned source');
}
