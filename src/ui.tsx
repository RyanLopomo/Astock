import { cloneElement, createContext, isValidElement, useContext, useEffect, useEffectEvent, useId, useRef, useState, type ButtonHTMLAttributes, type InputHTMLAttributes, type ReactNode } from 'react';
import { matches, useTask } from './task';
import type { BusinessClient, Sku, Workspace } from './business-api';
import { statusName } from './format';

export type Page = 'Dashboard'|'Produtos'|'Recebimentos'|'Importar XML'|'Lotes e Validades'|'Inventário'|'Movimentações'|'Fornecedores'|'Relatórios'|'Configurações';
export interface PageProps { data:Workspace; client:BusinessClient; refresh:()=>Promise<void>; navigate:(page:Page)=>void }
export function TaskStatus({task}:{task:ReturnType<typeof useTask>}) { return <>{task.error&&<div role="alert" className="notice notice-error">{task.error}</div>}{task.message&&<div role="status" className="success-notice">{task.message}</div>}</>; }
export function Button({children,className='',...props}:ButtonHTMLAttributes<HTMLButtonElement>) { return <button type="button" {...props} className={`button-secondary ${className}`}>{children}</button>; }
export function Primary(props:ButtonHTMLAttributes<HTMLButtonElement>) { return <Button {...props} className={`primary ${props.className??''}`}/>; }
export function Heading({title,description,actions}:{title:string;description:string;actions?:ReactNode}) { return <div className="page-heading"><div><p className="eyebrow">Astock · LOCAL</p><h1>{title}</h1><p>{description}</p></div><div className="actions">{actions}</div></div>; }

export function Field({label,children,hint,error}:{label:string;children:ReactNode;hint?:string;error?:string}) {
 const generatedId=useId();
 const control=isValidElement<InputHTMLAttributes<HTMLInputElement>>(children)&&typeof children.type==='string'&&['input','select','textarea'].includes(children.type)?children:null;
 const id=control?.props.id??generatedId;
 const describedBy=[control?.props['aria-describedby'],hint?`${id}-hint`:null,error?`${id}-error`:null].filter(Boolean).join(' ')||undefined;
 return <div className="field"><div className="field-label"><label htmlFor={id}>{label}</label>{control?.props.required&&<span className="required-marker" aria-hidden="true"> *</span>}</div>{control?cloneElement(control,{id,'aria-describedby':describedBy,'aria-invalid':error?true:control.props['aria-invalid']}):children}{hint&&<small id={`${id}-hint`}>{hint}</small>}{error&&<small id={`${id}-error`} className="field-error">{error}</small>}</div>;
}
export function Check({label,checked,onChange,disabled=false}:{label:string;checked:boolean;onChange:(value:boolean)=>void;disabled?:boolean}) { return <label className="check"><input type="checkbox" checked={checked} onChange={e=>onChange(e.target.checked)} disabled={disabled}/><span>{label}</span></label>; }
export function Empty({children}:{children:ReactNode}) { return <div className="empty-state">{children}</div>; }
export function Badge({value}:{value:string}) { return <span className={`badge badge-${value}`}>{statusName[value]??value}</span>; }

const ModalCloseContext=createContext<(()=>void)|null>(null);
export function ModalCancel(props:Omit<ButtonHTMLAttributes<HTMLButtonElement>,'onClick'>) {
 const close=useContext(ModalCloseContext);
 return <Button {...props} onClick={()=>close?.()}/>;
}

export function Modal({title,children,onClose,busy=false,wide=false,dirty=false,suspended=false}:{title:string;children:ReactNode;onClose:()=>void;busy?:boolean;wide?:boolean;dirty?:boolean;suspended?:boolean}) {
 const dialog=useRef<HTMLDivElement>(null);
 const continueButton=useRef<HTMLButtonElement>(null);
 const previousControl=useRef<HTMLElement|null>(null);
 const [discarding,setDiscarding]=useState(false);
 function requestClose() {
  if(busy||suspended)return;
  if(dirty){previousControl.current=document.activeElement as HTMLElement|null;setDiscarding(true);}
  else onClose();
 }
 const escape=useEffectEvent(()=>{
  if(discarding){setDiscarding(false);return;}
  requestClose();
 });
 useEffect(()=>{
  if(suspended)return;
  const previous=document.activeElement as HTMLElement|null;
  const controls=()=>Array.from(dialog.current?.querySelectorAll<HTMLElement>('button,input,select,textarea,a[href],[tabindex]')??[]).filter(el=>!el.matches(':disabled,[tabindex="-1"],input[type="hidden"]')&&!el.closest('[hidden],[inert]'));
  const first=controls().find(el=>el.closest('.modal-body'))??dialog.current;
  first?.focus();
  function key(event:KeyboardEvent) {
   if(event.key==='Escape'){event.preventDefault();event.stopPropagation();escape();}
   if(event.key==='Tab'){
    const nodes=controls();const first=nodes[0],last=nodes[nodes.length-1];
    if(!first){event.preventDefault();dialog.current?.focus();return;}
    if(!dialog.current?.contains(document.activeElement)){event.preventDefault();first.focus();}
    else if(event.shiftKey&&(document.activeElement===first||document.activeElement===dialog.current)){event.preventDefault();last.focus();}
    else if(!event.shiftKey&&(document.activeElement===last||document.activeElement===dialog.current)){event.preventDefault();first.focus();}
   }
  }
  document.addEventListener('keydown',key);
  return()=>{document.removeEventListener('keydown',key);if(previous?.isConnected&&!previous.closest('[hidden],[inert]'))previous.focus();};
 },[suspended]);
 useEffect(()=>{
  if(discarding)continueButton.current?.focus();
  else if(previousControl.current?.isConnected&&!suspended){previousControl.current.focus();previousControl.current=null;}
 },[discarding,suspended]);
 useEffect(()=>{
  if(!dirty)return;
  const guard=(event:BeforeUnloadEvent)=>{event.preventDefault();event.returnValue='';};
  window.addEventListener('beforeunload',guard);
  return()=>window.removeEventListener('beforeunload',guard);
 },[dirty]);
 return <div className="modal-shade" hidden={suspended} inert={suspended}><div ref={dialog} role="dialog" aria-modal="true" aria-label={discarding?'Descartar alterações?':title} aria-busy={busy} tabIndex={-1} className={`modal-card ${wide?'modal-wide':''}`}><div className="modal-title"><h2>{discarding?'Descartar alterações?':title}</h2><Button onClick={requestClose} disabled={busy||discarding} aria-label="Fechar janela">×</Button></div><ModalCloseContext.Provider value={requestClose}><fieldset disabled={busy||discarding} hidden={discarding} className="modal-body">{children}</fieldset></ModalCloseContext.Provider>{discarding&&<div className="confirm-content"><p>Há alterações que ainda não foram salvas. Deseja descartá-las?</p><div className="modal-actions"><button ref={continueButton} type="button" className="button-secondary primary" onClick={()=>setDiscarding(false)}>Continuar editando</button><Button className="danger" onClick={onClose}>Descartar alterações</Button></div></div>}</div></div>;
}
export function Confirm({title,children,onCancel,onConfirm,busy=false,label='Confirmar'}:{title:string;children:ReactNode;onCancel:()=>void;onConfirm:()=>void;busy?:boolean;label?:string}) { return <Modal title={title} onClose={onCancel} busy={busy}><div className="confirm-content">{children}</div><div className="modal-actions"><Button onClick={onCancel} disabled={busy}>Voltar à conferência</Button><Primary onClick={onConfirm} disabled={busy}>{busy?'Aplicando…':label}</Primary></div></Modal>; }
export function Pager({page,total,size=20,onChange}:{page:number;total:number;size?:number;onChange:(page:number)=>void}) { const pages=Math.max(1,Math.ceil(total/size));return <div className="pager"><span>{total} registro(s) · Página {page+1} de {pages}</span><div className="actions"><Button disabled={page<=0} onClick={()=>onChange(page-1)}>Anterior</Button><Button disabled={page+1>=pages} onClick={()=>onChange(page+1)}>Próxima</Button></div></div>; }
export function Table({headers,children}:{headers:string[];children:ReactNode}) { return <div className="table-wrap" tabIndex={0} role="region" aria-label={`Tabela: ${headers.join(', ')}`}><table><thead><tr>{headers.map(h=><th scope="col" key={h}>{h}</th>)}</tr></thead><tbody>{children}</tbody></table></div>; }

export function SkuPicker({skus,value,onChange,label='Produto / SKU',required=true}:{skus:Sku[];value:number|null;onChange:(value:number|null)=>void;label?:string;required?:boolean}) {
 const [search,setSearch]=useState('');
 const [feedback,setFeedback]=useState('');
 const rows=skus.filter(s=>matches(`${s.name} ${s.internalCode} ${s.brand} ${s.barcodes.join(' ')}`,search)||s.id===value);
 function identify() {
  const code=search.trim();if(!code)return;
  const exact=skus.filter(s=>s.internalCode===code||s.barcodes.includes(code));
  if(exact.length===1){onChange(exact[0].id);setFeedback(`Selecionado: ${exact[0].internalCode} · ${exact[0].name}. Confira os dados antes de salvar.`);}
  else {onChange(null);setFeedback(exact.length>1?'Código compartilhado por várias variantes. Escolha o produto na lista.':'Nenhum código exato encontrado. Pesquise pelo nome ou cadastre o produto.');}
 }
 return <div className="sku-picker"><Field label={`Buscar ${label.toLowerCase()}`} hint="Bipe o código e pressione Enter para selecionar. Códigos compartilhados exigem escolha na lista."><input value={search} onChange={e=>{setSearch(e.target.value);setFeedback('');}} onKeyDown={e=>{if(e.key==='Enter'){e.preventDefault();identify();}}} autoComplete="off" spellCheck={false} placeholder="Nome, código interno ou código de barras"/></Field><Field label={label}><select value={value??''} onChange={e=>{onChange(e.target.value?Number(e.target.value):null);setFeedback('');}} required={required}><option value="">Selecione…</option>{rows.map(s=><option value={s.id} key={s.id}>{s.internalCode} · {s.name} · {s.presentation}{!s.active||!s.productActive?' (inativo)':''}</option>)}</select></Field><p role="status" className="picker-feedback">{feedback}</p></div>;
}
