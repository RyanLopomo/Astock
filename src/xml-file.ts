export const MAX_XML_FILE_BYTES=8*1024*1024;

export async function readXmlFile(file:File):Promise<string>{
 if(file.size>MAX_XML_FILE_BYTES)throw new Error('O XML excede 8 MiB.');
 const bytes=await new Promise<ArrayBuffer>((resolve,reject)=>{
  const reader=new FileReader();
  reader.onerror=()=>reject(new Error('Não foi possível ler o arquivo XML.'));
  reader.onabort=()=>reject(new Error('A leitura do XML foi interrompida.'));
  reader.onload=()=>reader.result instanceof ArrayBuffer?resolve(reader.result):reject(new Error('Arquivo XML inválido.'));
  reader.readAsArrayBuffer(file);
 });
 try{return new TextDecoder('utf-8',{fatal:true}).decode(bytes);}
 catch{throw new Error('O XML contém bytes inválidos em UTF-8. Selecione o arquivo original nessa codificação.');}
}
