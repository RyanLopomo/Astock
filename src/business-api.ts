import { invoke, isTauri } from '@tauri-apps/api/core';
export interface ProductInput { productId:number|null; skuId:number|null; name:string; description:string; brand:string; category:string; internalCode:string; presentation:string; purchaseCents:number; saleCents:number; defaultPack:number; expiryControl:boolean; active:boolean; productActive:boolean; barcodes:string[] }
export interface Sku extends Omit<ProductInput,'productId'|'skuId'> {id:number;productId:number;balance:number;unidentifiedBalance:number}
export interface Supplier {id:number;name:string;taxId:string;contact:string;active:boolean}
export interface LotDraft {label:string;manufacturedOn:string|null;expiresOn:string|null;vintage:number|null;quantity:number}
export interface XmlLot {label:string;manufacturedOn:string|null;expiresOn:string|null;quantity:string}
export interface ReceiptItem {id:number;skuId:number|null;supplierCode:string|null;description:string;barcode:string|null;taxBarcode:string|null;quantity:string;unitsPerPack:number;units:number|null;commercialUnit:string;unitPrice:string;totalCents:number;unitCostCents:number|null;roundingDifferenceCents:number|null;taxQuantity:string|null;taxUnit:string|null;sourceQuantity:string|null;sourceUnitPrice:string|null;sourceTotalCents:number|null;sourceAdjustmentCents:number|null;sourceLots:XmlLot[];lots:LotDraft[];candidates:number[]}
export interface ReceiptSummary {id:number;supplierId:number|null;supplierName:string;status:'draft'|'confirmed'|'canceled';source:'manual'|'xml';invoiceNumber:string|null;invoiceSeries:string|null;issuedAt:string|null;accessKey:string|null;notes:string;revision:number;createdAt:string;totalCents:number}
export interface ReceiptDetail {receipt:ReceiptSummary;items:ReceiptItem[]}
export interface ReceiptItemInput {receiptId:number;expectedRevision:number;itemId:number|null;skuId:number|null;quantity:string;unitsPerPack:number;commercialUnit:string;unitPrice:string;totalCents:number;lots:LotDraft[];rememberMapping:boolean}
export interface ReceiptAction {id:number;expectedRevision:number;confirmed:boolean}
export interface Confirmation {id:number;alreadyCompleted:boolean}
export interface XmlImportResult {detail:ReceiptDetail;reused:boolean;warnings:string[]}
export interface Lot {id:number;skuId:number;skuName:string;internalCode:string;category:string;label:string;manufacturedOn:string|null;expiresOn:string|null;vintage:number|null;supplierId:number|null;supplierName:string;receiptId:number|null;receivedQuantity:number;balance:number;unitCostCents:number;alert:'expired'|'short'|'long'|'safe'|'undated'}
export interface LotQuantity {lotId:number|null;quantity:number}
export interface IssuePlan {skuId:number;quantity:number;allocations:LotQuantity[];warnings:string[]}
export interface Movement {id:number;skuId:number;internalCode:string;skuName:string;lotId:number|null;lotLabel:string;quantity:number;kind:string;originType:string;originId:string;reason:string;unitCostCents:number;createdAt:string}
export interface MovementQuery {search:string;skuId:number|null;fromDate:string|null;toDate:string|null;page:number}
export interface MovementPage {items:Movement[];total:number;page:number}
export interface Settings {shortDays:number;longDays:number}
export interface InventorySummary {id:number;name:string;status:'active'|'closing'|'finalized'|'canceled';scope:'full'|'partial';revision:number;createdAt:string;closedAt:string|null;uncountedDecision:'skip'|'zero'|null;skuCount:number}
export interface InventoryLine {id:number;skuId:number;skuName:string;internalCode:string;lotId:number|null;lotLabel:string;expiresOn:string|null;reference:number;counted:number|null;difference:number|null}
export interface CountEvent {id:number;code:string;skuId:number|null;lotId:number|null;quantity:number;mode:string;outcome:string;createdAt:string;previousCount:number|null;newCount:number|null}
export interface CountEventPage {items:CountEvent[];total:number;page:number}
export interface InventoryDetail {inventory:InventorySummary;lines:InventoryLine[];events:CountEvent[];eventsTotal:number}
export interface InventoryScan {inventoryId:number;requestKey:string;code:string;quantity:number;skuId:number|null;lotId:number|null;lotSelected:boolean}
export interface ScanResult {outcome:string;message:string;candidates:number[];lotChoices:InventoryLine[];detail:InventoryDetail}
export interface Workspace {skus:Sku[];suppliers:Supplier[];receipts:ReceiptSummary[];lots:Lot[];inventories:InventorySummary[];settings:Settings;localDate:string;schemaVersion:number;sqliteVersion:string;dataProfile?:string|null;databasePath?:string}
function input<T>(command:string,value:unknown):Promise<T>{return invoke<T>(command,{input:value});}
export const business={
 available:isTauri(),workspace:()=>invoke<Workspace>('get_workspace'),
 saveProduct:(value:ProductInput)=>input<Sku>('save_product',value),
 saveSupplier:(value:Omit<Supplier,'id'>&{id:number|null})=>input<Supplier>('save_supplier',value),
 createReceipt:(value:{supplierId:number|null;notes:string})=>input<ReceiptDetail>('create_receipt',value),
 receipt:(id:number)=>invoke<ReceiptDetail>('get_receipt',{id}),
 saveReceiptItem:(value:ReceiptItemInput)=>input<ReceiptDetail>('save_receipt_item',value),
 deleteReceiptItem:(value:{receiptId:number;itemId:number;expectedRevision:number})=>input<ReceiptDetail>('delete_receipt_item',value),
 confirmReceipt:(value:ReceiptAction)=>input<Confirmation>('confirm_receipt',value),cancelReceipt:(value:ReceiptAction)=>input<void>('cancel_receipt',value),
 importXml:(content:string,fileName:string)=>input<XmlImportResult>('import_xml',{content,fileName}),
 planIssue:(value:{skuId:number;quantity:number;includeUndated:boolean;allowExpired:boolean})=>input<IssuePlan>('plan_issue',value),
 issue:(value:{requestKey:string;skuId:number;allocations:LotQuantity[];reason:string;allowExpired:boolean;confirmed:boolean})=>input<void>('issue_stock',value),
 adjust:(value:{requestKey:string;skuId:number;lotId:number|null;quantity:number;reason:string;reference:string;confirmed:boolean})=>input<void>('adjust_stock',value),
 regularize:(value:{requestKey:string;skuId:number;lot:LotDraft;reason:string;confirmed:boolean})=>input<void>('regularize_lot',value),
 createInventory:(value:{name:string;skuIds:number[]|null})=>input<InventoryDetail>('create_inventory',value),inventory:(id:number)=>invoke<InventoryDetail>('get_inventory',{id}),
 inventoryEvents:(id:number,page:number)=>invoke<CountEventPage>('get_inventory_events',{id,page}),scan:(value:InventoryScan)=>input<ScanResult>('inventory_scan',value),
 correct:(value:{inventoryId:number;lineId:number;requestKey:string;quantity:number;expectedRevision:number})=>input<InventoryDetail>('correct_count',value),
 finishInventory:(value:{id:number;expectedRevision:number;uncountedDecision:'skip'|'zero'|null;confirmed:boolean})=>input<Confirmation>('finish_inventory',value),cancelInventory:(id:number)=>input<void>('cancel_inventory',{id,confirmed:true}),
 movements:(value:MovementQuery)=>input<MovementPage>('get_movements',value),saveSettings:(value:Settings)=>input<Settings>('save_settings',value),backup:()=>invoke<string>('create_backup'),
};
export type BusinessClient=typeof business;
