# ADR 0001 — Prova isolada e comandos fechados

Data: 05/11/2026. Status: adotada na prova; homologação física e de distribuição pendentes.

Atualização de 06/11/2026: cinco testes Rust aprovados e janela Tauri aberta. A resolução no Cargo.lock usa libsqlite3-sys 0.39.1, e a interface nativa confirmou SQLite 3.54.1. A barreira mínima 3.54.0 permanece. Homologação do leitor físico e distribuição continuam pendentes.

O repositório estava vazio. A primeira execução deve limitar-se à Fase 0.

## Decisão

- Usar workspace Cargo com `axis-estoque-core` independente de Tauri e um shell desktop mínimo.
- Usar React, TypeScript, Vite e Tailwind, com `get_probe_state` e `record_probe` encapsulados em `src/api.ts`.
- Nenhum comando recebe SQL ou caminho fornecido pela interface. Sem plugins de shell, SQL ou filesystem. Sem acesso remoto ou atualização nesta prova.
- Apenas a janela `main` tem uma capability local, sem permissões de plugins. Os dois comandos próprios são registrados explicitamente no invoke handler; não há outras janelas ou conteúdo remoto.
- Usar `%LOCALAPPDATA%\AxisEstoque\phase0\prova.db`, reservando `%LOCALAPPDATA%\AxisEstoque\data\estoque.db` para o produto.
- Identificar a prova por `application_id=0x41585030` e `user_version=1`; inicializar somente banco vazio. Recusar qualquer outro esquema antes de modificar journal mode. Não há migração de banco existente nesta fase.
- Transações `IMMEDIATE`, parâmetros SQL, WAL, foreign_keys em toda conexão, busy_timeout de 5 s e synchronous FULL. Gravar, reler e confirmar antes de retornar sucesso.
- Incorporar SQLite via `rusqlite =0.41.1` (`bundled`, fonte documentada 3.54.0). Runtime exige SQLite >=3.54.0, posterior à correção WAL de 3.51.3. Revalidar a versão real ao compilar.
- Usar o modo teclado HID com campo explícito e envio por Enter. Sem heurística de velocidade, acesso USB especial ou interpretação numérica de códigos.
- Não criar uma interface de repositório artificial nesta prova: a abstração `ProbeDatabase` delimita persistência e permite testes em arquivos temporários. Repositórios do domínio serão definidos com os casos reais na Fase 1.

## Consequências

A prova não cria dados comerciais nem migra um banco de produção. Os registros repetidos são intencionais para teste de leituras. A trava da interface evita submissões simultâneas, mas não oferece idempotência persistente entre processos nem resolve resultado incerto após queda na resposta IPC; esses requisitos pertencem às operações reais futuras.

O navegador mostra apenas a interface sem gravação, sem localStorage e sem banco simulado. Testes React usam um cliente injetável e não demonstram funcionamento Tauri.

Não foram implementados backup, restauração, migração operacional, updater, PIN, logs estruturados, diagnóstico exportável ou instalador de distribuição. A inicialização da prova ocorre apenas em banco vazio e não modifica esquemas antigos.

## Fontes

- [Tauri: requisitos Windows](https://v2.tauri.app/start/prerequisites/)
- [Tauri: instalador Windows e WebView2](https://v2.tauri.app/distribute/windows-installer/)
- [SQLite: WAL e correção de integridade](https://www.sqlite.org/wal.html)
- [rusqlite: versão incorporada e compilação](https://docs.rs/crate/rusqlite/0.41.1)
- [SQLite 3.54.0](https://www.sqlite.org/releaselog/3_54_0.html)
