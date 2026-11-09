# Auditoria de segurança do Astock

Data: **08/11/2026**. Versão: **1.3.0**. Base Git: `23f89161cbdbf3754fafb6b44825d2e2b6938fb8` (main; árvore inicialmente limpa). O relatório descreve a árvore de trabalho corrigida, sem novo commit, push ou reescrita de histórico. Checkpoint inicial recuperável em `.cache/security-before/repository.bundle`; verificado com git bundle verify. Lockfiles e executável anterior preservados localmente.

## Decisão

**APTO COM RESSALVAS para homologação local em Windows, por um usuário confiável.** Isso não é garantia absoluta de segurança nem autorização para anunciar distribuição assinada ou criptografia que não existem. A distribuição pública deve aguardar assinatura/validação da origem e a definição de recuperação/aceitação do risco de dados em repouso.

Foram registrados **12 achados**: 8 corrigidos ou endurecidos, 2 riscos residuais do produto e 2 avisos informativos de dependências de Linux fora da árvore Windows. Não foi comprovada vulnerabilidade crítica ou alta explorável no escopo inspecionado. Um grupo adicional de quatro falsos positivos do scanner local corresponde às migrações SQL legítimas. [Classificação e reproduções](VULNERABILITIES.md).

| Severidade | Corrigidos | Pendentes | Contexto |
|---|---:|---:|---|
| Crítica / Alta | 0 | 0 | Nenhum achado confirmado nessas categorias |
| Média | 3 | 2 | Abuso IPC, ACL de dados, saída HTTP; criptografia e assinatura pendentes |
| Baixa | 4 | 0 | ACL declarativa, caminhos, codificação XML, barreiras de publicação |
| Informativa | 1 | 2 | Defaults SQLite endurecidos; glib/proc-macro-error somente Linux |

## Ambiente e arquitetura reais

Windows x64, PowerShell, Node 24.16.0, npm 11.13.0, Cargo 1.99.1, Git 2.53.0.windows.2; Tauri 2.12.5, React 19.3.1, Vite 8.3.6, rusqlite 0.41.1 e SQLite incorporado 3.54.1. Gitleaks 8.30.1 e cargo-audit 0.22.2 foram obtidos dos releases oficiais em `.cache/security-tools`, com checksum/digest SHA-256 conferidos; não foram adicionados ao aplicativo.

Há catálogo, fornecedores, recebimentos manuais/XML, lotes/FEFO, movimentos e inventário. **Não há CSV, API REST, servidor HTTP de produção, integração remota Axis ou atualizador automático.** Os endereços tauri.localhost/ipc.localhost são protocolos internos da WebView, não um servidor de estoque escutando TCP. O Vite só atende desenvolvimento em loopback. A interface e o SQLite continuam locais.

## Correções implementadas

- Admissão antes de spawn_blocking: até 32 operações pendentes por processo; uma importação XML e um backup simultâneos; intervalo de 60 segundos após backup bem-sucedido. Falhas liberam a vaga. Limites de corpo/estrutura IPC e seleções extensas complementam os limites anteriores do XML. Bipagem não tem limite artificial de leituras por segundo.
- AppManifest registra os 26 comandos e a capability concede somente os necessários à janela main, sem permissões remotas ou plugins genéricos. Argumentos extras, caminhos fornecidos ao comando de backup e comandos desconhecidos são rejeitados. Backend mantém validação, revisões e idempotência.
- Navegação e novas janelas restritas; filtro WebResourceRequested do WebView2 impede pedidos HTTP/HTTPS fora das origens internas e do Vite debug autorizado. A reprodução encontrou inicialmente um pedido HTTP mesmo com navegação cancelada; o filtro corrigiu o caso. Falha ao instalar o filtro encerra o aplicativo.
- CSP de produção sem unsafe-inline/unsafe-eval, com objetos, frames, formulários, workers e mídia externos bloqueados. A CSP foi exercitada no WebView, além da inspeção estática.
- ACL protegida no diretório dedicado, banco, sidecars e backups: usuário atual, SYSTEM e Administradores. O perfil WebView novo também fica sob essa pasta privada, tanto em release quanto debug. Links/junções, hard links, UNC, traversal, nomes reservados e alternate data streams são recusados. Destino de backup existente não é sobrescrito. Cópias diagnósticas locais conhecidas também receberam ACL restrita.
- SQLite com trusted_schema desabilitado, modo defensivo, temporários em memória e limites explícitos. WAL/FULL, integridade/FKs, transações e livro imutável preservados.
- Arquivo XML decodificado estritamente como UTF-8 antes do IPC, mantendo o limite de 8 MiB. Dados externos continuam sendo texto React, sem renderização HTML.
- Prefixo público explícito ASTOCK_PUBLIC_ no Vite, verificadores de configuração/artefato, scanner de segredos, hook local ativo e workflow CI com ações fixadas por digest. Nenhum banco, backup ou segredo foi incluído no bundle verificado.

## Preservação e provas

Antes de editar fontes: checkpoint Git, dependências, versão, testes e backup online do banco sintético. Antes de corrigir a ACL real: backup privado consistente, origem aberta somente leitura e comparação do conteúdo lógico de todas as tabelas. O digest antes/depois foi igual e total_changes da origem foi zero. O esquema continua 5; nenhuma migração de dados foi necessária nesta auditoria. [Prova de preservação](evidencias/primary-protection.json), [ACL](evidencias/primary-acl.json).

Baseline: 51 testes de interface e 32 Rust. Após correções: 57 de interface, 46 Rust e 3 verificadores Node aprovados; o helper de crash aparece ignorado na execução principal e é efetivamente executado em subprocesso pelo teste pai. O teste com 1000 SKUs/5000 movimentos passou. Houve regressão específica adicional após a inclusão de proteção ao rollback journal. Typecheck, lint, fmt, clippy e build foram executados. [Comandos, falhas corrigidas e limites](SECURITY_TESTS.md).

No Tauri real, em bancos explicitamente sintéticos: fluxo XML → 60 → FEFO −10 → inventário −2 → 48, reabertura, associação e backup; bateria de IPC/XSS/CSP/abuso com 16 importações concorrentes, 8 pedidos de backup e 100 consultas. A última bateria criou um só backup, não duplicou entrada e não enviou pedido HTTP ao servidor local bloqueado. [Fluxo](evidencias/native-flow.json), [segurança](evidencias/native-security.json).

## Dependências, segredos e distribuição

npm audit: zero vulnerabilidades reportadas. cargo-audit: zero vulnerabilidades classificadas pelo utilitário e dois avisos informativos; não interpretar isso como ausência absoluta de riscos. glib 0.18.5 e proc-macro-error 1.0.4 estão no lockfile multiplataforma, mas não na árvore Windows, inclusive build. Não houve atualização forçada ou remoção de biblioteca necessária. [Detalhes](DEPENDENCY_AUDIT.md).

Gitleaks em Git e arquivos atuais não encontrou vazamentos. O scanner adicional cobriu blobs locais, incluindo objetos sem caminho atual, bundles frontend e strings do PE. Nenhuma rotação foi indicada pelos resultados; segredos ofuscados, imagens e recursos comprimidos têm limitações explícitas. Não foi feita busca de branches ausentes no servidor. [Escopo de segredos](SECRETS_AUDIT.md).

O executável permanece sem assinatura Authenticode. Não existe updater para testar download/manifesto/assinatura inválida; o verificador impede ativá-lo silenciosamente na configuração. [Plano de criptografia](ENCRYPTION_PLAN.md), [checklist](SECURITY_CHECKLIST.md).

## Modelo de ameaça e limitações

ACL protege contra acesso de outros usuários não autorizados; não protege contra Administradores, malware no mesmo usuário, handles já abertos ou cópias anteriores. O banco e backups seguem em SQLite sem criptografia. Não foi habilitado BitLocker/EFS nem SQLCipher. Não há autenticação própria ou separação de operadores: a conta Windows é a fronteira atual.

O perfil antigo de cache WebView não foi apagado nem migrado automaticamente. Preferências de consulta guardadas nesse perfil podem precisar ser refeitas; cadastros, estoque, recebimentos, contagens e histórico são SQLite e permanecem no mesmo banco. Isso evita transportar caches antigos para a pasta privada sem revisão.

O filtro testado bloqueia requisições da página, não prova ausência de DNS, preconexões ou telemetria interna do runtime/Windows. O IPC tem admissão limitada após o recebimento/parse inicial do transporte Tauri; isso não promete um teto absoluto para memória da WebView comprometida. Leituras grandes não possuem deadline universal de SQL; existem busy timeout, limites de entrada, fila e limite de backup.

Não foram feitos pentest de infraestrutura externa, fuzzing exaustivo, teste físico de energia/disco/scanner, teste em Windows limpo ou execução da CI em servidor. Os testes nativos mutáveis usaram debug com assets incorporados e perfil próprio; release ignora o perfil de testes. Toda conclusão se limita ao código, versões e evidências registrados.
