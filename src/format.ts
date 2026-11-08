export const money=(value:number|bigint|null)=>{if(value===null)return 'Pendente';const v=BigInt(value),absolute=v<0n?-v:v;return `${v<0n?'-':''}R$ ${(absolute/100n).toLocaleString('pt-BR')},${String(absolute%100n).padStart(2,'0')}`;};
export const day=(value:string|null)=>value?value.slice(0,10).split('-').reverse().join('/'):'Sem validade';
export const dateTime=(value:string)=>new Date(value).toLocaleString('pt-BR');
export function moneyInput(value:number):string{const v=BigInt(value);return `${v/100n},${String(v%100n).padStart(2,'0')}`;}
export function parseMoney(value:string):number{
 const raw=value.trim();if(raw.length>30)throw new Error('Valor acima do limite permitido.');
 const grouped=/^\d{1,3}(\.\d{3})+(,\d{1,2})?$/.test(raw);
 const normalized=grouped?raw.replace(/\./g,'').replace(',','.'):raw.replace(',','.');
 if(!/^\d+(\.\d{1,2})?$/.test(normalized))throw new Error('Informe o valor em reais, com até duas casas decimais.');
 const[whole,fraction='']=normalized.split('.');const cents=BigInt(whole)*100n+BigInt(fraction.padEnd(2,'0'));
 if(cents>9_000_000_000_000n)throw new Error('Valor acima do limite permitido.');return Number(cents);
}
export function integer(value:string,positive=false):number{
 if(!/^-?\d+$/.test(value.trim()))throw new Error('Informe uma quantidade inteira.');const number=Number(value);
 if(!Number.isSafeInteger(number)||Math.abs(number)>1_000_000_000||(positive&&number<=0))throw new Error('Quantidade fora do limite permitido.');return number;
}
export function convertedUnits(value:string,pack:number):number|null{
 if(value.length>32||!/^\d+(\.\d{0,10})?$/.test(value)||!Number.isInteger(pack)||pack<1)return null;
 const[whole,fraction='']=value.split('.');const numerator=BigInt(whole+fraction)*BigInt(pack);const denominator=10n**BigInt(fraction.length);
 if(numerator%denominator!==0n||numerator/denominator<=0n||numerator/denominator>1_000_000_000n)return null;return Number(numerator/denominator);
}
export const requestKey=()=>crypto.randomUUID();
export function decimalProductCents(quantity:string,price:string):number{
 const parts=[quantity,price].map(raw=>{const value=raw.trim().replace(',','.');if(value.length>32||!/^\d+(\.\d{0,10})?$/.test(value))throw new Error('Informe valores decimais válidos, com até dez casas.');const[w,f='']=value.split('.');return{n:BigInt(w+f),scale:f.length};});
 const numerator=parts[0].n*parts[1].n*100n;const divisor=10n**BigInt(parts[0].scale+parts[1].scale);const result=(numerator+divisor/2n)/divisor;if(result>9_000_000_000_000n)throw new Error('Total acima do limite permitido.');return Number(result);
}
export const statusName:Record<string,string>={draft:'Rascunho',confirmed:'Confirmado',canceled:'Cancelado',active:'Em andamento',closing:'Finalizando',finalized:'Finalizado',receipt:'Recebimento',issue:'Saída',adjustment:'Ajuste',inventory:'Inventário',reclassify:'Regularização de lote',expired:'Vencido',short:'Primeiro alerta',long:'Segundo alerta',safe:'Dentro do prazo',undated:'Sem validade',unknown:'Não associado',ambiguous:'Código ambíguo',lot_required:'Escolha de lote',counted:'Contado',corrected:'Correção'};
