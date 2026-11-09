# Achados, classificação e validação

Data: 08/11/2026. Referências de linha da árvore corrigida; o estado anterior está no bundle recuperável. Severidade considera as condições de exploração, não apenas a existência de um padrão. Itens preventivos não são apresentados como cadeias de exploração comprovadas.

| ID | Achado | Severidade | Status |
|---|---|---|---|
| SEC-01 | Fila IPC e backups sem admissão/frequência limitadas | Média | Corrigida |
| SEC-02 | Comandos próprios fora da ACL declarativa | Baixa | Corrigida |
| SEC-03 | Permissões herdadas adicionais em dados/backups | Média | Corrigida |
| SEC-04 | Aliases de arquivo/diretório sem rejeição explícita | Baixa | Corrigida |
| SEC-05 | Cancelamento de navegação ainda permitia pedido HTTP | Média | Corrigida |
| SEC-06 | Substituição silenciosa de bytes XML inválidos | Baixa | Corrigida |
| SEC-07 | Defaults SQLite sem defesa adicional de schema/limites | Informativa | Corrigida |
| SEC-08 | Ausência de barreira automatizada de segredos/artefatos | Baixa | Corrigida |
| SEC-09 | Banco/backups sem criptografia em repouso | Média | Pendente |
| SEC-10 | Artefato sem assinatura e distribuição ainda não homologada | Média | Pendente |
| SEC-11 | glib 0.18.5: RUSTSEC-2024-0429 | Informativa no alvo Windows | Pendente upstream; fora do alvo |
| SEC-12 | proc-macro-error 1.0.4: RUSTSEC-2024-0370 | Informativa no alvo Windows | Pendente upstream; fora do alvo |

## SEC-01 — Consumo de recursos

CWE-770. `src-tauri/src/main.rs:36`, `src-tauri/src/ipc_security.rs:63,128`; limites complementares em inventory.rs/stock.rs. Antes, cada chamada criava tarefa bloqueante aguardando o mesmo mutex e cada backup podia gerar outra cópia. Condição: script executando na WebView autorizada ou abuso local; não há endpoint HTTP público. Impacto: pressão de memória/threads, fila prolongada e disco ocupado por cópias.

Correção: admissão de 32 pendências antes de spawn_blocking, exclusão por operação pesada, intervalo de backup, limite estrutural de JSON e listas. Testes: bounded_queue_and_heavy_operations, failures_release_permits_without_backup_cooldown e bateria nativa. Oito pedidos nativos geraram uma cópia; 16 imports preservaram uma chave/rascunho. Residual: limites por processo não autenticam um usuário malicioso; parse inicial do transporte e memória do renderer continuam fora da admissão.

## SEC-02 — Escopo de comandos

CWE-862, defesa preventiva. `src-tauri/build.rs`, `src-tauri/commands.rs`, `src-tauri/capabilities/main.json`, `src-tauri/src/main.rs:249`. Comandos próprios não são automaticamente restringidos pela capability vazia. Não foi encontrada página remota habilitada nem uma segunda janela explorável no produto anterior; o risco é exposição indevida a outro contexto se introduzido. [Modelo oficial de capabilities](https://v2.tauri.app/security/capabilities/).

Correção: 26 comandos gerados no AppManifest, permissões individuais somente main, validação de origem/rótulo/payload, sem plugin genérico. Testes rejeitam outro rótulo/origem, execute_sql, paths de backup, IDs inválidos e tentativa nativa de criar WebView sem permissão. Residual: JavaScript confiável da janela main precisa dos comandos de negócio; isso não constitui autenticação entre operadores.

## SEC-03 — ACL de dados

CWE-200. `crates/axis-estoque-core/src/security.rs:115` e inicialização nativa. Foi observada leitura herdada para grupo local adicional no diretório do banco; não foi comprovado compartilhamento público/HTTP. Condição: membro desse grupo ou outra configuração herdada permissiva. Impacto: leitura dos dados locais.

Correção: DACL protegida para usuário atual, SYSTEM e Administradores; aplicação também aos arquivos existentes/sidecars/backups diretos. Backup privado anterior à correção, origem somente leitura e digest lógico idêntico. ACL final com três entradas foi inspecionada no banco real; cópias diagnósticas conhecidas receberam proteção equivalente. Residual: não revoga handles/cópias anteriores e não bloqueia Administradores ou malware da mesma conta.

## SEC-04 — Caminhos e links

CWE-59, defesa preventiva. `security.rs:17`, `storage.rs:28,225`. Caminhos não vêm do frontend, mas arquivos/ancestrais locais podiam ser substituídos por links/junções ou apontar para armazenamento de rede. Não foi comprovado ataque por outro usuário na ACL original; manipulação requer escrita no local ou configuração ambiental pelo usuário.

Correção: caminho absoluto local, rejeição de traversal/UNC/dispositivos/ADS/nomes reservados, checagem de reparse points e hard links, NOFOLLOW no SQLite, proteção dos sidecars e recusa de backup sobre destino com dados. Testes criam junção e hard link sintéticos, verificam erro antes de criar/alterar o arquivo externo e preservam seus bytes. Residual: checagens não prometem imunidade a corrida por código com os mesmos privilégios ou administração do sistema.

## SEC-05 — Rede antes do cancelamento

CWE-200. `src-tauri/src/network_guard.rs:3,13` e main.rs. Reproduzido em dois perfis sintéticos: navegação cancelada, mas servidor HTTP em loopback registrou um pedido. Não houve vazamento real nem XSS comprovado; o teste precisou iniciar a navegação programaticamente. Um script comprometido poderia tentar colocar informação em um URL.

Correção: filtro WebResourceRequested gera resposta local 403 fora das origens internas permitidas; cancelamento de navegação e negação de novas janelas permanecem. A falha de instalação encerra o app. Repetição final: zero pedidos HTTP, URL local preservado e IPC funcionando. [Antes](evidencias/navigation-before-filter.json), [depois](evidencias/native-security.json). Residual: não é um firewall do Windows nem controle de telemetria/preconexões do motor.

## SEC-06 — UTF-8

CWE-20. `src/xml-file.ts:3`, `src/Receipts.tsx`. File.text usa decodificação que pode substituir bytes inválidos, alterando descrições sem aviso. Impacto: fidelidade do documento importado; não foi demonstrada alteração automática de saldo por esse caso.

Correção: leitura limitada e TextDecoder fatal para UTF-8 antes do IPC. Testes aceitam acentos e rejeitam sequência inválida/tamanho excessivo sem chamar importXml. O parser Rust continua rejeitando DTD, entidades externas/desconhecidas, excesso de profundidade/eventos e layouts não suportados. Residual: a importação não verifica autenticidade fiscal.

## SEC-07 — Defesa SQLite

CWE-693, observação preventiva. `security.rs:142`. Não há SQL arbitrário ou funções SQL externas registradas que demonstrem exploração dos defaults anteriores. Foram habilitados modo defensivo, trusted_schema OFF, temporários em memória e limites de SQL/valores/recursão/parâmetros/ATTACH. Testes recusam ATTACH, load_extension e alteração de sqlite_schema e preservam os fluxos de negócio. Residual: integridade estrutural não detecta toda alteração comercial feita por quem já controla o arquivo/usuário. [Orientações oficiais SQLite](https://www.sqlite.org/security.html).

## SEC-08 — Prevenção de publicação

CWE-200, defesa preventiva. `.gitignore`, scripts/security-*.mjs, `.githooks/pre-commit`, `.github/workflows/security.yml`, `vite.config.ts:8`. Não foram encontrados segredos reais no build; havia falta de gates e assets antigos no diretório dist inicial. Foram adicionadas extensões sensíveis, scanner redigido, hook ativo, CI e verificador que recusa bancos, env, maps, links e assets órfãos. VITE_* deixou de ser o prefixo público; ASTOCK_PUBLIC_* continua sendo público, nunca um cofre. Fixtures do verificador provam rejeição sem acessar dados reais. Residual: commit com --no-verify ou alteração do próprio pipeline pode contornar controles; revisar PRs e permissões do servidor continua necessário.

## SEC-09 — Criptografia pendente

CWE-311. SQLite e backups estão sem criptografia. ACL reduz acesso por outras contas, mas não cobre leitura offline do disco, Administradores ou mesma conta comprometida. Não foi criada chave fixa nem migração arriscada. [Plano SQLCipher/DPAPI e recuperação](ENCRYPTION_PLAN.md). Decisão pendente sobre titularidade/exportação/recuperação de chaves e homologação de desempenho/restauração.

## SEC-10 — Origem do executável

Risco de distribuição, sem updater vulnerável implementado. Não há assinatura Authenticode configurada nem instalador/infraestrutura de atualização homologados. Substituição de um download/executável não assinado dificulta validar a origem Axis. Hash do próprio canal não substitui assinatura.

Pendente: certificado/processo de assinatura, custódia da chave privada e liberação controlada. O guard de configuração impede adicionar updater sem revisão. Chave pública de verificação pode ser distribuída; privada deve permanecer fora do repositório/build. Manifesto/assinatura inválidos não foram testados porque esse fluxo não existe.

## SEC-11 / SEC-12 — Avisos de dependências

glib: problema de soundness corrigido em >=0.20.0; proc-macro-error: sem manutenção/versão corrigida. Ambos ausentes de cargo tree para x86_64-pc-windows-msvc, inclusive build. Não foram forçados upgrades incompatíveis na cadeia GTK/Tauri. Linux não está liberado por esta auditoria. [Detalhes](DEPENDENCY_AUDIT.md).

## Falso positivo FP-01

O scanner local inicial classificou as quatro migrações .sql por extensão. Revisão confirmou somente DDL e gatilhos, sem dump/credenciais. A exceção foi limitada ao caminho/versionamento dessas migrações; outros .sql continuam bloqueados. Status: **Falso positivo**. Nenhum segredo foi inserido no relatório.
