import { useEffect, useState } from 'react';

// Preferências de consulta são opcionais. Cadastros e rascunhos permanecem no SQLite.
export function useSessionPreference<T>(key:string,initial:T,valid:(value:unknown)=>value is T) {
 const [value,setValue]=useState<T>(()=>{
  try { const stored=window.sessionStorage.getItem(`astock:ux:v1:${key}`);if(stored!==null){const parsed:unknown=JSON.parse(stored);if(valid(parsed))return parsed;} } catch { /* Janela sem armazenamento: estado em memória. */ }
  return initial;
 });
 useEffect(()=>{
  try { window.sessionStorage.setItem(`astock:ux:v1:${key}`,JSON.stringify(value)); } catch { /* Consulta funciona sem persistência. */ }
 },[key,value]);
 return [value,setValue] as const;
}
