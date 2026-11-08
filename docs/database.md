# Banco de dados e preservação

O arquivo principal continua `%LOCALAPPDATA%\AxisEstoque\phase0\prova.db`, com application_id `0x41585030`. Não há recriação do banco antigo. O esquema atual é `PRAGMA user_version=5`. `probe_entries` e seus registros permanecem; o histórico não vira estoque comercial automaticamente.

| Versão | Migração | Estruturas |
|---|---|---|
| 1 | Fase 0 existente | probe_entries |
| 2 | 002_base.sql | categories, products, skus, barcodes, suppliers, receipts, receipt_items, stock_movements, stock_operations, operation_requests, audit_events, app_settings |
| 3 | 003_xml.sql | purchase_xml_imports, supplier_product_mappings; metadados originais nos itens |
| 4 | 004_lots.sql | lots, receipt_item_lots; lot_id no livro, proteção por lote |
| 5 | 005_inventory.sql | inventories, inventory_skus, inventory_lines, inventory_events; bloqueios e histórico de valores antes/depois |

São 21 tabelas de aplicação no total. Scripts em `crates/axis-estoque-core/migrations`. Tabelas novas são STRICT; consultas recebem parâmetros; índices cobrem códigos, categoria, SKU, datas, lotes, associações e sessões. Código de barras é texto e pode apontar para mais de um SKU; a escolha precisa ser explícita.

## Migração e backup

1. Verificar identidade, versão e integridade; bancos desconhecidos/futuros são recusados antes de alterações persistentes.
2. Para banco existente, gerar backup consistente pela API online do SQLite, incluindo dados confirmados no WAL; verificar integridade e foreign keys.
3. Ensaiar as mesmas migrações numa cópia consistente temporária e verificar o resultado.
4. Aplicar todas as migrações pendentes em transação IMMEDIATE. Falhas fazem rollback; a cópia de backup continua disponível.

Backups ficam no diretório `backups` ao lado do banco, com nomes exclusivos. Configurações oferece backup manual pelo mesmo mecanismo. Copiar apenas o `.db` enquanto o aplicativo está aberto não é o procedimento recomendado, pois pode omitir o WAL. Para recuperação manual, feche todas as instâncias, guarde o banco atual e seus arquivos auxiliares e restaure um conjunto consistente com suporte técnico. Não há botão de restauração destrutiva nesta versão.

Conexão: SQLite incorporado, WAL, synchronous=FULL, foreign_keys=ON, espera de bloqueio 5 segundos. Comandos de gravação usam transações. Entrada por recebimento e fechamento de inventário confirmam integralmente ou desfazem integralmente. A API de backup tem tempo limitado para não ficar esperando indefinidamente.

## Invariantes

- Livro imutável, saldo total e por lote não negativo; lote deve pertencer ao SKU.
- Chave de NF-e única; confirmação de recebimento repetida não duplica movimentos.
- Chaves de operação persistidas com payload: repetir a mesma solicitação é seguro; reutilizar a chave com conteúdo diferente é erro.
- Apenas um inventário ativo; gatilhos bloqueiam movimentos dos SKUs abrangidos, inclusive por outra conexão.
- Fechamento usa revisão e estado transacional `closing`; somente os ajustes da própria sessão podem passar pelo bloqueio.
- Lotes e datas não são fabricados para saldos antigos. A regularização gera dois movimentos compensatórios sem mudar o total.

Os testes abrem bancos temporários reais e injetam falhas de gatilho para verificar rollback de múltiplos itens/lotes e do fechamento. O cenário de escala cria 1000 SKUs e 5000 movimentos pela API de negócio.
