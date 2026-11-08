import { useRef,useState } from 'react';
import { errorMessage } from './api';
export function useTask(){
 const[busy,setBusy]=useState(false);const[error,setError]=useState('');const[message,setMessage]=useState('');const flight=useRef(false);const lastError=useRef<unknown>(null);
 async function run<T>(action:()=>Promise<T>,success=''):Promise<T|undefined>{
  if(flight.current)return;flight.current=true;lastError.current=null;setBusy(true);setError('');setMessage('');
  try{const value=await action();setMessage(success);return value;}catch(e){lastError.current=e;setError(errorMessage(e));return undefined;}finally{flight.current=false;setBusy(false);}
 }
 return{busy,error,message,run,setError,setMessage,lastError};
}
export function matches(value:string,search:string){const fold=(v:string)=>v.normalize('NFD').replace(/\p{Diacritic}/gu,'').toLocaleLowerCase('pt-BR');return fold(value).includes(fold(search));}
