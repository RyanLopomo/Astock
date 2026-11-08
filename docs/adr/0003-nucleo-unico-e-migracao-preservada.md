# ADR 0003 — núcleo único e preservação do banco

Data: 07/11/2026. Aprovada pela implementação e testes da Etapa A.

O projeto contém somente o esquema 1 da prova. A evolução mantém o arquivo existente em `%LOCALAPPDATA%\AxisEstoque\phase0\prova.db`, seu application_id e a tabela `probe_entries`. Não há troca silenciosa para um arquivo vazio em outro caminho.

Migrações SQL 2–5 acrescentam catálogo/estoque, XML, lotes e inventário. Antes de migrar um arquivo existente, a SQLite Online Backup API gera um backup verificado e uma cópia de ensaio; as migrações são aplicadas primeiro nessa cópia. Só depois são aplicadas atomicamente ao principal. Esquema futuro ou banco estranho é recusado antes de alterações persistentes. O backup íntegro permanece no subdiretório `backups` do banco.

`Store` é o repositório e fronteira de casos de uso. O núcleo não depende de Tauri. Recebimentos, baixas FEFO, regularizações e ajustes por inventário usam a mesma tabela imutável `stock_movements`; o saldo é a soma desses movimentos. `lot_id=NULL` representa explicitamente saldo sem lote identificado. Uma migração nunca inventa lote ou data de validade.

Transações IMMEDIATE, constraints STRICT, FKs, índices e triggers protegem saldo total, saldo do lote, titularidade do lote, imutabilidade e bloqueio de inventário. IDs de requisição persistidos impedem repetição de saídas/ajustes/contagens; reutilizar um ID com outro conteúdo é erro. Confirmações de recebimento e inventário usam o ID da própria operação como identidade e revisão otimista para a conferência.

Nas operações de banco demoradas, a ponte nativa deverá usar tarefas bloqueantes fora da thread da interface. Dados sintéticos ficam apenas em testes com arquivos temporários. O frontend não recebe uma capacidade de SQL arbitrário.
