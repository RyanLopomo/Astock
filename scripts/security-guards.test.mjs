import {test} from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,mkdir,writeFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {execFileSync,spawnSync} from 'node:child_process';
import {readFile} from 'node:fs/promises';
import {checkConfiguration,checkDistribution} from './security-config-check.mjs';
test('CSP, comandos explícitos e ausência de updater são verificados',async()=>{assert.equal((await checkConfiguration()).localCommands,26);});
test('o verificador de distribuição rejeita banco, env, source map e asset órfão',async()=>{
 const dir=await mkdtemp(path.join(tmpdir(),'astock-build-security-'));
 try{await mkdir(path.join(dir,'assets'));await writeFile(path.join(dir,'index.html'),'<script src="/assets/app.js"></script>');await writeFile(path.join(dir,'assets/app.js'),'console.log("synthetic")');assert.equal((await checkDistribution(dir)).files,2);
 for(const name of ['database.db','.env','assets/app.js.map','assets/old.js']){const target=path.join(dir,name);await writeFile(target,'synthetic');await assert.rejects(checkDistribution(dir));await rm(target);}
 }finally{assert.equal(path.dirname(path.resolve(dir)),path.resolve(tmpdir()));assert.ok(path.basename(dir).startsWith('astock-build-security-'));await rm(dir,{recursive:true,force:true});}
});
test('scanner staged bloqueia segredo sintético e não imprime o valor',async()=>{
 const dir=await mkdtemp(path.join(tmpdir(),'astock-secret-fixture-'));
 try{
  await mkdir(path.join(dir,'scripts'));const scanner=new URL('./security-scan.mjs',import.meta.url);await writeFile(path.join(dir,'scripts/security-scan.mjs'),await readFile(scanner));
  execFileSync('git',['init','-q'],{cwd:dir});const synthetic='ghp_'+'Z1aB2cD3eF4gH5iJ6kL7mN8oP9qR0sT1uV2w';await writeFile(path.join(dir,'synthetic.txt'),synthetic);execFileSync('git',['add','synthetic.txt'],{cwd:dir});
  const result=spawnSync(process.execPath,['scripts/security-scan.mjs','--staged'],{cwd:dir,encoding:'utf8'});assert.equal(result.status,1);assert.ok(!result.stdout.includes(synthetic));assert.ok(JSON.parse(result.stdout).findings.some(f=>f.rule==='github-token'));
 }finally{assert.equal(path.dirname(path.resolve(dir)),path.resolve(tmpdir()));assert.ok(path.basename(dir).startsWith('astock-secret-fixture-'));await rm(dir,{recursive:true,force:true});}
});
