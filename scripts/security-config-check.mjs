import {readFile,readdir,lstat} from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import assert from 'node:assert/strict';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');

export async function checkDistribution(directory){
 const files=[];
 async function walk(dir){for(const item of await readdir(dir,{withFileTypes:true})){const full=path.join(dir,item.name);const metadata=await lstat(full);assert.ok(!metadata.isSymbolicLink(),`Asset com link: ${item.name}`);if(item.isDirectory())await walk(full);else files.push(path.relative(directory,full).replaceAll('\\','/'));}}
 await walk(directory);assert.ok(files.includes('index.html'),'Build sem index.html');
 const html=await readFile(path.join(directory,'index.html'),'utf8');
 const expected=new Set(['index.html']);
 for(const m of html.matchAll(/(?:src|href)=["']([^"']+)["']/g)){
  assert.ok(m[1].startsWith('/assets/'),`Referência externa ou inesperada no HTML: ${m[1].split('?')[0]}`);expected.add(m[1].slice(1));
 }
 for(const file of files){assert.ok(/^(?:index\.html|assets\/[\w.-]+\.(?:js|css|woff2?|png|svg|ico))$/.test(file),`Arquivo privado ou inesperado no build: ${file}`);assert.ok(!file.endsWith('.map'),'Source map no artefato');}
 // Os bundles atuais não usam importações dinâmicas. Novos chunks/assets exigem revisão deste manifesto.
 for(const file of files)assert.ok(expected.has(file),`Asset não referenciado no build atual: ${file}`);
 return {files:files.length,sourceMaps:false,sensitiveFiles:false};
}

export async function checkConfiguration(){
 const config=JSON.parse(await readFile(path.join(root,'src-tauri/tauri.conf.json'),'utf8'));
 const cap=JSON.parse(await readFile(path.join(root,'src-tauri/capabilities/main.json'),'utf8'));
 assert.deepEqual(config.app.security.capabilities,['main']);assert.deepEqual(cap.windows,['main']);assert.equal(cap.remote,undefined);
 const commands=[...(await readFile(path.join(root,'src-tauri/commands.rs'),'utf8')).matchAll(/"(\w+)"/g)].map(m=>'allow-'+m[1].replaceAll('_','-')).sort();assert.deepEqual([...cap.permissions].sort(),commands);
 const csp=config.app.security.csp;for(const directive of ["default-src 'self'","script-src 'self'","object-src 'none'","frame-src 'none'","form-action 'none'","frame-ancestors 'none'"])assert.ok(csp.includes(directive));
 assert.ok(!/unsafe-eval|unsafe-inline|https:|\*/.test(csp),'CSP de produção permissiva');assert.equal(config.app.security.dangerousDisableAssetCspModification,undefined);
 assert.equal(config.app.windows[0].create,false);assert.equal(config.app.windows[0].dragDropEnabled,false);
 assert.equal(config.plugins?.updater,undefined,'Ativação de updater exige revisão de HTTPS, assinatura e recuperação');
 assert.ok(!config.bundle.resources&&!config.bundle.externalBin,'Recursos nativos extras exigem revisão de publicação');
 const vite=await readFile(path.join(root,'vite.config.ts'),'utf8');assert.ok(vite.includes("envPrefix: 'ASTOCK_PUBLIC_'"));assert.ok(!vite.includes('sourcemap: true'));
 return {localCommands:commands.length,cspRestricted:true,remoteCapabilities:false,updaterEnabled:false};
}
if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url)){
 try{const result={configuration:await checkConfiguration()};if(process.argv.includes('--dist'))result.distribution=await checkDistribution(path.join(root,'dist'));console.log(JSON.stringify(result,null,2));}
 catch(error){console.error(error.message);process.exitCode=1;}
}
