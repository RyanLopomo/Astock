// Homologação contra uma janela Tauri real. Recusa o banco principal.
import { readFile,writeFile,mkdir,stat } from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const phase=process.argv.includes('--capture')?'capture':process.argv.includes('--resume')?'resume':'exercise';
const targets=await fetch('http://127.0.0.1:9223/json/list').then(r=>r.json());
const target=targets.find(t=>t.type==='page'&&t.url.includes('127.0.0.1:1420'));
if(!target)throw new Error('Janela Tauri de homologação não encontrada.');
const endpoint=new URL(target.webSocketDebuggerUrl);
if(!['localhost','127.0.0.1','[::1]'].includes(endpoint.hostname))throw new Error('CDP deve ser local.');
const socket=new WebSocket(endpoint);await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});});
let sequence=0;const waiting=new Map();
socket.addEventListener('message',e=>{const message=JSON.parse(e.data);if(message.id){const handler=waiting.get(message.id);if(handler){clearTimeout(handler.timer);waiting.delete(message.id);if(message.error)handler.reject(new Error(message.error.message));else handler.resolve(message.result);}}});
function rpc(method,params={}){const id=++sequence;return new Promise((resolve,reject)=>{const timer=setTimeout(()=>{waiting.delete(id);reject(new Error(`Timeout ${method}`));},45000);waiting.set(id,{resolve,reject,timer});socket.send(JSON.stringify({id,method,params}));});}
async function evaluate(expression){const response=await rpc('Runtime.evaluate',{expression,awaitPromise:true,returnByValue:true});if(response.exceptionDetails)throw new Error(response.exceptionDetails.text);return response.result.value;}
async function call(command,args={}){const expression=`(async()=>{try{return {ok:true,value:await window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)})};}catch(error){return {ok:false,error};}})()`;const result=await evaluate(expression);if(!result.ok){const error=new Error(result.error?.message??'Falha no comando nativo');error.code=result.error?.code;throw error;}return result.value;}
let workspace;
for(let attempt=0;attempt<100;attempt++){try{workspace=await call('get_workspace');break;}catch(e){if(e.code!=='STARTING')throw e;await new Promise(resolve=>setTimeout(resolve,200));}}
assert.ok(workspace?.dataProfile,'RECUSADO: não executar testes mutáveis no banco principal.');
const statePath=path.join(root,'.cache/native-integration-state.json');
const evidencePath=path.join(root,'docs/evidencias/native-v1-integrada.json');
const input=(command,value)=>call(command,{input:value});
const steps=[];const passed=(name)=>{steps.push(name);console.log(`PASS ${name}`);};
try{
 if(phase==='capture'){
  await evaluate(`Array.from(document.querySelectorAll('.app-topbar button')).find(b=>b.textContent.includes('Atualizar dados'))?.click()`);
  for(let attempt=0;attempt<30;attempt++){const text=await evaluate('document.body.innerText');if(text.includes('48'))break;await new Promise(resolve=>setTimeout(resolve,100));}
  for(const [name,page]of[['dashboard','Dashboard'],['lotes','Lotes e Validades'],['inventario','Inventário']]){
   await evaluate(`Array.from(document.querySelectorAll('.sidebar nav button')).find(b=>b.textContent.trim().endsWith(${JSON.stringify(page)}))?.click()`);
   await new Promise(resolve=>setTimeout(resolve,250));
   const screenshot=await rpc('Page.captureScreenshot',{format:'png',captureBeyondViewport:false});
   await writeFile(path.join(root,`docs/evidencias/v1-${name}-homologacao.png`),Buffer.from(screenshot.data,'base64'));
  }
  passed('Capturas da interface nativa com dados reais do banco de homologação');
 }else if(phase==='exercise'){
  assert.equal(workspace.skus.length,0,'Use um perfil novo para esta homologação.');
  const sku=await input('save_product',{productId:null,skuId:null,name:'AGUA SINTETICA - HOMOLOGACAO',description:'Dados de teste em banco separado',brand:'TESTE',category:'Bebidas de teste',internalCode:'TESTE-AGUA500',presentation:'500 ml',purchaseCents:1000,saleCents:2000,defaultPack:12,expiryControl:true,active:true,productActive:true,barcodes:['7890000000017']});passed('Produto real criado no banco isolado');
  const xml=await readFile(path.join(root,'crates/axis-estoque-core/tests/fixtures/nfe-compra-exemplo.xml'),'utf8');
  let imported=await input('import_xml',{content:xml,fileName:'nfe-compra-exemplo.xml'});assert.equal(imported.detail.items[0].skuId,sku.id);assert.equal((await call('get_workspace')).skus[0].balance,0);passed('XML importado em rascunho sem entrada de estoque');
  const duplicate=await input('import_xml',{content:xml,fileName:'duplicado.xml'});assert.equal(duplicate.reused,true);assert.equal(duplicate.detail.receipt.id,imported.detail.receipt.id);passed('Chave XML duplicada recupera recebimento existente');
  const receiptId=imported.detail.receipt.id;const item=imported.detail.items[0];
  const receipt=await input('save_receipt_item',{receiptId,expectedRevision:imported.detail.receipt.revision,itemId:item.id,skuId:sku.id,quantity:item.quantity,unitsPerPack:12,commercialUnit:'CX',unitPrice:item.unitPrice,totalCents:item.totalCents,lots:[{label:'TESTE-LOTE-01',manufacturedOn:'2026-11-01',expiresOn:'2026-12-15',vintage:null,quantity:60}],rememberMapping:true});
  assert.equal(receipt.items[0].units,60);await input('confirm_receipt',{id:receiptId,expectedRevision:receipt.receipt.revision,confirmed:true});
  assert.equal((await input('confirm_receipt',{id:receiptId,expectedRevision:0,confirmed:true})).alreadyCompleted,true);
  workspace=await call('get_workspace');assert.equal(workspace.skus[0].balance,60);assert.equal(workspace.skus[0].saleCents,2000);assert.equal(workspace.lots[0].balance,60);passed('Conversão 5 caixas × 12, lote e confirmação idempotente');
  const plan=await input('plan_issue',{skuId:sku.id,quantity:10,includeUndated:false,allowExpired:false});assert.equal(plan.allocations[0].lotId,workspace.lots[0].id);
  const issue={requestKey:'native-e2e-issue',skuId:sku.id,allocations:plan.allocations,reason:'Saída sintética por FEFO',allowExpired:false,confirmed:true};await input('issue_stock',issue);await input('issue_stock',issue);assert.equal((await call('get_workspace')).skus[0].balance,50);passed('Saída FEFO e repetição sem duplicar baixa');
  const inventory=await input('create_inventory',{name:'INVENTARIO SINTETICO E2E',skuIds:null});
  await assert.rejects(input('adjust_stock',{requestKey:'blocked-e2e',skuId:sku.id,lotId:null,quantity:1,reason:'Tentativa durante inventário',reference:'teste',confirmed:true}),e=>e.code==='INVENTORY_LOCK');passed('Movimentação bloqueada no Rust durante inventário');
  const unknown=await input('inventory_scan',{inventoryId:inventory.inventory.id,requestKey:'unknown-e2e',code:'https://exemplo.invalid/qr',quantity:1,skuId:null,lotId:null,lotSelected:false});assert.equal(unknown.outcome,'unknown');
  const scan={inventoryId:inventory.inventory.id,requestKey:'count-e2e',code:'7890000000017',quantity:48,skuId:sku.id,lotId:plan.allocations[0].lotId,lotSelected:true};let counted=await input('inventory_scan',scan);counted=await input('inventory_scan',scan);assert.equal(counted.detail.lines.find(l=>l.lotId===scan.lotId).counted,48);passed('Contagem por código, QR desconhecido e repetição idempotente');
  const finish={id:inventory.inventory.id,expectedRevision:counted.detail.inventory.revision,uncountedDecision:'skip',confirmed:true};await input('finish_inventory',finish);assert.equal((await input('finish_inventory',finish)).alreadyCompleted,true);
  workspace=await call('get_workspace');assert.equal(workspace.skus[0].balance,48);assert.equal(workspace.lots[0].balance,48);const movements=await input('get_movements',{search:'',skuId:sku.id,fromDate:null,toDate:null,page:0});assert.equal(movements.total,3);assert.deepEqual(movements.items.map(m=>m.quantity),[-2,-10,60]);passed('Fechamento atômico, divergência -2 e histórico único');
  await writeFile(statePath,JSON.stringify({profile:workspace.dataProfile,skuId:sku.id,receiptId,inventoryId:inventory.inventory.id,expectedBalance:48,steps},null,2));
  console.log('Exercício concluído. Feche e reabra a janela com o mesmo perfil e execute --resume.');
 }else{
  const state=JSON.parse(await readFile(statePath,'utf8'));assert.equal(workspace.dataProfile,state.profile);assert.equal(workspace.skus.find(s=>s.id===state.skuId).balance,state.expectedBalance);assert.equal(workspace.lots.find(l=>l.skuId===state.skuId).balance,48);
  const inventory=await call('get_inventory',{id:state.inventoryId});assert.equal(inventory.inventory.status,'finalized');assert.equal(inventory.eventsTotal,2);passed('Saldo, lote, sessão e eventos preservados após reinício do processo');
  let xml=await readFile(path.join(root,'crates/axis-estoque-core/tests/fixtures/nfe-compra-exemplo.xml'),'utf8');const body='3526111234567800019555001000000124112345678';let weight=2,sum=0;for(let i=body.length-1;i>=0;i--){sum+=Number(body[i])*weight;weight=weight===9?2:weight+1;}const remainder=sum%11;const key=body+(remainder<2?0:11-remainder);
  xml=xml.replace(/Id="NFe\d{44}"/,`Id="NFe${key}"`).replace('<nNF>123</nNF>','<nNF>124</nNF>').replace('<qCom>5.0000</qCom>','<qCom>2.0000</qCom>').replace('<vProd>600.00</vProd>','<vProd>240.00</vProd>').replace('<qLote>5.000</qLote>','<qLote>2.000</qLote>').replace('<qTrib>60.0000</qTrib>','<qTrib>24.0000</qTrib>');
  const next=await input('import_xml',{content:xml,fileName:'segunda-compra-sintetica.xml'});assert.equal(next.detail.items[0].skuId,state.skuId);assert.equal(next.detail.items[0].unitsPerPack,12);assert.equal(next.detail.items[0].units,24);assert.equal((await call('get_workspace')).skus[0].balance,48);passed('Associação do fornecedor e conversão recuperadas numa nova importação após reinício');
  const backup=await call('create_backup');assert.ok((await stat(backup)).size>0);passed('Backup online verificado criado pelo comando nativo');
  await mkdir(path.dirname(evidencePath),{recursive:true});await writeFile(evidencePath,JSON.stringify({date:'2026-11-07',scope:'Tauri/IPC/Rust/SQLite real em perfil de homologação separado',dataProfile:state.profile,sqliteVersion:workspace.sqliteVersion,schemaVersion:workspace.schemaVersion,steps:[...state.steps,...steps],passed:true,finalBalance:48,hardwareScannerTested:false,productionDataUsed:false},null,2));
 }
}finally{socket.close();for(const handler of waiting.values()){clearTimeout(handler.timer);handler.reject(new Error('Conexão encerrada'));}}
