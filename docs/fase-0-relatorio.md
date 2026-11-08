# Fase 0 — relatório técnico

Data: **05/11/2026**, America/Sao_Paulo. Versão do checkpoint: **0.0.1**.

Atualização em **06/11/2026**: Rust/Cargo, MSVC e Windows SDK instalados; Cargo.lock gerado e formatação Rust aprovada. **Cinco testes Rust passaram e a janela Tauri abriu**, mostrando SQLite 3.54.1, WAL/FULL e integridade OK. A homologação física permanece pendente. O relatório abaixo preserva os resultados da execução original de 05/11. Consulte o [registro atualizado da abertura](abertura-tauri-2026-11-06.md).

**Resultado: prova implementada; Fase 0 ainda não homologada.** Frontend validado. Compilação e execução nativas impedidas pela ausência de Rust/Cargo e Build Tools C++/Windows SDK. Não houve avanço para a Fase 1.

## O que foi criado

- Workspace Cargo com núcleo `axis-estoque-core` independente do Tauri e shell Tauri 2.
- Interface React/TypeScript/Vite/Tailwind em português: estado real do banco, leitura por Enter/digitação, observação opcional, feedback, foco explícito e histórico das últimas 20 leituras.
- Comandos tipados `get_probe_state` e `record_probe`; nenhum SQL ou caminho arbitrário recebido da interface.
- Prova de persistência SQLite: tabela técnica, gravação parametrizada em transação IMMEDIATE, releitura antes do commit e retorno de sucesso após confirmação. Zeros iniciais preservados em texto.
- Código de abertura com WAL, foreign_keys, synchronous FULL e busy_timeout de 5 s em toda conexão; quick_check consultável pela interface.
- Banco isolado em `%LOCALAPPDATA%\AxisEstoque\phase0\prova.db`. O caminho comercial `data\estoque.db` permanece reservado.
- Seis testes de interface e cinco testes Rust de persistência/configuração, validação e parâmetros, falha injetada, recusa de banco incompatível e limite/ordem do histórico. **Os testes Rust foram escritos, mas não executados.**
- Diagnóstico reproduzível, ícone local, configuração preliminar NSIS/WebView2, roteiro de aceite, ADRs, README, CHANGELOG e checkpoint SHA-256.

## Ambiente encontrado

| Item | Evidência |
|---|---|
| Windows | Windows 10 Pro 22H2, x64, build 19045.6466 |
| Node/npm | 24.16.0 / 11.13.0 |
| WebView2 | Runtime 156.0.4314.58 no registro e detectado pelo Tauri CLI |
| Git | 2.53.0.windows.2; diretório inicial sem repositório |
| Rust/Cargo/rustup | Não encontrados no PATH nem nos diretórios padrão consultados |
| MSVC/Windows SDK | Não encontrados; confirmado pelo diagnóstico `tauri info` |
| Leitor | Modelo, configuração e funcionamento físico ainda não verificados |

O inventário está em [ambiente.json](evidencias/ambiente.json). Consultas CIM de sistema/hardware retornaram acesso negado; Windows e WebView2 foram conferidos por APIs/registro e Tauri CLI. RAM/CPU/disco não foram homologados ou medidos.

## Verificações executadas

Saídas e códigos de retorno estão em [verificacao.json](evidencias/verificacao.json).

| Comando | Resultado final |
|---|---|
| `npm.cmd run typecheck` | Aprovado; código 0 |
| `npm.cmd run lint` | Aprovado; zero avisos; código 0 |
| `npm.cmd test` | 1 arquivo, **6 testes aprovados**; código 0 |
| `npm.cmd run build` | Bundle de produção gerado; código 0 |
| `npm.cmd audit --json` | **0 vulnerabilidades reportadas** na árvore npm final; código 0 |
| `npm.cmd run tauri -- info` | Executado; detectou WebView2 e apontou MSVC/SDK, Rust e Cargo ausentes. Código 0 do diagnóstico não significa ambiente aprovado |
| `npm.cmd run desktop:build` | Bloqueado em `cargo metadata`: programa não encontrado; código 1; nenhum executável nativo gerado |
| `npm.cmd run test:core` | Bloqueado: Cargo não encontrado; código 1; nenhum teste Rust executado |
| `npm.cmd run check:rust` | Bloqueado: Cargo não encontrado; código 1; fmt/clippy não executados |

O lint inicial apontou problemas em globals dos scripts, chamada síncrona de atualização de estado no efeito React e escape do caminho Windows. Todos foram corrigidos antes da execução final acima. A instalação inicial de TypeScript 7 foi recusada por conflito de peer dependency; adotou-se TypeScript 6.0.3 compatível.

A auditoria inicial apontou vulnerabilidade transitiva de minimatch na ferramenta de lint. A árvore final usa minimatch 11.0.1 com override documentado; lint, tipos, testes e build passaram após a correção. Nenhuma resolução foi forçada com `--legacy-peer-deps` ou `audit fix --force`.

Os seis testes de interface cobrem: Enter/zeros/foco, envio simultâneo e repetição posterior, erro sem falso sucesso, entrada vazia/longa, bloqueio de gravação no navegador e recuperação de falha de consulta. São testes com cliente injetado: **não validam banco, IPC ou USB reais**.

## O que não pôde ser testado

- Compilação do workspace, resolução Cargo.lock, fmt, clippy e cinco testes Rust.
- Janela Tauri real, round-trip IPC, abertura/gravação/reabertura do SQLite incorporado e seus PRAGMAs em runtime.
- Leitor USB real, layout ABNT2, EAN-8/EAN-13/Code128 do equipamento e terminador configurado.
- Execução do desktop sem rede, instalação NSIS e Windows limpo com/sem WebView2.
- Queda de energia, interrupção de processo, disco cheio, locks entre processos ou hardware de produção.
- Benchmarks de abertura, busca, recebimento, memória, backup e CSV; não há metas declaradas como atingidas. O tempo de build exibido pelo Vite não mede desempenho do aplicativo.

Backup/recuperação operacional, migrações comerciais, atualizações assinadas, PIN, auditoria de estoque, 1.000 SKUs e importação CSV pertencem a fases futuras e não foram implementados nesta prova.

## Riscos técnicos e decisões

1. **Bloqueio de toolchain:** instalar Rust stable MSVC, rustfmt/clippy, Microsoft C++ Build Tools e Windows SDK é necessário antes de homologar. As dependências do instalador estão em [windows-instalador.md](windows-instalador.md).
2. **Integridade WAL:** a [documentação oficial do SQLite](https://www.sqlite.org/wal.html) registra a correção em 3.51.3 e posteriores. Foi selecionado rusqlite 0.41.1 bundled, documentado com SQLite 3.54.0; o código recusa runtime inferior a 3.54.0. A versão efetivamente incorporada ainda precisa ser comprovada no build nativo. [Fonte rusqlite](https://docs.rs/crate/rusqlite/0.41.1).
3. **Compatibilidade do banco:** application_id próprio e versão única da prova; somente banco vazio pode ser inicializado. Outro esquema é recusado antes de alterar o journal. Não foi criada infraestrutura de migração de produção.
4. **Conferência física:** a entrada não usa detecção por velocidade. Durante gravação o campo fica somente leitura; aguardar confirmação antes de nova bipagem. Throughput e comportamento do leitor real continuam pendentes.
5. **Integridade e resultado incerto:** a trava visual evita envios simultâneos desta janela; não equivale a idempotência de recebimento. Uma queda após commit e antes da resposta exige conferência do histórico. Identificadores de origem persistentes e compensações serão requisitos do domínio nas próximas fases.
6. **Distribuição:** NSIS por usuário, dados fora da instalação e WebView2 offline previstos em configuração. Bundle desabilitado; sem instalador publicado, chaves ou endpoint fictício. Atualizador Tauri e Authenticode serão separados; sem alegação de rollback automático.
7. **Dependências:** versões JS travadas em package-lock; override de minimatch precisa ser revisitado quando seus consumidores aceitarem a correção normalmente. Falta Cargo.lock, bloqueando reprodutibilidade transitiva Rust até a instalação de Cargo.

Decisões completas: [ADR 0001](adr/0001-prova-isolada-e-fronteiras.md) e [ADR 0002](adr/0002-versoes-e-dependencias-da-prova.md).

## Checkpoint e encerramento

Como a pasta não continha repositório Git, foi adotado um checkpoint verificável em `docs/checkpoints/fase-0.sha256`, gerado por `node scripts/checkpoint.mjs`. Validar com `node scripts/checkpoint.mjs --verify`. O manifesto registra os arquivos da entrega, inclusive estas evidências; aprovação de conteúdo não substitui homologação nativa.

Para concluir a Fase 0: disponibilizar toolchain, gerar/revisar Cargo.lock, corrigir qualquer falha nativa, executar os testes Rust, abrir o desktop e preencher o [roteiro de aceite](aceitacao-fase-0.md) com leitor físico. Não avançar com falhas pendentes.

## Planejamento da Fase 1

Somente após homologação da Fase 0: estruturar tipos de domínio, repositórios pequenos, banco comercial, constraints/índices e migrações versionadas com backup consistente validado antes de alterações. O livro de movimentos imutável será a fonte do saldo; não haverá edição direta de quantidade. Testar migração anterior, recusa de esquema futuro, falha com rollback e recuperação antes de liberar o uso comercial.

Arquivos previstos, riscos e critérios de aceite estão no [plano da Fase 1](fase-1-plano.md). **Esse plano não foi executado.**
