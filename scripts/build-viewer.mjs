import { build } from 'esbuild';
import { mkdir,copyFile } from 'node:fs/promises';
await mkdir('viewer/dist',{recursive:true});
for(const [format,file] of [['iife','standalone.js'],['esm','index.js']])await build({entryPoints:['viewer/index.js'],outfile:'viewer/dist/'+file,bundle:true,format,minify:true,legalComments:'inline',target:['es2022']});
await copyFile('node_modules/three/LICENSE','viewer/THREE-LICENSE');
