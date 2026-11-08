import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { vi } from 'vitest';
import App from './ProbeApp';
import type { ProbeClient, ProbeEntry, ProbeState } from './api';

const initialState: ProbeState = {
  appVersion: '0.0.1', databasePath: 'C:\\prova\\prova.db', entries: [],
  database: { sqliteVersion: '3.54.0', schemaVersion: 1, journalMode: 'wal', foreignKeys: true, synchronous: 2, busyTimeoutMs: 5000, integrity: 'ok', totalEntries: 0 },
};

function client(): ProbeClient {
  return { available: true, getState: vi.fn().mockResolvedValue(structuredClone(initialState)), record: vi.fn().mockImplementation(async (input) => ({ id: 1, ...input, createdAt: '2026-11-05T12:00:00Z' })) };
}

test('Enter grava código como texto, exibe resposta e devolve foco', async () => {
  const api = client();
  const user = userEvent.setup();
  render(<App client={api} />);
  await screen.findByText('3.54.0');
  const field = screen.getByLabelText('Código de barras ou identificador');
  await user.type(field, '0012345678905{Enter}');
  expect(api.record).toHaveBeenCalledWith({ code: '0012345678905', note: '' });
  await screen.findByText('Código 0012345678905 gravado e relido no banco local.');
  expect(screen.getByRole('cell', { name: '0012345678905' })).toBeVisible();
  expect(field).toHaveValue('');
  expect(field).toHaveFocus();
});

test('não duplica submissão durante a transação, permite nova leitura depois', async () => {
  const api = client();
  let finish!: (entry: ProbeEntry) => void;
  api.record = vi.fn().mockImplementation(() => new Promise<ProbeEntry>((resolve) => { finish = resolve; }));
  render(<App client={api} />);
  await screen.findByText('3.54.0');
  const field = screen.getByLabelText('Código de barras ou identificador');
  fireEvent.change(field, { target: { value: '00000123' } });
  const form = field.closest('form')!;
  fireEvent.submit(form);
  fireEvent.submit(form);
  expect(api.record).toHaveBeenCalledTimes(1);
  expect(screen.getByRole('button', { name: 'Gravando…' })).toBeDisabled();
  finish({ id: 1, code: '00000123', note: '', createdAt: '2026-11-05T12:00:00Z' });
  await screen.findByText('Código 00000123 gravado e relido no banco local.');
  fireEvent.change(field, { target: { value: '00000123' } });
  fireEvent.submit(form);
  expect(api.record).toHaveBeenCalledTimes(2);
  finish({ id: 2, code: '00000123', note: '', createdAt: '2026-11-05T12:00:01Z' });
  await screen.findByRole('cell', { name: '#2' });
});

test('falha mantém código para nova tentativa e não informa sucesso', async () => {
  const api = client();
  api.record = vi.fn().mockRejectedValue({ code: 'DATABASE_ERROR', message: 'Banco ocupado. Tente novamente.' });
  render(<App client={api} />);
  await screen.findByText('3.54.0');
  const field = screen.getByLabelText('Código de barras ou identificador');
  fireEvent.change(field, { target: { value: '00123' } });
  fireEvent.submit(field.closest('form')!);
  expect(await screen.findByRole('alert')).toHaveTextContent('Banco ocupado. Tente novamente.');
  expect(field).toHaveValue('00123');
  expect(field).toHaveFocus();
  expect(screen.queryByRole('cell')).not.toBeInTheDocument();
});

test('bloqueia entrada vazia e código maior que o limite antes de invocar Rust', async () => {
  const api = client();
  render(<App client={api} />);
  await screen.findByText('3.54.0');
  const field = screen.getByLabelText('Código de barras ou identificador');
  const form = field.closest('form')!;
  fireEvent.submit(form);
  expect(await screen.findByRole('alert')).toHaveTextContent('Informe um código');
  fireEvent.change(field, { target: { value: 'a'.repeat(129) } });
  fireEvent.submit(form);
  expect(api.record).not.toHaveBeenCalled();
});

test('navegador informa ausência do Tauri sem inventar conexão SQLite', () => {
  const api = client();
  api.available = false;
  render(<App client={api} />);
  expect(screen.getByText(/Pré-visualização no navegador/)).toBeVisible();
  expect(screen.getByRole('button', { name: 'Gravar e reler' })).toBeDisabled();
  expect(api.getState).not.toHaveBeenCalled();
  expect(api.record).not.toHaveBeenCalled();
});

test('falha ao abrir consulta permite tentar novamente', async () => {
  const api = client();
  api.getState = vi.fn().mockRejectedValueOnce(new Error('Falha ao consultar banco.')).mockResolvedValue(initialState);
  render(<App client={api} />);
  expect(await screen.findByRole('alert')).toHaveTextContent('Falha ao consultar banco.');
  fireEvent.click(screen.getByRole('button', { name: 'Verificar banco' }));
  await screen.findByText('3.54.0');
  await waitFor(() => expect(screen.queryByRole('alert')).not.toBeInTheDocument());
});
