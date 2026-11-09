# Testes de segurança executados

Data: 08/11/2026. Todos os testes mutáveis usaram cliente injetado, arquivos SQLite temporários ou perfis debug sintéticos. A base principal só foi lida para backup/digest e recebeu a alteração de ACL autorizada; sourceDataChanges foi zero.

## Resultados

| Comando / prova | Resultado observado |
|---|---|
| npm test, antes | 51 aprovados |
| cargo test -p axis-estoque-core --offline, antes | 5 + 27 aprovados |
| npm run typecheck | Aprovado |
| npm run lint | Aprovado após corrigir expressão do harness de teste |
| npm test, depois | 57 aprovados |
| npm run test:security | 3 aprovados |
| cargo test --workspace --offline | 46 aprovados: 5 legados, 27 integração, 9 segurança, 5 shell/IPC/rede |
| Helper crash_fixture_child | Ignorado na lista principal; invocado de verdade pelo teste pai e encerrado com código 91 esperado |
| cargo test -p axis-estoque-core --test security --offline | 9 aprovados em regressão adicional após proteção de journal |
| cargo fmt --all --check / clippy --workspace --all-targets --offline -- -D warnings | Aprovados |
| npm run build / desktop:build | Builds executados; artefato final e digest na evidência de build |
| security-config-check --dist | 26 comandos, capability local, CSP restrita, updater ausente e 3 arquivos de frontend; sem map/arquivo privado/asset órfão |
| npm audit --json --registry=https://registry.npmjs.org | Zero vulnerabilidades reportadas |
| cargo-audit 0.22.2 portátil, --no-fetch --no-yanked | Zero vulnerabilidades, dois avisos informativos Linux |
| Gitleaks 8.30.1 git/dir --redact | Nenhum vazamento encontrado |
| Scanner local e self-test | Token sintético reconhecido sem imprimir valor; scans de código/Git/artefatos sem credencial real reconhecida |
| Fluxo Tauri real | XML, lote, FEFO, inventário, idempotência, reinício, associação e backup aprovados |
| Bateria nativa de segurança | Oito verificações aprovadas; detalhadas no JSON |
| Proteção ACL real | Três entradas permitidas no diretório/banco/backups; digest lógico igual e zero mudanças SQL na origem |

## Cobertura comprovada

- **SQL/estoque:** string de tentativa DROP TABLE persistida como código e localizada por parâmetro, sem execução; constraints, negativo total/lote, recebimento duplicado, revisão, idempotência, falhas injetadas e rollback de múltiplos itens/fechamento. O caso 1000 SKUs/5000 movimentos passou; duração de testes concorrentes não é benchmark de produção.
- **Recuperação:** backup online com WAL ativo, recusa de sobrescrita, restauração de cópia em outro arquivo, integrity_check/FKs, falha em migração sem mudar original e subprocesso encerrado com transação não confirmada. A reabertura preservou o commit anterior e desfez a alteração não confirmada. Não foi simulado desligamento físico nem kill no meio de DDL da migração.
- **Caminhos:** diretório relativo/parent, UNC/dispositivos Windows, hard link e junção sintéticos. Tentativas não criaram o banco no destino externo e não alteraram seus bytes. Nenhum comando aceita caminho arbitrário do frontend.
- **XSS:** tags/scripts/eventos/URLs como texto em produtos, busca, tabelas, fornecedor, observações, notificações e importação XML. Teste React verifica ausência de nós executáveis; prova em WebView real persiste produto malicioso e verifica que não executa marcador JavaScript.
- **CSP e rede:** inserções sintéticas de script inline/remoto, evento de imagem, fetch e iframe foram bloqueadas. O servidor de teste escutou apenas loopback; não foi feito pedido a terceiros. Navegação fora da origem e popup foram rejeitados; filtro final impediu pedidos HTTP ao servidor controlado.
- **IPC:** outra origem/rótulo, comando SQL inexistente, plugin de arquivos ausente, criação de WebView sem permissão, ID inválido, argumento path extra, estruturas profundas/corpo grande e listas extensas. Limites são aplicados no Rust antes de enfileirar trabalho, além das validações de negócio.
- **Abuso:** 16 importações concorrentes preservaram uma chave de compra e saldo zero; 8 backups simultâneos criaram só um; 100 consultas retornaram resultado ou BUSY. Testes unitários provam limite 32 e intervalo 60 segundos, com liberação após falhas. As contagens exatas de admissão dependem do agendamento e estão no JSON, sem alegação de taxa por segundo.
- **Artefatos:** fixtures temporárias demonstram recusa de database.db, .env, source map e JS órfão; build real contém só índice/assets esperados. Gitleaks e scanner complementar examinaram o texto disponível sem upload de código/bancos.

## Falhas encontradas e corrigidas durante a execução

1. Dois ensaios nativos demonstraram um pedido HTTP anterior ao cancelamento de navegação. Foi adicionado o filtro WebResourceRequested. A execução final terminou com exit 0, oito verificações e zero pedidos ao servidor local. [Reprodução anterior](evidencias/navigation-before-filter.json), [resultado final](evidencias/native-security.json).
2. Conexão inicial ao CDP ocorreu antes de a janela estar pronta; o harness ganhou espera limitada. Isso não foi contado como prova de falha do banco.
3. O servidor local de teste conservava conexão especulativa aberta apesar de não receber pedido HTTP. O encerramento foi corrigido com closeAllConnections; a execução final terminou normalmente.
4. Houve erro de tipo no wrapper generate_handler, avisos de Result no código de limite SQLite, falha de montagem de dependência no utilitário privado e uma regra de lint do harness. Todos foram corrigidos; checks finais recompilaram os componentes afetados. Nenhum desses utilitários ou estado incompleto foi apresentado como release aprovado.
5. Quatro .sql legítimos foram sinalizados pelo scanner por extensão; classificados como FP-01 e allowlist limitada ao diretório/versionamento de migrações.
6. A busca exata de heading por UIAutomation não encontrou o elemento na conferência do release. A janela foi então capturada com PrintWindow e inspecionada visualmente: dashboard normal, sem faixa de homologação. Não declarar essa conferência como aprovação de acessibilidade. A porta de inspeção fornecida no ambiente estava fechada, o cache WebView tinha ACL privada e o digest do banco permaneceu igual após a abertura.

## Não aplicável / não executado

| Item | Motivo |
|---|---|
| CSV, formula injection em exportação CSV | Não existe módulo CSV/exportação Excel nesta versão |
| HTTP 429/Retry-After/autenticação REST | Não existe API HTTP de estoque; o controle aplicável é IPC |
| Manifesto, assinatura inválida, download interrompido de atualização | Updater/infraestrutura ausentes. Não foi criado verificador demonstrativo para simular aprovação |
| SQLCipher/DPAPI/restauração de chave em Windows novo | Bloqueado pela decisão de recuperação; plano isolado documentado |
| CI remota, branches nunca obtidas | Workflow configurado; nenhum push/disparo/busca de infraestrutura privada autorizado/executado |
| Hardware HID, queda física de energia/disco, Windows limpo | Fora do hardware/ambiente disponível nesta auditoria |
| Exploração externa, fuzzing exaustivo | Não foram executados; não declarar cobertura total |

## Evidências

[Fluxo integrado](evidencias/native-flow.json), [bateria nativa](evidencias/native-security.json), [captura sintética](evidencias/native-security.png), [preservação real somente leitura](evidencias/primary-protection.json), [ACL](evidencias/primary-acl.json). Auditorias redigidas e metadados finais ficam nesta pasta; bancos e backups ficam somente na área privada ignorada.
