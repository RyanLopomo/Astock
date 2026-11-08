# Windows — dependências e preparação do instalador

Esta é a documentação da Fase 0. Não foi produzido instalador homologado. A distribuição pertence à Fase 7.

## Máquina de desenvolvimento/build

1. Windows x64; o ambiente inspecionado é Windows 10 Pro 22H2, build 19045.6466.
2. Node 24.15+ da linha 24, npm e dependências do `package-lock.json`.
3. Rust stable com host/target `x86_64-pc-windows-msvc`; `cargo`, `rustc`, `rustfmt` e `clippy`.
4. Microsoft Visual Studio Build Tools, carga **Desktop development with C++**, compilador/linker MSVC x64/x86 e Windows SDK. O SQLite bundled também precisa do compilador C.
5. WebView2 Evergreen Runtime x64.
6. Acesso à rede para baixar dependências e ferramentas de empacotamento. Isso não implica rede durante a operação diária do aplicativo.

Não foi necessário alterar a política global de scripts: `npm.cmd` funciona no PowerShell deste ambiente. O cache npm foi direcionado a `.cache/npm` dentro do projeto porque o diretório padrão está fora da área gravável da sessão.

## Máquina do cliente

- Não precisa instalar Node, Rust, SQLite separado, Docker ou servidor.
- Precisa de WebView2 compatível; a instalação deverá conferir sua presença.
- A configuração inicial aponta para NSIS por usuário (`currentUser`) e `offlineInstaller` do WebView2. O build deve incorporar o runtime para permitir instalação sem internet em Windows limpo; validar tamanho, arquitetura e comportamento com/sem runtime existente.
- O instalador é desabilitado nesta prova (`bundle.active=false`). `npm.cmd run desktop:build` tenta produzir apenas o executável para validação, quando a toolchain estiver disponível.
- Sem certificados, segredos ou endpoints fictícios. Tauri Updater assinado e assinatura Authenticode Windows são mecanismos distintos a implementar e testar na Fase 7. Não há promessa de rollback automático.

## Dados e atualização

O diretório de instalação conterá apenas o aplicativo. Os dados comerciais deverão usar `%LOCALAPPDATA%\AxisEstoque\data\estoque.db`; nesta prova usa-se `phase0\prova.db`.

Instalador e desinstalador devem preservar dados e backups, salvo exclusão explícita e confirmada. Esse comportamento ainda precisa de teste em VM limpa. Migração de produção só poderá ocorrer após backup consistente validado; restauração requer cópia recuperável e teste em banco temporário. Não implementar migrações comerciais antes dessas condições.

## Portões para distribuição

- Build nativo e testes Rust/IPC aprovados com Cargo.lock revisado.
- Instalação/desinstalação em VM limpa x64 com WebView2 presente e ausente, incluindo cenário sem internet.
- Verificação de runtime MSVC no Windows limpo e dependências reais do binário produzido.
- Abertura, persistência após reinício, uso por usuário sem privilégios e leitor HID real.
- Assinatura, manifesto HTTPS e recuperação de falha de migração testados nas fases correspondentes.

Referências: [pré-requisitos oficiais](https://v2.tauri.app/start/prerequisites/) e [NSIS/WebView2](https://v2.tauri.app/distribute/windows-installer/).
