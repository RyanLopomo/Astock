import { useCallback, useEffect, useRef, useState, type FormEvent } from 'react';
import { desktopClient, errorMessage, type ProbeClient, type ProbeState } from './api';

export default function App({ client = desktopClient }: { client?: ProbeClient }) {
  const [state, setState] = useState<ProbeState | null>(null);
  const [code, setCode] = useState('');
  const [note, setNote] = useState('');
  const [loading, setLoading] = useState(client.available);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');
  const [feedback, setFeedback] = useState('');
  const input = useRef<HTMLInputElement>(null);
  const inFlight = useRef(false);
  const alive = useRef(true);

  const refresh = useCallback(async () => {
    if (!client.available || inFlight.current) return;
    setLoading(true);
    setError('');
    try {
      const result = await client.getState();
      if (alive.current) setState(result);
    } catch (e) {
      if (alive.current) setError(errorMessage(e));
    } finally {
      if (alive.current) setLoading(false);
    }
  }, [client]);

  useEffect(() => {
    alive.current = true;
    let cancelled = false;
    if (client.available) {
      void client.getState()
        .then((result) => { if (!cancelled) setState(result); })
        .catch((e: unknown) => { if (!cancelled) setError(errorMessage(e)); })
        .finally(() => { if (!cancelled) setLoading(false); });
    }
    input.current?.focus();
    return () => { cancelled = true; alive.current = false; };
  }, [client]);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!client.available || !state || inFlight.current || loading) return;
    const cleanCode = code.trim();
    if (!cleanCode || Array.from(cleanCode).length > 128 || /[\p{Cc}]/u.test(cleanCode)) {
      setError('Informe um código de 1 a 128 caracteres, sem caracteres de controle.');
      input.current?.focus();
      return;
    }
    if (Array.from(note.trim()).length > 240 || /[\p{Cc}]/u.test(note.trim())) {
      setError('A observação aceita até 240 caracteres, sem caracteres de controle.');
      return;
    }
    inFlight.current = true;
    setSaving(true);
    setError('');
    setFeedback('');
    try {
      const entry = await client.record({ code: cleanCode, note: note.trim() });
      if (!alive.current) return;
      // A resposta contém o registro relido no SQLite antes do commit.
      setState((previous) => previous ? {
        ...previous,
        database: { ...previous.database, totalEntries: previous.database.totalEntries + 1 },
        entries: [entry, ...previous.entries].slice(0, 20),
      } : previous);
      setCode('');
      setFeedback(`Código ${entry.code} gravado e relido no banco local.`);
    } catch (e) {
      if (alive.current) setError(errorMessage(e));
    } finally {
      inFlight.current = false;
      if (alive.current) {
        setSaving(false);
        input.current?.focus();
        input.current?.select();
      }
    }
  }

  const db = state?.database;
  return (
    <div className="min-h-screen bg-stone-50 text-slate-900">
      <header className="border-b border-emerald-950/10 bg-white">
        <div className="mx-auto flex max-w-6xl items-center justify-between gap-4 px-6 py-5">
          <div className="flex items-center gap-3">
            <span className="brand-mark" aria-hidden="true">A</span>
            <div><p className="font-bold tracking-widest">Astock</p><p className="text-xs text-slate-600">Validação técnica · Fase 0</p></div>
          </div>
          <span className="rounded-full bg-amber-50 px-3 py-1.5 text-xs font-semibold text-amber-900">Ambiente de prova</span>
        </div>
      </header>

      <main className="mx-auto max-w-6xl space-y-6 px-6 py-8">
        <div className="flex flex-wrap items-end justify-between gap-4">
          <div><p className="eyebrow">BASE LOCAL · LEITOR USB</p><h1 className="mt-2 text-3xl font-semibold tracking-tight">Vamos conferir a base.</h1><p className="mt-2 max-w-2xl text-sm text-slate-600">Teste a gravação local e a leitura de códigos antes de iniciar o cadastro de produtos.</p></div>
          <button className="button-secondary" onClick={() => void refresh()} disabled={!client.available || loading || saving}>{loading ? 'Verificando…' : 'Verificar banco'}</button>
        </div>

        {!client.available && <div role="status" className="notice">Pré-visualização no navegador. Abra o aplicativo Tauri para testar o SQLite e gravar leituras.</div>}
        {error && <div role="alert" className="notice notice-error">{error}</div>}

        <section aria-label="Estado do banco" className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
          <Status label="SQLite incorporado" value={db?.sqliteVersion ?? 'Aguardando aplicativo'} />
          <Status label="Persistência" value={db ? `${db.journalMode.toUpperCase()} · ${db.synchronous === 2 ? 'FULL' : `nível ${db.synchronous}`}` : 'Não verificada'} />
          <Status label="Integridade verificada" value={db ? (db.integrity === 'ok' ? 'OK · quick_check' : 'Verificar banco') : 'Não verificada'} />
          <Status label="Leituras registradas" value={db ? String(db.totalEntries) : '—'} />
        </section>

        <div className="grid items-start gap-6 lg:grid-cols-[1.25fr_1fr]">
          <section className="panel">
            <div className="section-heading"><span className="step">01</span><div><h2>Teste de leitura e gravação</h2><p>Use o leitor USB ou digite o código e pressione Enter.</p></div></div>
            <form onSubmit={(event) => void submit(event)} noValidate className="mt-6 space-y-5">
              <div><label htmlFor="barcode">Código de barras ou identificador</label><input ref={input} id="barcode" name="barcode" value={code} onChange={(event) => setCode(event.target.value)} autoComplete="off" spellCheck={false} placeholder="Ex.: 0012345678905" className="barcode-input" readOnly={saving} aria-describedby="barcode-help" /><p id="barcode-help" className="mt-2 text-xs text-slate-600">Zeros iniciais são preservados. Configure o terminador do leitor como Enter.</p></div>
              <div><label htmlFor="note">Observação do teste <span className="font-normal text-slate-500">(opcional)</span></label><input id="note" value={note} onChange={(event) => setNote(event.target.value)} placeholder="Ex.: Leitor USB, etiqueta de garrafa" readOnly={saving} /></div>
              <div className="flex flex-wrap gap-3"><button type="submit" className="button-primary" disabled={!client.available || !state || loading || saving}>{saving ? 'Gravando…' : 'Gravar e reler'}</button><button type="button" className="button-secondary" onClick={() => input.current?.focus()}>Focar campo de leitura</button></div>
              <p role="status" aria-live="polite" aria-atomic="true" className="min-h-5 text-sm font-medium text-emerald-800">{feedback}</p>
            </form>
          </section>

          <aside className="panel">
            <div className="section-heading"><span className="step">02</span><div><h2>Conferência do teste</h2><p>O que observar nesta etapa.</p></div></div>
            <ol className="mt-6 space-y-5 text-sm text-slate-700">
              <li><strong>1. Bipar e conferir.</strong><p className="mt-1">O código deve chegar completo, incluindo os zeros, e gerar uma confirmação.</p></li>
              <li><strong>2. Repetir a leitura.</strong><p className="mt-1">Após gravar, o foco retorna ao campo. Cada nova leitura é um novo registro de teste.</p></li>
              <li><strong>3. Fechar e abrir.</strong><p className="mt-1">Os registros devem continuar no histórico após reiniciar o aplicativo.</p></li>
            </ol>
            <p className="mt-6 border-t border-slate-200 pt-4 text-xs leading-relaxed text-slate-600">Esta prova grava apenas leituras de teste. A homologação exige um leitor físico e a conferência dos resultados no Windows.</p>
          </aside>
        </div>

        <section className="panel" aria-labelledby="history-title">
          <div className="flex flex-wrap items-center justify-between gap-3"><div><h2 id="history-title">Histórico da prova</h2><p className="mt-1 text-sm text-slate-600">Últimas 20 leituras persistidas no SQLite.</p></div><span className="text-xs text-slate-500">Data e hora local</span></div>
          {!state?.entries.length ? <div className="empty-state">{loading ? 'Consultando o banco local…' : 'Nenhuma leitura disponível. Faça a primeira gravação no aplicativo.'}</div> : <div className="mt-5 overflow-x-auto"><table><thead><tr><th>Registro</th><th>Código lido</th><th>Observação</th><th>Data e hora</th></tr></thead><tbody>{state.entries.map((entry) => <tr key={entry.id}><td className="text-slate-500">#{entry.id}</td><td className="font-mono font-medium">{entry.code}</td><td>{entry.note || '—'}</td><td className="whitespace-nowrap">{new Date(entry.createdAt).toLocaleString('pt-BR')}</td></tr>)}</tbody></table></div>}
        </section>

        <footer className="space-y-1 text-xs leading-relaxed text-slate-600">
          <p className="break-all">Banco da prova: {state?.databasePath ?? String.raw`%LOCALAPPDATA%\AxisEstoque\phase0\prova.db`}</p>
          <p>Versão {state?.appVersion ?? '0.0.1'} · Esquema {db?.schemaVersion ?? 'não verificado'} · Chaves estrangeiras: {db ? (db.foreignKeys ? 'ativas' : 'inativas') : 'não verificadas'} · Espera por bloqueio: {db ? `${db.busyTimeoutMs} ms` : 'não verificada'}</p>
        </footer>
      </main>
    </div>
  );
}

function Status({ label, value }: { label: string; value: string }) {
  return <div className="rounded-xl border border-slate-200 bg-white px-5 py-4"><p className="text-xs text-slate-600">{label}</p><p className="mt-2 text-lg font-semibold tracking-tight">{value}</p></div>;
}
