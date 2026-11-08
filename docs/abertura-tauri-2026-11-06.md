# Abertura nativa — 06/11/2026

Solicitação: abrir o AXIS ESTOQUE com Tauri, mantendo o escopo da Fase 0.

## Resultado

**A janela nativa foi aberta e conferida.** Após a confirmação de administrador no Windows, a terceira instalação oficial dos Build Tools terminou com sucesso. A compilação Tauri de desenvolvimento passou e o processo `axis-estoque-desktop.exe` iniciou a janela **AXIS ESTOQUE — Validação técnica**.

A captura da própria janela confirmou a comunicação React → Tauri → SQLite: **SQLite 3.54.1**, **WAL · FULL**, **OK · quick_check** e **0 leituras**. O arquivo `%LOCALAPPDATA%\AxisEstoque\phase0\prova.db` foi criado. [Captura da janela nativa](evidencias/tauri-native-2026-11-06.png).

Ambiente validado: Visual Studio Build Tools **17.14.44** (`17.14.37827.9`), MSVC x64/x86 e Windows SDK **10.0.26100.0**. O vswhere informou `isComplete=true`, `isLaunchable=true` e `isRebootRequired=false`.

`cargo test -p axis-estoque-core --locked`: **5 aprovados, 0 falhas**. Os casos exercitam arquivos SQLite temporários reais: persistência após reabertura e configurações de durabilidade; validação e parâmetros; rollback após erro injetado; recusa de banco incompatível sem alterar journal; histórico limitado com códigos repetidos. Não há simulação da conexão SQLite nesses testes.

`cargo clippy --workspace --all-targets --locked -- -D warnings`: **aprovado**, código 0 e nenhum aviso. `cargo fmt --all --check`: aprovado. Resultados estruturados em [nativo-2026-11-06.json](evidencias/nativo-2026-11-06.json).

O Vite já respondia em `127.0.0.1:1420`. Para reaproveitá-lo, o Tauri foi iniciado com um override de desenvolvimento local em `.cache/tauri-reuse-vite.json`, desabilitando apenas o `beforeDevCommand` dessa execução. Nenhuma configuração de segurança ou de banco foi relaxada. O servidor local continua sendo exclusivo do modo de desenvolvimento.

## Ações concluídas

- `npm.cmd run desktop:dev` confirmou inicialmente o bloqueio em `cargo metadata`, pois Cargo não estava instalado.
- Rustup 1.29.3 foi instalado pelo pacote oficial `Rustlang.Rustup` do winget.
- `rustc --version`: **rustc 1.99.1 (58cf8f9c3 2026-10-13)**.
- `cargo --version`: **cargo 1.99.1 (5f94df478 2026-08-27)**.
- Host/toolchain: `stable-x86_64-pc-windows-msvc`.
- `cargo fetch --target x86_64-pc-windows-msvc`: concluído, código 0; Cargo.lock criado e dependências baixadas.
- `cargo fmt --all`: aplicado. `cargo fmt --all --check`: aprovado, código 0.
- Nenhuma alteração de regra de negócio ou avanço de fase.

## Histórico das tentativas anteriores

Pacote oficial: `Microsoft.VisualStudio.2022.BuildTools`, versão oferecida na instalação **17.14.44**. As primeiras tentativas usaram `--quiet`; a terceira usou `--interactive` no winget e o override `--wait --passive --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended`, com progresso visível.

As duas primeiras tentativas retornaram **Installer failed with exit code: 1602**. Na terceira tentativa, o usuário confirmou o UAC e o instalador com progresso visível concluiu a instalação. O SDK concluiu seus 66 pacotes com sucesso; o winget confirmou **Successfully installed**. Não foi contornado o UAC.

Para novas execuções, disponibilizar `%USERPROFILE%\.cargo\bin` no PATH da sessão e executar `npm.cmd run desktop:dev` com a porta 1420 livre. Nesta sessão a janela nativa usa o Vite que já estava aberto.

## Pendências de homologação

Em uma conferência posterior solicitada pelo usuário, a janela estava fechada e havia **1 leitura de teste** persistida. O aplicativo foi reaberto no mesmo banco. A inspeção Rust somente leitura encontrou **1 registro antes e depois**, com conteúdo completo idêntico por SHA-256 e integridade OK. Nenhum registro foi criado ou alterado pela inspeção. [Evidência da reabertura](evidencias/reabertura-2026-11-06.json).

O leitor USB real, layout ABNT2, encerramento controlado seguido de reabertura, executável release sem rede, interrupção abrupta e instalação limpa continuam pendentes. A reabertura do arquivo real existente e a preservação de sua leitura de teste foram verificadas; a operação de gravação/leitura e fechamento/reabertura de conexões também foi verificada pelos testes Rust em arquivos temporários. Não houve avanço para a Fase 1.
