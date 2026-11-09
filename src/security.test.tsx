import {fireEvent,render,screen,waitFor} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {vi} from 'vitest';
import {business,type Workspace,type ReceiptDetail,type Sku} from './business-api';
import Products from './Products';
import {ImportXml,ReceiptView} from './Receipts';
import {TaskStatus,type PageProps} from './ui';
import {readXmlFile,MAX_XML_FILE_BYTES} from './xml-file';

const payload='<img src=x onerror="globalThis.__astock_xss=1"><script>globalThis.__astock_xss=1</script>javascript:alert(1)';
const sku:Sku={id:1,productId:1,name:payload,description:payload,brand:'Teste',category:'Teste',internalCode:'XSS-TEST',presentation:'UN',purchaseCents:100,saleCents:200,defaultPack:1,expiryControl:false,active:true,productActive:true,barcodes:['001234'],balance:0,unidentifiedBalance:0};
function data():Workspace{return{skus:[sku],suppliers:[],receipts:[],lots:[],inventories:[],settings:{shortDays:7,longDays:30},localDate:'2026-11-08',schemaVersion:5,sqliteVersion:'3.54.1'};}
function detail():ReceiptDetail{return{receipt:{id:1,supplierId:null,supplierName:payload,status:'draft',source:'xml',invoiceNumber:'1',invoiceSeries:'1',issuedAt:'2026-11-08T12:00:00-03:00',accessKey:'1'.repeat(44),notes:payload,revision:0,createdAt:'2026-11-08T15:00:00Z',totalCents:100},items:[{id:1,skuId:null,supplierCode:'XSS',description:payload,barcode:'001234',taxBarcode:null,quantity:'1',unitsPerPack:1,units:1,commercialUnit:'UN',unitPrice:'1.00',totalCents:100,unitCostCents:100,roundingDifferenceCents:0,taxQuantity:null,taxUnit:null,sourceQuantity:'1',sourceUnitPrice:'1',sourceTotalCents:100,sourceAdjustmentCents:0,sourceLots:[],lots:[],candidates:[]}]};}
function props():PageProps{return{data:data(),client:{...business,available:true,receipt:vi.fn().mockResolvedValue(detail()),importXml:vi.fn().mockResolvedValue({detail:detail(),warnings:[payload],reused:false})},refresh:vi.fn().mockResolvedValue(undefined),navigate:vi.fn()};}
function inert(container:HTMLElement){expect(container.querySelector('img,script,iframe,object,a[href^="javascript:"]')).toBeNull();expect((globalThis as typeof globalThis&{__astock_xss?:number}).__astock_xss).toBeUndefined();}

test('nome externo e busca renderizam tags e URLs como texto',()=>{const p=props();const {container}=render(<Products {...p}/>);expect(screen.getByText(payload)).toBeVisible();const search=screen.getByLabelText('Buscar produtos');fireEvent.change(search,{target:{value:'XSS-TEST'}});expect(screen.getByText(payload)).toBeVisible();inert(container);});
test('observações, fornecedor e itens externos não se tornam HTML',async()=>{const p=props();const {container}=render(<ReceiptView {...p} id={1} initial={detail()}/>);await waitFor(()=>expect(screen.getAllByText(payload).length).toBeGreaterThan(1));inert(container);});
test('notificações e erros externos permanecem texto',()=>{const task={error:payload,message:payload,busy:false} as Parameters<typeof TaskStatus>[0]['task'];const {container}=render(<TaskStatus task={task}/>);expect(screen.getByRole('alert')).toHaveTextContent(payload);expect(screen.getByRole('status')).toHaveTextContent(payload);inert(container);});
test('conteúdo malicioso importado de XML aparece como texto inerte',async()=>{const p=props();const {container}=render(<ImportXml {...p}/>);await userEvent.upload(screen.getByLabelText('Importar XML'),new File(['<NFe>&lt;script&gt;</NFe>'],'synthetic.xml',{type:'application/xml'}));await screen.findByText('synthetic.xml');await waitFor(()=>expect(p.client.importXml).toHaveBeenCalled());expect(screen.getAllByText(payload).length).toBeGreaterThan(0);inert(container);});
test('bytes UTF-8 inválidos são rejeitados antes de invocar importação',async()=>{const p=props();render(<ImportXml {...p}/>);await userEvent.upload(screen.getByLabelText('Importar XML'),new File([new Uint8Array([0x3c,0x78,0x3e,0xc3,0x28])],'invalid.xml',{type:'application/xml'}));expect(await screen.findByRole('alert')).toHaveTextContent('bytes inválidos em UTF-8');expect(p.client.importXml).not.toHaveBeenCalled();});
test('leitura estrita aceita UTF-8 com acentos e rejeita tamanho excessivo',async()=>{expect(await readXmlFile(new File(['<x>Água</x>'],'valid.xml'))).toBe('<x>Água</x>');await expect(readXmlFile(new File([new Uint8Array(MAX_XML_FILE_BYTES+1)],'large.xml'))).rejects.toThrow('8 MiB');});
