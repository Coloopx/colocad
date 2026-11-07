import { createServer } from 'node:http';
import { mkdtemp,readFile,rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join,resolve } from 'node:path';
import { spawn } from 'node:child_process';
import { exportSessionHtml } from './session.mjs';

export function parseViewOptions(args){
 const options={port:0,open:true,cutaway:false,view:'iso'};
 for(let i=0;i<args.length;i++){
  const arg=args[i];
  if(arg==='--no-open')options.open=false;
  else if(arg==='--cutaway')options.cutaway=true;
  else if(arg==='--port'){
   const value=args[++i];if(!/^\d+$/.test(value??'')||Number(value)>65535)throw Error('--port must be an integer from 0 to 65535');options.port=Number(value);
  }else if(arg==='--export'){
   const value=args[++i];if(!value||value.startsWith('--'))throw Error('--export requires an HTML path');options.export=resolve(value);
  }else if(arg==='--view'){
   const value=args[++i];if(!['iso','front','side','top'].includes(value))throw Error('--view must be iso, front, side or top');options.view=value;
  }else throw Error('Unknown viewer option: '+arg);
 }
 return options;
}
function openBrowser(url){
 const command=process.platform==='darwin'?'open':process.platform==='win32'?'cmd':'xdg-open';
 const args=process.platform==='win32'?['/c','start','',url]:[url];
 const child=spawn(command,args,{stdio:'ignore'});
 child.on('error',()=>process.stderr.write('Browser could not be opened; use the printed viewer URL.\n'));
 child.on('exit',code=>{if(code)process.stderr.write('Browser opener exited '+code+'; use the printed viewer URL.\n');});
 child.unref();
}

export async function startViewer(directory,options={}){
 const root=resolve(directory),temp=await mkdtemp(join(tmpdir(),'colocad-view-'));
 const htmlPath=options.export??join(temp,'history.html');let html,server;
 try{
  await exportSessionHtml(root,htmlPath,{cutaway:options.cutaway??false,view:options.view??'iso'});html=await readFile(htmlPath);
  server=createServer((request,response)=>{
   const path=new URL(request.url,'http://127.0.0.1').pathname;
   if(path==='/favicon.ico'){response.writeHead(204);response.end();return;}
   if(path!=='/'||!['GET','HEAD'].includes(request.method)){response.writeHead(404);response.end('Not found');return;}
   response.writeHead(200,{'Content-Type':'text/html; charset=utf-8','Content-Length':html.length,'Cache-Control':'no-store','X-Content-Type-Options':'nosniff'});
   response.end(request.method==='HEAD'?undefined:html);
  });
  await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(options.port??0,'127.0.0.1',resolve);});
 }catch(error){await rm(temp,{recursive:true,force:true});throw error;}
 const address=server.address(),url='http://127.0.0.1:'+address.port+'/';
 if(options.open!==false)openBrowser(url);
 return {url,exportedHtml:options.export??null,async close(){await new Promise(resolve=>{server.close(resolve);server.closeAllConnections();});await rm(temp,{recursive:true,force:true});}};
}
