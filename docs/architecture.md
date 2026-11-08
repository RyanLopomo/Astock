# Arquitetura integrada

O frontend React chama comandos Tauri tipados em `src/business-api.ts`. O navegador comum exibe indisponibilidade do banco, sem substituição por dados de exemplo. O shell `src-tauri/src/main.rs` mantém uma conexão protegida por mutex e despacha operações com `spawn_blocking`. A inicialização/migração ocorre em thread de trabalho; a interface trata o estado STARTING e tenta novamente.

`crates/axis-estoque-core` concentra todas as regras. `catalog` trata produtos/SKUs/fornecedores; `receipts` e `xml` preparam e confirmam compras; `stock` grava o livro; `lots` calcula FEFO/alertas/regularização; `inventory` persiste sessões/contagens e ajusta diferenças; `storage` abre, migra e copia o banco. Os módulos compartilham transações e identificadores. Não existem três estoques independentes.

O saldo é a soma de movimentos, incluindo o conjunto sem lote (`lot_id NULL`). Não existe campo de saldo editável. Recebimento confirmado, movimento, auditoria e evento de contagem são preservados. Correções de estoque geram novos movimentos com motivo e origem. SQL reforça chaves, unicidade, não negatividade e bloqueio do inventário.

Dinheiro usa centavos inteiros. Parsing decimal usa inteiro i128, escala até 10 e arredondamento half-up; a UI usa BigInt nos cálculos monetários. Quantidades de estoque são inteiras. Limites de negócio: 1 bilhão de unidades por quantidade/saldo e 9 trilhões de centavos por valor monetário. Erros de conversão/overflow impedem confirmar. O custo de lote é preservado; a estimativa do saldo não identificado usa o último custo cadastrado. Preço de venda só muda no cadastro explícito.

Datas de fabricação/validade são datas civis ISO, sem conversão UTC. Alertas usam a data local do computador/estabelecimento. Eventos usam UTC e são apresentados em hora local. O relógio e o fuso do Windows precisam estar corretos.

As telas estão separadas em `Products`, `Suppliers`, `Receipts`, `Inventory`, `Views` e componentes comuns. `WorkspaceApp` organiza dez páginas, carregamento e atualização. A prova anterior permanece em `ProbeApp`, acessível em Configurações. A janela começa 1200 × 900, permite redimensionar/maximizar e adapta navegação/formulários; tabelas largas rolam dentro de seus contêineres.

Testes de interface usam clientes injetados identificados como testes; a aplicação distribuída usa exclusivamente IPC real. O perfil debug isolado é explícito na janela e não existe como seletor de banco em release. O script `scripts/native-integration.mjs` recusa operações mutáveis fora desse perfil.
