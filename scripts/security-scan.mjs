// Scanner local: nunca imprime o valor de uma possível credencial.
import {execFileSync,spawn} from 'node:child_process';
import {readFile,writeFile,readdir,lstat} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import path from 'node:path';
import {fileURLToPath} from 'node:url';

const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const rules=[
 ['private-key',/-----BEGIN (?:RSA |EC |OPENSSH |DSA |ENCRYPTED )?PRIVATE KEY-----/g],
 ['github-token',/\b(?:gh[pousr]_[A-Za-z0-9]{36,255}|github_pat_[A-Za-z0-9_]{60,255})\b/g],
 ['aws-access-id',/\b(?:AKIA|ASIA)[A-Z0-9]{16}\b/g],
 ['google-api-key',/\bAIza[0-9A-Za-z_-]{35}\b/g],
 ['stripe-live',/\b(?:sk|rk)_live_[0-9A-Za-z]{20,255}\b/g],
 ['slack-token',/\bxox[baprs]-[0-9A-Za-z-]{20,255}\b/g],
 ['credential-url',/\b(?:https?|postgres(?:ql)?|mysql|mongodb(?:\+srv)?):\/\/[^\s/:@"'<>]{1,120}:[^\s/@"'<>]{4,200}@/g],
 ['credential-assignment',/\b(?:api[_-]?key|api[_-]?token|access[_-]?token|auth[_-]?token|client[_-]?secret|password|passwd|jwt[_-]?secret|private[_-]?key)\b\s*["']?\s*[:=]\s*["']([^\s"']{12,250})["']/gi],
];
const sensitive=/\.(?:db(?:-wal|-shm|-journal)?|db3|sqlite(?:3)?(?:-wal|-shm|-journal)?|sqlcipher|pem|pfx|p12|p8|jks|keystore|dump|sql|bak|bkp|backup|zip|7z|rar|tgz|bundle)$/i;
const allowedMigration=file=>/^crates\/axis-estoque-core\/migrations\/\d{3}_[a-z_]+\.sql$/.test(file);
const findings=[];let filesScanned=0,objectsScanned=0,historyObjects=0;const scannedObjects=new Set();
function scan(text,location){
 for(const [rule,pattern]of rules){pattern.lastIndex=0;for(const m of text.matchAll(pattern)){
  const value=m[1]??m[0];if(rule==='credential-assignment'&&(/example|placeholder|changeme|invalid|your[_-]|test|synthetic/i.test(value)||new Set(value).size<6))continue;
  if(rule==='credential-url'&&/example|invalid|user:pass@/i.test(value))continue;
  findings.push({rule,location,line:text.slice(0,m.index).split('\n').length,fingerprint:createHash('sha256').update(value).digest('hex').slice(0,16)});
 }}
}
function git(args,options={}){return execFileSync('git',args,{cwd:root,maxBuffer:64*1024*1024,...options});}
async function inspectFile(file){
 const full=path.resolve(root,file);if(!full.startsWith(root+path.sep))throw new Error('Arquivo fora do projeto');
 const metadata=await lstat(full).catch(()=>null);if(!metadata||!metadata.isFile())return;
 if(metadata.size>64*1024*1024){findings.push({rule:'oversized-unscanned',location:file,line:0});return;}
 if((sensitive.test(file)&&!allowedMigration(file))||/(^|\/)\.env(?:\.|$)/.test(file)&&!file.endsWith('.example'))findings.push({rule:'sensitive-file',location:file,line:0});
 scan((await readFile(full)).toString('utf8'),file);filesScanned++;
}

async function scanHistory(){
 // Todos os blobs presentes, inclusive objetos fora de branches/tags e reflogs.
 const listing=git(['cat-file','--batch-all-objects','--batch-check=%(objectname) %(objecttype) %(objectsize)']).toString('utf8').trim().split('\n');
 const blobs=listing.map(line=>line.trim().split(' ')).filter(([,type])=>type==='blob');historyObjects=listing.length;
 const names=new Map();for(const line of git(['rev-list','--objects','--all']).toString('utf8').split('\n')){const index=line.indexOf(' ');if(index>0)names.set(line.slice(0,index),line.slice(index+1));}
 for(const [hash,,size]of blobs){if(Number(size)>64*1024*1024){findings.push({rule:'oversized-git-blob',location:`git:${hash}`,line:0});}}
 const selected=blobs.filter(([, ,size])=>Number(size)<=64*1024*1024);
 const child=spawn('git',['cat-file','--batch'],{cwd:root,stdio:['pipe','pipe','pipe']});
 let buffer=Buffer.alloc(0);let current=null;
 child.stderr.resume();
 const consume=async()=>{for await(const chunk of child.stdout){buffer=Buffer.concat([buffer,chunk]);for(;;){
  if(!current){const nl=buffer.indexOf(10);if(nl<0)break;const [hash,type,size]=buffer.subarray(0,nl).toString('utf8').split(' ');if(type!=='blob')throw new Error('Resposta Git inesperada');current={hash,size:Number(size)};buffer=buffer.subarray(nl+1);}
  if(buffer.length<current.size+1)break;
  const label=`git:${current.hash}:${names.get(current.hash)??'[objeto sem caminho atual]'}`;
  scan(buffer.subarray(0,current.size).toString('utf8'),label);scannedObjects.add(current.hash);objectsScanned++;buffer=buffer.subarray(current.size+1);current=null;
 }}};
 const consumed=consume();child.stdin.end(selected.map(([hash])=>hash+'\n').join(''));await consumed;
 const code=await new Promise((resolve,reject)=>{if(child.exitCode!==null)resolve(child.exitCode);else{child.on('exit',resolve);child.on('error',reject);}});if(code!==0)throw new Error('Falha ao ler os objetos Git');
 for(const [hash,file]of names)if(sensitive.test(file)&&!allowedMigration(file)||/(^|\/)\.env(?:\.|$)/.test(file)&&!file.endsWith('.example'))findings.push({rule:'sensitive-git-path',location:`git:${hash}:${file}`,line:0});
}

if(process.argv.includes('--self-test')){
 const synthetic='ghp_'+'Z1aB2cD3eF4gH5iJ6kL7mN8oP9qR0sT1uV2w';scan(synthetic,'[teste sintético]');
 if(!findings.some(f=>f.rule==='github-token'))throw new Error('Scanner não reconheceu o token sintético');
 console.log('Scanner: reconhecimento sintético e saída sem valores aprovados.');
}else{
 const staged=process.argv.includes('--staged');
 if(staged){for(const file of git(['diff','--cached','--name-only','--diff-filter=ACMR','-z']).toString('utf8').split('\0').filter(Boolean)){
  if(sensitive.test(file)&&!allowedMigration(file)||/(^|\/)\.env(?:\.|$)/.test(file)&&!file.endsWith('.example'))findings.push({rule:'sensitive-staged-file',location:file,line:0});
  scan(git(['show',`:${file}`]).toString('utf8'),`index:${file}`);filesScanned++;
 }}else{
  const files=git(['ls-files','--cached','--others','--exclude-standard','-z']).toString('utf8').split('\0').filter(Boolean);for(const file of new Set(files))await inspectFile(file);
  await scanHistory();
  // Configuração local pode conter URL de remoto com credencial; nunca imprimir o URL.
  scan(await readFile(path.join(root,'.git/config'),'utf8').catch(()=>''),'[git config local]');
  async function assets(dir){for(const item of await readdir(path.join(root,dir),{withFileTypes:true}).catch(()=>[])){const file=path.posix.join(dir,item.name);if(item.isSymbolicLink()){findings.push({rule:'linked-build-asset',location:file,line:0});continue;}if(item.isDirectory())await assets(file);else await inspectFile(file);}}
  await assets('dist');await assets('public');
  if(process.argv.includes('--include-local-text')){
   for(const entry of await readdir(path.join(root,'.cache'),{withFileTypes:true})){if(entry.isFile()&&/\.(?:json|log|txt|mjs|rs|toml)$/.test(entry.name)){const data=await readFile(path.join(root,'.cache',entry.name));scan(data.toString('utf8'),`local-cache:${entry.name}`);filesScanned++;}}
   for(const directory of ['.cache/npm/_logs','.cache/security-before'])for(const entry of await readdir(path.join(root,directory),{withFileTypes:true}).catch(()=>[])){if(entry.isFile()&&/\.(?:json|log|txt)$/.test(entry.name)){let data=await readFile(path.join(root,directory,entry.name));const text=data[0]===255&&data[1]===254?data.subarray(2).toString('utf16le'):data.toString('utf8');scan(text,`local-log:${entry.name}`);filesScanned++;}}
  }
  for(const name of ['astock.exe','axis-estoque-desktop.exe']){const f=path.join(root,'target/release',name);const value=await readFile(f).catch(()=>null);if(value){scan(value.toString('latin1'),`artifact:${name}`);filesScanned++;}}
 }
 const unique=Array.from(new Map(findings.map(f=>[JSON.stringify(f),f])).values());
 const result={tool:'Astock local credential-pattern scanner',date:new Date().toISOString(),mode:staged?'staged':'worktree + all local Git blobs + dist/public + PE strings',filesScanned,objectsScanned,historyObjects,uniqueBlobs:scannedObjects.size,findings:unique,limitations:['Não é prova de ausência de segredos; padrões genéricos precisam de revisão.','Não decodifica imagens, ofuscação ou seções comprimidas do PE.','Refs nunca buscadas/removidas no servidor remoto não são cobertas.']};
 const outputIndex=process.argv.indexOf('--output');if(outputIndex>=0)await writeFile(process.argv[outputIndex+1],JSON.stringify(result,null,2)+'\n');
 console.log(JSON.stringify(result,null,2));if(unique.length)process.exitCode=1;
}
