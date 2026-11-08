// Verificação somente de leitura do executável release e do histórico original.
import assert from 'node:assert/strict';
import {readFile,writeFile,readdir} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import path from 'node:path';
let target;
for(let attempt=0;attempt<100;attempt++){
 const targets=await fetch('http://127.0.0.1:9224/json/list').then(r=>r.json()).catch(()=>[]);
 target=targets.find(t=>t.type==='page'&&t.url.includes('tauri.localhost'));if(target)break;
 await new Promise(resolve=>setTimeout(resolve,200));
}
assert.ok(target,'Release com interface incorporada não encontrado');
const endpoint=new URL(target.webSocketDebuggerUrl);
assert.ok(['127.0.0.1','localhost','[::1]'].includes(endpoint.hostname));
const socket=new WebSocket(endpoint);await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});});
let sequence=0;const waiting=new Map();
socket.addEventListener('message',e=>{const m=JSON.parse(e.data);const h=waiting.get(m.id);if(h){clearTimeout(h.timer);waiting.delete(m.id);if(m.error)h.reject(new Error(m.error.message));else h.resolve(m.result);}});
function rpc(method,params={}){return new Promise((resolve,reject)=>{const id=++sequence;const timer=setTimeout(()=>{waiting.delete(id);reject(new Error('CDP timeout'));},30000);waiting.set(id,{resolve,reject,timer});socket.send(JSON.stringify({id,method,params}));});}
async function evaluate(expression){const r=await rpc('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true});assert.equal(r.exceptionDetails,undefined);return r.result.value;}
async function call(command){const result=await evaluate(`(async()=>{try{return {ok:true,value:await window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)})};}catch(error){return {ok:false,error};}})()`);if(!result.ok){const error=new Error(result.error?.message);error.code=result.error?.code;throw error;}return result.value;}
try{
 let workspace;
 for(let i=0;i<100;i++){try{workspace=await call('get_workspace');break;}catch(e){if(e.code!=='STARTING')throw e;await new Promise(resolve=>setTimeout(resolve,200));}}
 assert.ok(workspace);assert.equal(workspace.dataProfile,null);assert.equal(workspace.schemaVersion,5);
 const legacy=await call('get_probe_state');const before=JSON.parse(await readFile('.cache/v1-upgrade-before.json','utf8'));
 assert.deepEqual(legacy.entries,before);assert.equal(legacy.database.totalEntries,before.length);assert.equal(legacy.database.integrity,'ok');assert.equal(legacy.database.journalMode,'wal');assert.equal(legacy.database.foreignKeys,true);assert.equal(legacy.database.synchronous,2);
 assert.equal(legacy.appVersion,'1.3.0');assert.equal(workspace.skus.length,0,'Banco original não deve receber SKUs sintéticos');
 const distHtml=await readFile('dist/index.html','utf8');const expectedScript=distHtml.match(/assets\/(index-[\w-]+\.js)/)?.[1];assert.ok(expectedScript);
 const bundledScripts=await evaluate('Array.from(document.scripts).map(s=>s.src)');assert.ok(bundledScripts.some(src=>src.endsWith('/'+expectedScript)),'Interface incorporada difere do build final');
 const viteAvailable=await fetch('http://127.0.0.1:1420',{signal:AbortSignal.timeout(1500)}).then(()=>true).catch(()=>false);assert.equal(viteAvailable,false,'Desligue o servidor de desenvolvimento antes desta verificação');
 const backupDirectory=path.join(path.dirname(workspace.databasePath),'backups');const backups=(await readdir(backupDirectory)).filter(name=>name.startsWith('antes-schema-'));
 assert.ok(backups.length>0,'Backup pré-migração não encontrado');
 await rpc('Emulation.setDeviceMetricsOverride',{width:760,height:640,deviceScaleFactor:1,mobile:false});
 const smallViewport=[];
 for(const page of ['Dashboard','Produtos','Recebimentos','Importar XML','Lotes e Validades','Inventário','Movimentações','Fornecedores','Relatórios','Configurações']){
  await evaluate(`Array.from(document.querySelectorAll('.sidebar nav button')).find(b=>b.textContent.trim().endsWith(${JSON.stringify(page)}))?.click()`);
  await new Promise(resolve=>setTimeout(resolve,150));
  const measurement=await evaluate('({heading:document.querySelector("h1")?.textContent,width:innerWidth,contentWidth:document.documentElement.scrollWidth})');
  assert.equal(measurement.heading,page==='Dashboard'?'Visão geral':page);assert.ok(measurement.contentWidth<=measurement.width,`Página com overflow: ${page}`);smallViewport.push({page,...measurement});
 }
 await rpc('Emulation.clearDeviceMetricsOverride');await evaluate(`Array.from(document.querySelectorAll('.sidebar nav button')).find(b=>b.textContent.trim().endsWith('Dashboard'))?.click()`);await new Promise(resolve=>setTimeout(resolve,200));
 const screenshot=await rpc('Page.captureScreenshot',{format:'png',captureBeyondViewport:false});await writeFile('docs/evidencias/v1-release-principal.png',Buffer.from(screenshot.data,'base64'));
 const evidence={date:'2026-11-07',passed:true,version:legacy.appVersion,frontendUrl:target.url,bundledScripts,bundledAssetsMatch:true,devServerRunning:false,dataProfile:workspace.dataProfile,schemaVersion:workspace.schemaVersion,sqliteVersion:workspace.sqliteVersion,integrity:legacy.database.integrity,journalMode:legacy.database.journalMode,foreignKeys:legacy.database.foreignKeys,synchronous:legacy.database.synchronous,legacyRowsBefore:before.length,legacyRowsAfter:legacy.database.totalEntries,legacyRowsIdentical:true,syntheticSkusInPrimary:workspace.skus.length,backupDirectory,preMigrationBackups:backups,smallViewport,executableSha256:createHash('sha256').update(await readFile('target/release/astock.exe')).digest('hex'),physicalNetworkDisconnected:false,cleanWindowsInstallationTested:false};
 await writeFile('docs/evidencias/release-v1-integrada.json',JSON.stringify(evidence,null,2));console.log(JSON.stringify(evidence,null,2));
}finally{socket.close();for(const h of waiting.values()){clearTimeout(h.timer);h.reject(new Error('CDP encerrado'));}}
