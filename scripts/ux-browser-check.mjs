// Browser-only UX verification. Synthetic clients are injected by Playwright;
// no test data or persistence substitute is included in the distributed app.
/* global document, HTMLInputElement, requestAnimationFrame, innerWidth, window */
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { access, mkdir, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const require = createRequire(path.join(root, 'package.json'));
const phase = process.argv.includes('--before') ? 'before' : 'after';
const modulePath = process.env.PLAYWRIGHT_MODULE_PATH || require.resolve('playwright');
const { chromium } = await import(pathToFileURL(modulePath).href);
const port = Number(process.env.ASTOCK_UX_PORT || 1431);
assert.ok(port !== 1420, 'Use a separate UX server, never the native development port.');
const baseURL = `http://127.0.0.1:${port}`;
const output = path.join(root, 'docs/ux-ui/evidencias');
await mkdir(output, { recursive: true });
if (phase === 'before' && await access(path.join(output, 'before.json')).then(() => true, () => false)) {
  throw new Error('Baseline before.json already exists; preserve the original evidence.');
}
const vite = spawn(process.execPath, [path.join(path.dirname(require.resolve('vite/package.json')), 'bin/vite.js'), '--configLoader', 'native', '--host', '127.0.0.1', '--port', String(port)], { cwd: root, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
let serverOutput = '';
vite.stdout.on('data', data => { serverOutput += String(data); });
vite.stderr.on('data', data => { serverOutput += String(data); });
let browser;
const errors = [];

function fixture() {
  const skus = Array.from({ length: 1000 }, (_, index) => ({
    id: index + 1, productId: index + 1, name: `HOMOLOGAÇÃO UX PRODUTO ${String(index + 1).padStart(4, '0')}`,
    description: 'Dado sintético exclusivo do teste no navegador', brand: 'HOMOLOGAÇÃO UX', category: index % 2 ? 'Bebidas' : 'Alimentos',
    internalCode: `UX-${String(index + 1).padStart(4, '0')}`, presentation: 'Unidade', purchaseCents: 100,
    saleCents: 200, defaultPack: 1, expiryControl: false, active: true, productActive: true,
    barcodes: [`00${String(index + 1).padStart(10, '0')}`], balance: 5, unidentifiedBalance: 5,
  }));
  const receipt = { receipt: { id: 1, supplierId: null, supplierName: 'HOMOLOGAÇÃO UX', status: 'draft', source: 'manual', invoiceNumber: null, invoiceSeries: null, issuedAt: null, accessKey: null, notes: 'Cenário sintético', revision: 1, createdAt: '2026-11-08T12:00:00Z', totalCents: 100 }, items: [{ id: 1, skuId: 1, supplierCode: null, description: skus[0].name, barcode: skus[0].barcodes[0], taxBarcode: null, quantity: '1', unitsPerPack: 1, units: 1, commercialUnit: 'UN', unitPrice: '1.00', totalCents: 100, unitCostCents: 100, roundingDifferenceCents: 0, taxQuantity: null, taxUnit: null, sourceQuantity: null, sourceUnitPrice: null, sourceTotalCents: null, sourceAdjustmentCents: null, sourceLots: [], lots: [], candidates: [] }] };
  const inventory = { inventory: { id: 9, name: 'HOMOLOGAÇÃO UX', status: 'active', scope: 'partial', revision: 0, createdAt: '2026-11-08T12:00:00Z', closedAt: null, uncountedDecision: null, skuCount: 1 }, lines: [{ id: 1, skuId: 1, skuName: skus[0].name, internalCode: skus[0].internalCode, lotId: null, lotLabel: 'Sem lote identificado', expiresOn: null, reference: 5, counted: null, difference: null }], events: [], eventsTotal: 0 };
  return { data: { skus, suppliers: [], receipts: [receipt.receipt], lots: [], inventories: [inventory.inventory], settings: { shortDays: 7, longDays: 30 }, localDate: '2026-11-08', schemaVersion: 5, sqliteVersion: 'TESTE SINTÉTICO', dataProfile: 'HOMOLOGAÇÃO UX: cliente sintético sem banco' }, receipt, inventory };
}
function entry(reactPath, reactDomPath) {
  return `import React from ${JSON.stringify(reactPath)};
import ReactDOM from ${JSON.stringify(reactDomPath)};
import WorkspaceApp from '/src/WorkspaceApp.tsx';
import { business } from '/src/business-api.ts';
import '/src/styles.css';
const fixture = ${JSON.stringify(fixture())};
window.__UX_CALLS__ = [];
const client = { ...business, available: true, workspace: async()=>structuredClone(fixture.data), receipt: async()=>structuredClone(fixture.receipt), inventory: async()=>structuredClone(fixture.inventory), inventoryEvents: async()=>({items:[],total:0,page:0}), movements: async()=>({items:[],total:0,page:0}), saveProduct: async input=>{window.__UX_CALLS__.push({name:'saveProduct',input});return {...fixture.data.skus[0],...input,id:1001};}, saveReceiptItem: async input=>{window.__UX_CALLS__.push({name:'saveReceiptItem',input});return structuredClone(fixture.receipt);}, scan: async input=>{window.__UX_CALLS__.push({name:'scan',input});return {outcome:'counted',message:'Leitura sintética confirmada',candidates:[1],lotChoices:[],detail:structuredClone(fixture.inventory)};} };
ReactDOM.createRoot(document.getElementById('root')).render(React.createElement(WorkspaceApp,{client}));`;
}
const nav = (page, label) => page.locator('.sidebar nav button').filter({ hasText: new RegExp(`${label}$`) }).click();
async function layout(page, label, width) {
  await page.setViewportSize({ width, height: 900 });
  await page.screenshot({ path: path.join(output, `${phase}-${label}-${width}.png`), fullPage: true });
  return page.evaluate(() => ({ width: innerWidth, documentWidth: document.documentElement.scrollWidth, overflow: document.documentElement.scrollWidth > innerWidth, tableScrollContainers: [...document.querySelectorAll('.table-wrap')].filter(el => el.scrollWidth > el.clientWidth).length }));
}
async function timedSearch(page) {
  return page.evaluate(async () => {
    const input = document.querySelector('input[aria-label="Buscar produtos"]');
    const setValue = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
    const samples = [];
    for (let n = 0; n < 35; n++) {
      const value = n % 2 ? 'UX-1000' : 'UX-0999';
      const start = performance.now();
      setValue.call(input, value);
      input.dispatchEvent(new Event('input', { bubbles: true }));
      await new Promise(resolve => {
        const check = () => {
          const rows = [...document.querySelectorAll('tbody tr')];
          if (rows.length === 1 && rows[0].textContent.includes(value)) requestAnimationFrame(() => requestAnimationFrame(resolve));
          else requestAnimationFrame(check);
        };
        check();
      });
      if (n >= 5) samples.push(performance.now() - start);
    }
    const sorted = [...samples].sort((a, b) => a - b);
    return { skuCount: 1000, warmup: 5, repetitions: samples.length, unit: 'ms', measured: 'Input event to matching row plus two animation frames; development React in headless Chrome, no database or IPC', p50: sorted[Math.ceil(sorted.length * .5) - 1], p95: sorted[Math.ceil(sorted.length * .95) - 1], min: sorted[0], max: sorted.at(-1), samples };
  });
}
try {
  let ready = false;
  for (let attempt = 0; attempt < 100; attempt++) {
    if (vite.exitCode !== null) throw new Error(serverOutput);
    ready = await fetch(baseURL).then(r => r.ok).catch(() => false);
    if (ready) break;
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  assert.ok(ready, 'Vite UX server did not start.');
  browser = await chromium.launch({ channel: process.env.PLAYWRIGHT_CHANNEL || 'chrome', headless: true });
  const page = await browser.newPage({ viewport: { width: 1200, height: 900 } });
  page.on('pageerror', e => errors.push(e.message));
  page.on('console', message => { if (message.type() === 'error') console.error(message.text()); });
  const originalMain = await fetch(`${baseURL}/src/main.tsx`).then(r => r.text());
  const reactPath = originalMain.match(/from\s+["']([^"']*deps\/react\.js[^"']*)["']/)?.[1];
  const reactDomPath = originalMain.match(/from\s+["']([^"']*deps\/react-dom_client\.js[^"']*)["']/)?.[1];
  assert.ok(reactPath && reactDomPath, 'Could not locate optimized React imports in transformed main.tsx.');
  await page.route('**/src/main.tsx', route => route.fulfill({ contentType: 'text/javascript', body: entry(reactPath, reactDomPath) }));
  await page.route('**/*', route => new URL(route.request().url()).origin === baseURL ? route.fallback() : route.abort());
  await page.goto(baseURL);
  await page.getByRole('heading', { name: 'Visão geral' }).waitFor({ timeout: 10000 }).catch(async error => {
    console.error(JSON.stringify({ errors, body: await page.locator('body').innerText(), main: originalMain.slice(0, 1500) }));
    throw error;
  });
  await nav(page, 'Produtos');
  await page.getByRole('heading', { name: 'Produtos', exact: true }).waitFor();
  const layouts = [];
  for (const width of [1200, 760, 320]) layouts.push({ page: 'Produtos', ...await layout(page, 'products', width) });
  if (phase === 'after') {
    const pages = ['Dashboard', 'Recebimentos', 'Importar XML', 'Lotes e Validades', 'Inventário', 'Movimentações', 'Fornecedores', 'Relatórios', 'Configurações'];
    for (const label of pages) {
      await nav(page, label);
      await page.getByRole('heading', { name: label === 'Dashboard' ? 'Visão geral' : label, exact: true }).waitFor();
      for (const width of [1200, 760, 320]) layouts.push({ page: label, ...await layout(page, label.normalize('NFD').replace(/[\u0300-\u036f]/g, '').toLowerCase().replaceAll(' ', '-'), width) });
    }
    await nav(page, 'Produtos');
  }
  await page.setViewportSize({ width: 1200, height: 900 });
  const search = await timedSearch(page);
  await page.getByRole('textbox', { name: 'Buscar produtos' }).fill('UX-0999');
  await nav(page, 'Fornecedores');
  await nav(page, 'Produtos');
  const filterPreserved = await page.getByRole('textbox', { name: 'Buscar produtos' }).inputValue() === 'UX-0999';
  await page.getByRole('button', { name: 'Novo produto', exact: true }).click();
  const productDialog = page.getByRole('dialog');
  const modalFocus = await page.evaluate(() => document.activeElement?.getAttribute('aria-label') || document.activeElement?.labels?.[0]?.textContent || document.activeElement?.closest('label')?.textContent || document.activeElement?.tagName);
  await page.screenshot({ path: path.join(output, `${phase}-product-dialog.png`), fullPage: true });
  let modalKeyboard = null;
  let unsavedProtected = null;
  let barcodeAddedWithoutSave = null;
  if (phase === 'after') {
    await productDialog.getByRole('button', { name: 'Fechar janela' }).focus();
    await page.keyboard.press('Shift+Tab');
    const backwardContained = await productDialog.evaluate(el => el.contains(document.activeElement));
    await page.keyboard.press('Tab');
    const forwardContained = await productDialog.evaluate(el => el.contains(document.activeElement));
    assert.ok(backwardContained && forwardContained, 'Modal should contain keyboard focus in both directions.');
    modalKeyboard = { backwardContained, forwardContained };
    const barcode = fixture().data.skus[0].barcodes[0];
    await productDialog.getByLabel(/Adicionar código de barras/).fill(barcode);
    await productDialog.getByLabel(/Adicionar código de barras/).press('Enter');
    assert.ok((await productDialog.getByLabel(/Códigos de barras/).inputValue()).includes(barcode));
    barcodeAddedWithoutSave = await page.evaluate(() => !window.__UX_CALLS__.some(call => call.name === 'saveProduct'));
    assert.ok(barcodeAddedWithoutSave, 'Barcode Enter should add a code without saving the product.');
    await productDialog.getByLabel('Nome do produto', { exact: true }).fill('HOMOLOGAÇÃO UX: alterações não salvas');
    await page.keyboard.press('Escape');
    const discard = page.getByRole('dialog', { name: 'Descartar alterações?' });
    await discard.waitFor();
    await discard.getByRole('button', { name: 'Continuar editando' }).click();
    assert.equal(await productDialog.getByLabel('Nome do produto', { exact: true }).inputValue(), 'HOMOLOGAÇÃO UX: alterações não salvas');
    unsavedProtected = true;
    await productDialog.getByRole('button', { name: 'Fechar janela' }).click();
    await discard.getByRole('button', { name: 'Descartar alterações', exact: true }).click();
  } else {
    await productDialog.getByRole('button', { name: 'Fechar janela' }).click();
  }
  await nav(page, 'Inventário');
  await page.getByRole('textbox', { name: 'Código de barras ou SKU' }).waitFor();
  const inventoryInitiallyFocused = await page.getByRole('textbox', { name: 'Código de barras ou SKU' }).evaluate(el => document.activeElement === el);
  await page.screenshot({ path: path.join(output, `${phase}-inventory.png`), fullPage: true });
  let quickCreatePreserved = null;
  let pickerEnterWithoutSave = null;
  if (phase === 'after') {
    await nav(page, 'Recebimentos');
    await page.getByRole('button', { name: 'Abrir', exact: true }).click();
    await page.getByRole('button', { name: 'Associar / conferir', exact: true }).click();
    const editor = page.getByRole('dialog', { name: 'Associar e conferir item' });
    await editor.getByLabel(/Buscar produto \/ sku/i).fill(fixture().data.skus[1].barcodes[0]);
    await editor.getByLabel(/Buscar produto \/ sku/i).press('Enter');
    assert.equal(await editor.getByLabel('Produto / SKU', { exact: true }).inputValue(), '2');
    pickerEnterWithoutSave = await page.evaluate(() => !window.__UX_CALLS__.some(call => call.name === 'saveReceiptItem'));
    assert.ok(pickerEnterWithoutSave, 'Picker Enter must not submit the receipt item.');
    await editor.getByLabel('Quantidade comercial (decimal)', { exact: true }).fill('9');
    await editor.getByRole('button', { name: 'Cadastrar produto novo', exact: true }).click();
    await page.getByRole('dialog', { name: 'Cadastrar produto do recebimento' }).getByRole('button', { name: 'Fechar janela' }).click();
    quickCreatePreserved = await editor.getByLabel('Quantidade comercial (decimal)', { exact: true }).inputValue() === '9';
    assert.ok(quickCreatePreserved, 'Quick product creation must preserve unsaved receipt quantities.');
    await page.screenshot({ path: path.join(output, 'after-receipt-draft-preserved.png'), fullPage: true });
    await editor.getByRole('button', { name: 'Fechar janela' }).click();
    await page.getByRole('dialog', { name: 'Descartar alterações?' }).getByRole('button', { name: 'Descartar alterações', exact: true }).click();
  }
  const result = { phase, date: '2026-11-08', browser: await browser.version(), node: process.version, syntheticClient: true, databaseAccessed: false, nativeTauriTested: false, layouts, search, observations: { filterPreserved, inventoryInitiallyFocused, modalFocus, modalKeyboard, unsavedProtected, barcodeAddedWithoutSave, quickCreatePreserved, pickerEnterWithoutSave }, errors, baselineOnly: phase === 'before' };
  if (phase === 'after') {
    assert.ok(filterPreserved, 'Product search should survive module navigation.');
    assert.ok(inventoryInitiallyFocused, 'Inventory reader should receive initial focus.');
    for (const viewport of layouts) assert.equal(viewport.overflow, false, `Document overflow at ${viewport.width}px.`);
    assert.deepEqual(errors, []);
  }
  await writeFile(path.join(output, `${phase}.json`), JSON.stringify(result, null, 2));
  console.log(JSON.stringify({ phase, layouts, search: { p50: search.p50, p95: search.p95 }, observations: result.observations, errors }, null, 2));
} finally {
  await browser?.close();
  vite.kill();
}
