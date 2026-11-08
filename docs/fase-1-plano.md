# Fase 1 — planejamento, sem implementação

**Pré-condição:** concluir os portões pendentes da Fase 0. Não há autorização nesta execução para avançar.

## Objetivo

Estabelecer o esquema comercial, migrações seguras e o núcleo de domínio independente da interface.

## Arquivos previstos

- `crates/axis-estoque-core/src/domain/`: tipos de centavos e quantidades inteiras, identificadores e estados válidos.
- `crates/axis-estoque-core/src/repository/`: contratos pequenos orientados a casos de uso reais, sem abstração genérica sem uso.
- `crates/axis-estoque-core/src/persistence/` e `migrations/`: conexões, esquema, índices e migrações versionadas.
- Módulo mínimo de backup pré-migração com Online Backup API e verificação de integridade; operação/configuração completa de backups na Fase 6.
- `src-tauri/src/commands/`: erros estáveis e comandos tipados, sem SQL recebido da interface.
- `docs/adr/`, testes de integração e fixtures: decisões e evidências.

## Trabalho previsto

1. Fechar contrato de esquema e compatibilidade: Product, SKU, Barcode, Category, Supplier, Receipt/Item, StockMovement, StockAdjustment, AuditEvent e AppSettings.
2. Incluir constraints, FKs e índices. Barcode como texto com vínculo muitos-para-muitos compatível com variantes; SKU interno único; valores monetários em centavos inteiros; quantidades inteiras.
3. Definir o livro de movimentos imutável como origem do saldo; impedir edição/exclusão e estoque negativo por padrão. Nenhum campo editável de quantidade em produtos.
4. Criar o banco em `%LOCALAPPDATA%\AxisEstoque\data\estoque.db`, separado da prova; rejeitar esquema mais novo antes de qualquer escrita.
5. Implementar backup consistente obrigatório e validado antes de aplicar migração em banco existente. Migrações em transação, mantendo cópia recuperável. Não copiar arquivos abertos com WAL.
6. Definir referência de origem/idempotência e auditoria na mesma transação para futuros recebimentos e ajustes.
7. Reservar evolução para lote/validade sem FEFO, sem implementar as fases de cadastro ou recebimento.

## Riscos

- Compatibilidade com upgrades/downgrades: proibir escrita por aplicação incompatível.
- Falha de migração, disco cheio e queda de energia: testar rollback e recuperação em banco temporário antes de tocar banco principal.
- Concorrência de instâncias/processos: resolver com transações e constraints, nunca apenas com botão desabilitado.
- Unicidade de origem e unidades: documentar antes de criar livro comercial.

## Aceite

- Testes Rust unitários e integração aprovados; `fmt`, `clippy`, frontend lint e tipos sem falhas.
- Migração de banco vazio e de versão anterior, falha injetada, backup íntegro e recusa de versão futura comprovados.
- Transações, constraints e imutabilidade verificados por testes reais em arquivo temporário.
- README, CHANGELOG, ADR e checkpoint atualizados com comandos/resultados e pendências.
- Nenhuma meta de 1.000 SKUs ou desempenho declarada sem benchmark; preparar a massa para fases e medições correspondentes.
