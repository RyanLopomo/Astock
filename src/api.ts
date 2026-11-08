import { invoke, isTauri } from '@tauri-apps/api/core';

export interface ProbeEntry {
  id: number;
  code: string;
  note: string;
  createdAt: string;
}

export interface ProbeInput {
  code: string;
  note: string;
}

export interface ProbeState {
  appVersion: string;
  databasePath: string;
  database: {
    sqliteVersion: string;
    schemaVersion: number;
    journalMode: string;
    foreignKeys: boolean;
    synchronous: number;
    busyTimeoutMs: number;
    integrity: string;
    totalEntries: number;
  };
  entries: ProbeEntry[];
}

export interface ProbeClient {
  available: boolean;
  getState(): Promise<ProbeState>;
  record(input: ProbeInput): Promise<ProbeEntry>;
}

export const desktopClient: ProbeClient = {
  available: isTauri(),
  getState: () => invoke<ProbeState>('get_probe_state'),
  record: (input) => invoke<ProbeEntry>('record_probe', { input }),
};

export function errorMessage(error: unknown): string {
  if (typeof error === 'object' && error !== null && 'message' in error && typeof error.message === 'string') {
    return error.message;
  }
  return 'Não foi possível concluir a operação. Verifique o aplicativo e tente novamente.';
}
