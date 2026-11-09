# Checklist de liberação

Data: 08/11/2026. `[x]` indica implementado/verificado no escopo; `[ ]` indica pendência, não um teste aprovado por inferência.

- [x] Estado Git/versionamento/dependências/testes iniciais registrados e checkpoint recuperável verificado.
- [x] Backup online consistente antes das alterações e antes da ACL real; cópia privada verificada sem gravar SQL na origem.
- [x] Diretório dedicado, DB, sidecars e backups conhecidos protegidos conforme modelo de conta Windows.
- [x] Caminhos de rede, aliases, traversal e links tratados; nenhum path arbitrário exposto por IPC.
- [x] Consultas parametrizadas; sem SQL arbitrário no frontend. Constraints/transações/revisões/idempotência/FEFO preservados.
- [x] SQLite defensivo/trusted_schema OFF/temporários em memória/limites configurados; WAL/FULL/FKs/backup e restauração sintética testados.
- [x] Nenhum segredo reconhecido por Gitleaks e scanner complementar; valores nunca incluídos em relatórios.
- [x] Hook local ativo, ignore ampliado e workflow de CI configurado sem push/execução remota.
- [x] CSP, renderização React e payloads XSS testados em testes de interface e WebView real.
- [x] ACL explícita para 26 comandos da janela main; APIs genéricas e chamadas inválidas rejeitadas.
- [x] Fila/concorrência/corpos/frequência de backup limitados sem taxa artificial para bipagem normal.
- [x] Navegação/popup e requisições fora da lista local bloqueados, com reprodução antes/depois.
- [x] XML com limites/XXE/entidades/depth; leitura UTF-8 estrita no frontend e validação independente no backend.
- [x] Dependências npm/Cargo auditadas; avisos de Linux separados do alvo Windows; nenhuma remoção/upgrade cego.
- [x] Frontend final validado contra arquivos privados, maps, links e assets órfãos; hashes registram identidade do build, não assinatura.
- [ ] Criptografia em repouso e recuperação de chave homologadas. Plano entregue; nenhuma ativação destrutiva.
- [ ] Assinatura Authenticode/instalador/distribuição Axis homologados; chaves/certificado dependem de infraestrutura externa.
- [ ] Atualizador com HTTPS/domínios/chave pública/assinatura/versão/recovery implementado e testado. Atualmente ausente e bloqueado pelo guard de configuração para ativação sem revisão.
- [ ] Avisos GTK/Linux resolvidos upstream antes de qualquer liberação para Linux.
- [ ] CI remota, scanner físico, Windows limpo e falha física de energia/disco homologados.
- [x] Riscos residuais e limites documentados; não declarar segurança absoluta, criptografia ou assinatura inexistentes.

Decisão: **APTO COM RESSALVAS**, restrito à homologação local Windows nas condições do [relatório](SECURITY_AUDIT.md).
