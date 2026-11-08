# Relatório consolidado — AXIS ESTOQUE V1.3.0

Data de referência: 07/11/2026. Entrega integrada V1.1, V1.2 e V1.3, incluindo a base comercial mínima ausente na Fase 0. Núcleo único Rust/SQLite real; frontend conectado ao Tauri. A estrutura React/Tauri existente e a identidade verde foram mantidas. Não há servidor obrigatório, dependência de nuvem nem dados sintéticos no banco principal.

## V1.1

Concluídos: XML local NF-e 4.00 UTF-8, namespaces/envelopes, validação estrutural/chave, bloqueio de DTD/entidades e limites, duplicidade, fornecedor, associação exata/ambígua/desconhecida, criação de produto, fator por SKU/fornecedor, decimais e centavos, conferência de arredondamento, rascunho persistido e confirmação transacional/idempotente. Venda não é alterada automaticamente. XML e dados originais ficam preservados; correções usam ajustes.

Testes aprovados: válido/prefixado, inválido/não suportado/chave, entidades/limites, desconhecido/ambíguo, associação após reabrir, 5 × 12 = 60, arredondamento, duplicidade e rollback. O fluxo nativo confirmou entrada e reaproveitamento da associação numa segunda chave depois de reiniciar o processo.

Pendências de homologação: XMLs reais dos fornecedores da adega, diferenças de embalagem praticadas e conferência operacional. Autenticidade/assinatura/autorização fiscal não são verificadas; não são anunciadas como verificadas.

## V1.2

Concluídos: lotes opcionais, fabricação/validade/safra, divisão de itens, origem/custo, saldo pelo livro, FEFO com tratamento explícito de vencidos/sem data, alertas locais 7/30 configuráveis, filtros e estimativas de custo, reclassificação de saldo sem lote sem mudar total. Sem datas fictícias para estoques anteriores.

Testes aprovados: múltiplos lotes, soma da distribuição, produto com/sem data, FEFO, vencidos, limites de alerta, negativos total/lote, custo original do lote e regularização. No Tauri, entrada 60 e saída FEFO 10 geraram saldo 50 no lote correto.

Pendências de homologação: regras operacionais para mercadoria vencida, informações reais de safra e usabilidade no estoque físico.

## V1.3

Concluídos: inventário completo/parcial, uma sessão ativa, referências por SKU/lote, eventos de leitura/correção, Enter/quantidade/zeros, escolha de variante/lote, desconhecidos/URLs, conferência/filtros, decisão obrigatória sobre não contados, bloqueio em Rust/SQL, cancelamento, fechamento atômico/idempotente, histórico e retomada após reiniciar.

Testes aprovados: completos/parciais, repetição/quantidade manual, desconhecidos/ambíguos, diferenças positivas/negativas/iguais, skip/zero, lotes, revisão desatualizada, cancelamento, duas conexões, sessão ativa reaberta, falha no fechamento com rollback e finalizados imutáveis. No fluxo nativo, referência 50 e contagem 48 geraram um ajuste −2; reabrir manteve saldo 48, lote, sessão, eventos e associação.

Pendências de homologação: scanner USB/celular HID físicos, ABNT2, ritmo de bipagem e recuperação após queda de energia real. A interface exige aguardar feedback; reenvios de resposta incerta mantêm o mesmo identificador.

## Banco e compatibilidade

Esquema evoluído de 1 para 5, application_id e caminho originais preservados. Migrações 002–005 adicionam a base comercial, XML/associações, lotes e inventário: 21 tabelas totais, incluindo probe_entries. Backup online consistente, ensaio em cópia e transação precedem migração real. Foreign keys, WAL/FULL, integridade, índices e gatilhos reforçam o núcleo. [Detalhes e recuperação](database.md).

A migração real para esquema 5 foi aplicada e conferida na abertura do release. A única leitura legada permaneceu idêntica: snapshots antes/depois e do backup têm SHA-256 `dd602690ad99f9efe888c939d546b96875a0b915782c046910de6473608c7d21`. SQLite 3.54.1, integridade `ok`, WAL, foreign keys e synchronous=FULL foram confirmados pelo IPC nativo. Backup em `%LOCALAPPDATA%\AxisEstoque\phase0\backups\antes-schema-1-1794083584471607900.db` (8 KiB), também lido e conferido. O banco original não recebeu SKUs sintéticos. [Evidência de release](evidencias/release-v1-integrada.json).

O histórico anterior está acessível em Configurações → histórico da Fase 0. Havia uma leitura técnica e nenhum cadastro comercial antes desta implementação; a migração preserva essa leitura sem convertê-la em produto fictício. Nenhum teste mutável nativo usa o banco principal: o script exige perfil debug separado.

## Validação e compilação

Resultados já concluídos:

- TypeScript, ESLint sem avisos, build web: aprovados. Bundle JavaScript aproximadamente 307 kB (91 kB gzip); CSS 23 kB.
- React/Vitest: 22 testes aprovados, incluindo os 6 testes legados. Clientes injetados são restritos aos testes.
- Rust: **32 testes aprovados** — 5 legados e 27 de integração. A repetição final executou os binários de teste compilados em `target/debug/deps` enquanto o Cargo compilava o release; 27/27 e 5/5 passaram, incluindo 1000 SKUs/5000 movimentos. O caso de massa usa a API real e SQLite temporário, sem mocks de persistência; não é uma medição de latência em máquina de produção.
- `cargo check --workspace --offline` e `cargo clippy --workspace --all-targets --offline -- -D warnings`: aprovados. `cargo build -p axis-estoque-desktop --offline`: aprovado.
- Tauri/IPC/SQLite reais: [11 verificações integradas aprovadas](evidencias/native-v1-integrada.json), processo fechado normalmente e reaberto; captura visual real em banco isolado.

Comandos principais: `npm.cmd run typecheck`, `npm.cmd run lint`, `npm.cmd test`, `npm.cmd run build`, `cargo test -p axis-estoque-core --offline`, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --offline -- -D warnings`, `npm.cmd run desktop:build`. O último usa `tauri build --no-bundle`, incorporando os arquivos locais. Build release e recompilação final aprovados: `target/release/axis-estoque-desktop.exe` (11.324.928 bytes). SHA-256: `ef7d0636045ac903c0da054bd664eb2ac6ee41b68fd4885cff6e9f3cf2d86afb`.

O executável abriu em `http://tauri.localhost/` com os mesmos assets do build final e com o Vite desligado. A variável de perfil debug foi explicitamente ignorada no release. As dez páginas foram abertas em viewport emulado 760 × 640 sem overflow horizontal da página; as capturas nativas mostram a janela 1200 × 900. Após a inspeção, a janela foi encerrada normalmente e reaberta sem flags temporárias de depuração. O aplicativo foi deixado aberto para os testes manuais. Não há erro conhecido de build ou teste automatizado pendente; homologação operacional continua necessária.

Não foi gerado instalador NSIS assinado nesta entrega. Instalação em Windows limpo, distribuição/atualização assinada e hardware físico permanecem pendentes; os documentos históricos da Fase 0 não substituem os resultados atuais. Não foi simulado desligamento físico da rede ou queda de energia.

## Arquivos e checkpoints

- Núcleo: `crates/axis-estoque-core/src/{model,decimal,storage,catalog,stock,receipts,xml,lots,inventory}.rs`, integração em `lib.rs`, migrações 002–005 e testes/fixtures.
- Native: `src-tauri/src/main.rs`, configuração da janela/versão e dependências em Cargo.toml/Cargo.lock.
- Interface: `WorkspaceApp`, `Products`, `Suppliers`, `Receipts`, `Inventory`, `Views`, `ui`, `format`, `task`, `business-api`, estilos e testes; prova anterior preservada em `ProbeApp`.
- Scripts: checkpoint, integração nativa isolada e verificação somente de leitura do release. Documentação arquitetural, banco, módulos, ADR e roteiro manual.
- Não existe repositório Git neste workspace. Foram usados manifestos SHA-256 por etapa em `docs/checkpoints`, preservando o manifesto histórico da Fase 0. Cada manifesto representa o momento da criação; alterações posteriores podem torná-lo diferente do estado atual. Para conferir a entrega final, use o manifesto `v1-integrada.sha256`.

## Teste manual separado

1. [Importação XML](validation-checklist.md): cadastrar SKU, importar fixture, conferir 5 caixas × 12, lotes/custos, confirmar, repetir chave e reabrir.
2. [Lotes e validade](validation-checklist.md): segundo lote, FEFO, negativos, saldo sem data, alertas por data local e compensações.
3. [Inventário](validation-checklist.md): escopo parcial/completo, 1+1+12, escolha/correção de lote, bloqueios, retomada, diferença −2, decisão sobre não contados e fechamento/cancelamento.

Use o perfil de homologação com a faixa visível. O roteiro detalha os saldos esperados e distingue cada verificação automatizada da homologação física pendente.
