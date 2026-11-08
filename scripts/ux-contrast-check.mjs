// Contraste calculado dos tokens compartilhados, não certificação WCAG completa.
import assert from 'node:assert/strict';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const css=await readFile(path.join(root,'src/styles.css'),'utf8');
const tokens=Object.fromEntries([...css.matchAll(/(--axis-[\w-]+):\s*(#[\da-f]{6});/gi)].map(match=>[match[1],match[2]]));
function luminance(hex) {
 const channels=hex.slice(1).match(/../g).map(channel=>parseInt(channel,16)/255).map(value=>value<=.04045?value/12.92:((value+.055)/1.055)**2.4);
 return channels[0]*.2126+channels[1]*.7152+channels[2]*.0722;
}
function contrast(first,second) { const a=luminance(first),b=luminance(second);return (Math.max(a,b)+.05)/(Math.min(a,b)+.05); }
const pairs=[
 ['Texto principal','--axis-text','--axis-surface',4.5],
 ['Texto secundário','--axis-text-muted','--axis-surface',4.5],
 ['Cabeçalho de tabela','--axis-text-muted','--axis-surface-muted',4.5],
 ['Ação principal','--axis-on-primary','--axis-primary',4.5],
 ['Navegação','--axis-brand-muted','--axis-brand',4.5],
 ['Erro','--axis-danger-text','--axis-danger-bg',4.5],
 ['Aviso','--axis-warning-text','--axis-warning-bg',4.5],
 ['Sucesso','--axis-success-text','--axis-success-bg',4.5],
 ['Limite de controle','--axis-control-border','--axis-surface',3],
 ['Foco em painel','--axis-focus','--axis-surface',3],
 ['Foco em navegação','--axis-focus','--axis-brand',3],
];
const results=pairs.map(([name,foreground,background,minimum])=>{
 assert.ok(tokens[foreground]&&tokens[background],`Token ausente: ${foreground}/${background}`);
 const ratio=contrast(tokens[foreground],tokens[background]);
 assert.ok(ratio>=minimum,`${name}: ${ratio.toFixed(2)} abaixo de ${minimum}`);
 return {name,foreground:tokens[foreground],background:tokens[background],ratio:Number(ratio.toFixed(2)),minimum,passed:true};
});
const evidence={date:'2026-11-08',method:'Luminância relativa sRGB dos valores hex dos tokens; elementos opacos e estados ativos. Não inclui toda combinação, texto sobre imagens, controles nativos ou certificação WCAG.',results,beforeControlBorder:{foreground:'#94a3b8',background:'#ffffff',ratio:Number(contrast('#94a3b8','#ffffff').toFixed(2))}};
const output=path.join(root,'docs/ux-ui/evidencias');await mkdir(output,{recursive:true});
await writeFile(path.join(output,'contrast.json'),JSON.stringify(evidence,null,2)+'\n');
console.log(`${results.length} pares de contraste aprovados; borda de controle: ${evidence.beforeControlBorder.ratio} → ${results.find(result=>result.name==='Limite de controle').ratio}.`);
