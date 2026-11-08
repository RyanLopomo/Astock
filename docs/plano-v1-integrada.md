# Plano integrado — 07/11/2026

Escopo autorizado: pré-requisitos V1.0 e versões V1.1, V1.2 e V1.3 na mesma execução. Sem troca de arquitetura ou dados simulados em produção.

## Auditoria

Existe somente a prova da Fase 0: `probe_entries`, esquema 1, SQLite 3.54.1, dois comandos e seis testes React/cinco Rust. Produtos, recebimentos, saldos comerciais, lotes e inventário ainda não existem. A pasta não é um repositório Git.

## Etapas e arquivos

| Etapa | Implementação | Arquivos principais | Aceite |
|---|---|---|---|
| A | Migrações, backup online, catálogo, fornecedores, recebimentos e livro único | `crates/axis-estoque-core/src/{storage,model,catalog,stock,receipts}.rs`, migrações SQL | Preservação do esquema 1, backup íntegro, SKU único, estoque sem negativos e transações reais |
| B | XML NF-e 4.00, decimais exatos, associações persistidas | `xml.rs`, `decimal.rs`, importação e tela XML | Parser limitado, DTD bloqueado, chave validada, duplicatas e confirmação idempotentes |
| C | Lotes, distribuição, FEFO e alertas por data local | `lots.rs`, recebimentos e dashboard | Saldos total/por lote coerentes, sem datas inventadas, FEFO e tratamento explícito de desconhecidos |
| D | Inventário persistido, bloqueios, contagens e fechamento | `inventory.rs`, comandos Tauri assíncronos, telas React | Uma sessão ativa, estoque bloqueado no Rust/SQL, recuperação, fechamento atômico e idempotente |

As migrações serão estruturadas em versões 2–5, sem apagar `probe_entries`. O banco existente continuará no caminho atual para evitar uma troca silenciosa de base. SQLite Online Backup API e verificação de integridade serão usados antes de qualquer migração; as mesmas migrações serão ensaiadas numa cópia consistente antes de alterar o arquivo principal.

Cada etapa terá testes Rust de integração, atualização de evidências e checkpoint SHA-256 próprio. Ao final: tipos, lint, testes React, fmt, clippy, build desktop e verificação do banco real. A janela inicial passará a 1200×900, redimensionável. A identidade visual e o módulo de diagnóstico da Fase 0 serão preservados.

## Riscos e decisões iniciais

- A confirmação modifica estoque apenas em transação IMMEDIATE; o livro imutável é a origem de todos os saldos.
- Recebimentos e inventários terão revisão para impedir confirmação de uma conferência desatualizada.
- Dinheiro: centavos inteiros; quantidades XML: decimais de escala limitada e operações verificadas, sem float.
- Leitor físico e instalação em Windows limpo continuam dependentes de testes manuais; não serão apresentados como homologados.
- A janela antiga será fechada antes de substituir o executável/migrar a base. A preservação das leituras atuais será comparada antes/depois.
