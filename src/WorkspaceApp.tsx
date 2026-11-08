import { useCallback,useEffect,useState } from 'react';
import { business,type BusinessClient,type Workspace } from './business-api';
import { errorMessage } from './api';
import { Button,type Page,type PageProps } from './ui';
import Products from './Products';
import Suppliers from './Suppliers';
import Receipts,{ ImportXml } from './Receipts';
import Inventory from './Inventory';
import { Dashboard,Lots,Movements,Reports,SettingsView } from './Views';

const pages:Page[]=['Dashboard','Produtos','Recebimentos','Importar XML','Lotes e Validades','Inventário','Movimentações','Fornecedores','Relatórios','Configurações'];
export default function WorkspaceApp({client=business}:{client?:BusinessClient}){
 const[data,setData]=useState<Workspace|null>(null);const[page,setPage]=useState<Page>('Dashboard');const[error,setError]=useState('');const[refreshing,setRefreshing]=useState(false);
 const refresh=useCallback(async()=>{const next=await client.workspace();setData(next);setError('');},[client]);
 useEffect(()=>{
  let stopped=false;let timer:ReturnType<typeof setTimeout>|undefined;
  async function load(){if(!client.available)return;try{const next=await client.workspace();if(!stopped){setData(next);setError('');}}catch(e){if(stopped)return;if(typeof e==='object'&&e!==null&&'code'in e&&e.code==='STARTING'){timer=setTimeout(()=>void load(),400);}else setError(errorMessage(e));}}
  void load();return()=>{stopped=true;if(timer)clearTimeout(timer);};
 },[client]);
 const props:PageProps|undefined=data?{data,client,refresh,navigate:setPage}:undefined;
 const active=data?.inventories.find(i=>i.status==='active'||i.status==='closing');
 let content=null;
 if(props){switch(page){case'Dashboard':content=<Dashboard {...props}/>;break;case'Produtos':content=<Products {...props}/>;break;case'Recebimentos':content=<Receipts {...props}/>;break;case'Importar XML':content=<ImportXml {...props}/>;break;case'Lotes e Validades':content=<Lots {...props}/>;break;case'Inventário':content=<Inventory {...props}/>;break;case'Movimentações':content=<Movements {...props}/>;break;case'Fornecedores':content=<Suppliers {...props}/>;break;case'Relatórios':content=<Reports {...props}/>;break;case'Configurações':content=<SettingsView {...props}/>;break;}}
 return <div className="app-shell"><a className="skip-link" href="#conteudo-principal">Ir para o conteúdo</a><aside className="sidebar"><div className="app-brand"><span className="brand-mark">A</span><div><strong>Astock</strong><small>Controle local de mercadorias</small></div></div><nav aria-label="Navegação principal">{pages.map((p,index)=><button key={p} className={page===p?'nav-active':''} onClick={()=>setPage(p)} aria-current={page===p?'page':undefined}><span className="nav-number">{String(index+1).padStart(2,'0')}</span>{p}</button>)}</nav><div className="sidebar-foot"><span className="local-dot"/>Operação local · offline<small>Versão 1.3.0 · Dados neste computador</small></div></aside><div className="app-main"><header className="app-topbar"><span>{page}</span><div className="actions">{active&&<span className="badge badge-active">Inventário #{active.id} ativo</span>}<Button disabled={!data||refreshing} onClick={()=>{setRefreshing(true);void refresh().catch(e=>setError(errorMessage(e))).finally(()=>setRefreshing(false));}}>{refreshing?'Atualizando…':'Atualizar dados'}</Button></div></header><main id="conteudo-principal" tabIndex={-1} className="workspace-content">{data?.dataProfile&&<div className="notice"><strong>Homologação: {data.dataProfile}</strong> · Banco separado para dados de teste. O banco principal não é usado nesta janela.</div>}{!client.available?<section className="panel"><h1>Abra o Astock no Tauri</h1><p className="muted">Esta pré-visualização não acessa o banco. Os cadastros, XMLs, lotes e inventários funcionam na janela nativa do aplicativo.</p></section>:<>{error&&<div role="alert" className="notice notice-error">{error}<Button onClick={()=>void refresh().catch(e=>setError(errorMessage(e)))}>Tentar novamente</Button></div>}{!data&&!error&&<div role="status" className="panel"><h1>Preparando o banco local…</h1><p>Verificando integridade, backups e migrações. Aguarde a conclusão sem fechar o aplicativo.</p></div>}{data&&active&&page!=='Inventário'&&<div className="inventory-lock-banner">Inventário #{active.id} em andamento: os SKUs incluídos estão bloqueados para movimentação. <button onClick={()=>setPage('Inventário')}>Retomar contagem</button></div>}{content}</>}</main><footer className="workspace-footer">Astock · {data?`SQLite ${data.sqliteVersion} · Esquema ${data.schemaVersion}`:'Banco local'} · Movimentações confirmadas são preservadas no histórico.</footer></div></div>;
}
