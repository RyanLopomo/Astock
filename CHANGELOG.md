# Changelog

## Hardening de segurança — 08/11/2026

- ACL protegida para dados/sidecars/backups; rejeição de links, junções, hard links e caminhos inseguros; backup não sobrescreve destino com dados.
- Comandos Tauri com AppManifest/capability explícitos, origem/argumentos limitados, fila de 32 e proteção de importação/backup contra abuso.
- Cancelamento de navegação complementado por filtro de requisições WebView2; CSP reforçada e flags de inspeção removidas no release.
- SQLite defensivo, trusted_schema OFF, limites e temporários em memória; XML com UTF-8 estrito antes do IPC.
- Scanner/hook/CI e verificador de artefatos; testes de injeção, recuperação, XSS, rede e concorrência. Relatórios em docs/security; criptografia/assinatura permanecem pendentes, sem migração destrutiva ou atualização cega.

## Nome Astock — 07/11/2026

- Nome exibido alterado para Astock na janela, interface e metadados Windows; novo executável `astock.exe`.
- Identificador interno, banco local e histórico mantidos no mesmo caminho.

## 1.3.0 integrada — 07/11/2026

- Base real de produtos/SKUs, códigos, fornecedores, recebimentos manuais e livro de movimentações imutável.
- V1.1: XML NF-e 4.00 seguro, chave/duplicidade, associação persistente por fornecedor, conversão e custos decimais exatos; entrada somente na confirmação atômica.
- V1.2: distribuição por lotes, fabricação/validade/safra opcionais, FEFO, alertas configuráveis e regularização explícita de saldo sem lote.
- V1.3: inventário completo/parcial persistente, leitura com Enter, escolha de variante/lote, correções auditáveis e fechamento idempotente com decisão sobre não contados.
- Migrações 2–5 com backup online e ensaio em cópia; histórico da Fase 0 mantido. Banco inicializado fora da thread da interface.
- Sidebar integrada, janela inicial 1200 × 900 redimensionável, filtros/paginação e perfil debug de homologação separado.
- Testes SQLite/React, cenário com 1000 SKUs/5000 movimentos e roteiro nativo documentados no relatório integrado. Homologação física permanece pendente.

## Validação nativa — 06/11/2026

- Rust/Cargo, MSVC e Windows SDK instalados; Cargo.lock gerado e formatação Rust conferida.
- Cinco testes Rust/SQLite aprovados e aplicativo Tauri de desenvolvimento compilado e aberto.
- Clippy aprovado no workspace e em todos os targets, sem avisos.
- SQLite 3.54.1, WAL/FULL e quick_check OK confirmados na própria janela; captura registrada.
- Homologação física do scanner, release offline e instalação limpa permanecem pendentes.

## 0.0.1 — 05/11/2026

- Prova limitada à Fase 0: React/TypeScript/Vite/Tailwind e shell Tauri 2.
- Núcleo Rust independente com SQLite incorporado, inicialização atômica, gravação/leitura parametrizada, identificação de banco e configuração de durabilidade.
- Campo para teclado HID com Enter, foco explícito, feedback, prevenção de envio concorrente e histórico das últimas 20 leituras.
- Testes de interface e cenários Rust em arquivos temporários, inventário do ambiente, ADR, roteiro físico e checkpoint de arquivos.
- Compilação nativa, testes Rust, leitor físico, operação desktop offline e instalação limpa pendentes de homologação.
