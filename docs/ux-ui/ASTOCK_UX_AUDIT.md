# Auditoria UX/UI do Astock

Adendo de segurança, 08/11/2026: diagnóstico anterior registrado em docs/security/INITIAL_STATE.md. A única mudança no fluxo visual foi rejeitar XML malcodificado em UTF-8 antes do IPC, com erro recuperável pelo TaskStatus existente e sem anunciar importação bem-sucedida. Identidade, componentes, foco e conferência foram preservados. Testes XSS/UTF-8 e regressão de interface constam de docs/security/SECURITY_TESTS.md; não constitui nova avaliação de conformidade WCAG ou hardware.

Data: 08/11/2026. Base inspecionada: Astock 1.3.0 em `C:\Users\R\Documents\PDV`.

## Diagnóstico anterior às alterações

Esta seção registra a análise estática anterior à implementação. As referências de linha são da versão inicial, preservada no projeto de origem durante a preparação. Resultados posteriores e limites de homologação devem ser acrescentados sem transformar hipóteses em resultados.

O Astock é um aplicativo desktop Tauri 2 com React 19/TypeScript, Vite 8, Tailwind 4, núcleo Rust e SQLite local. Não há servidor obrigatório. `src/business-api.ts` define os comandos IPC; `crates/axis-estoque-core` concentra as regras. `src-tauri/src/main.rs` executa trabalho de banco fora da thread da interface. O saldo deriva do livro de movimentos. Dinheiro é expresso em centavos; o parsing decimal e as conversões já têm validações. XML, FEFO, lotes, recebimentos e inventário compartilham o mesmo núcleo.

Preservar: revisões, idempotência, trava de inventário, confirmação de movimentos, tratamento explícito de não contados, transações, histórico e caminho do banco. Códigos de barras compartilhados entre variantes são permitidos. Um aviso de ambiguidade não pode virar proibição de cadastro.

## Mapa real da interface

| Área | Arquivo | Fluxo e dependências |
|---|---|---|
| Inicialização e navegação | `src/WorkspaceApp.tsx`, `App.tsx` | Dez páginas, carregamento STARTING, atualização e faixa de homologação |
| Dashboard | `src/Views.tsx` | Indicadores derivados do workspace, atalhos para operações |
| Produtos | `src/Products.tsx` | Cadastro, variantes, códigos, busca; saídas e ajustes em conferência separada |
| Recebimentos | `src/Receipts.tsx` | Rascunho manual/XML, associação, embalagem, custos, lotes, confirmação |
| Importação XML | `src/Receipts.tsx` | Arquivo local com limite de 8 MiB; preservação da chave e dados de origem |
| Lotes e validades | `src/Views.tsx`, `Products.tsx` | Filtros, FEFO, saldo sem lote, regularização compensatória |
| Inventário | `src/Inventory.tsx` | Sessão persistida, leitura Enter, variantes/lotes, correção, fechamento |
| Movimentos e relatórios | `src/Views.tsx` | Histórico consultável, filtros, paginação e estimativas existentes |
| Fornecedores | `src/Suppliers.tsx` | Cadastro e consulta local |
| Configurações/prova anterior | `src/Views.tsx`, `ProbeApp.tsx` | Alertas, backup, histórico preservado da Fase 0 |
| Componentes compartilhados | `src/ui.tsx`, `styles.css`, `task.ts` | Botões, campos, diálogos, tabelas, seletor SKU, proteção de envio |

## Achados e priorização

P1: risco de perder trabalho ou interpretar incorretamente uma operação; P2: esforço recorrente ou barreira de acessibilidade; P3: clareza/manutenção. Não foi demonstrada corrupção de estoque nesta auditoria.

| ID | Prioridade | Problema e evidência inicial | Impacto | Solução recomendada | Status inicial |
|---|---|---|---|---|---|
| UX-01 | P1 | `Receipts.tsx:32-33` desmonta `ReceiptItemEditor` ao abrir `quick`; seus estados locais estão em `:39` | Quantidade, custo, conversão e lotes não salvos desaparecem mesmo ao cancelar o cadastro rápido | Preservar o editor e suspender sua interação/foco enquanto o cadastro estiver aberto | Confirmado em código |
| UX-02 | P1 | `ui.tsx:18-33` fecha diálogos por Escape/× sem consultar alterações | Perda acidental de preenchimento | Guarda de descarte contextual nos formulários, sem confirmação em formulário intacto | Confirmado em código |
| UX-03 | P2 | `Inventory.tsx:20` foca antes de renderizar o campo de `:35` | A primeira leitura pode não ir para o campo correto | Focar quando o campo é montado; não capturar foco durante digitação em outros campos | Confirmado em código |
| UX-04 | P2 | `ui.tsx:38-40` filtra SKU sem tratar Enter do scanner | Mais seleção manual e possível tentativa de submeter formulário | Selecionar apenas código exato único com Enter; interromper submissão; ambiguidades exigem escolha | Confirmado em código |
| UX-05 | P2 | `Products.tsx:17` mantém filtros apenas no componente; shell desmonta páginas | É necessário repetir busca e categoria ao voltar | Preservar consulta na sessão, com validação e fallback se armazenamento indisponível | Confirmado em código |
| UX-06 | P2 | `Products.tsx:13` oferece textarea de códigos sem aviso de associação existente | Ambiguidade aparece somente em operações seguintes | Entrada sequencial por leitor e aviso informativo dos SKUs compartilhados, preservando zeros | Confirmado em código |
| UX-07 | P2 | Cores/medidas literais e definições duplicadas em `styles.css`; campos usam borda `#94a3b8` e fundo branco | Consistência e contraste de controles precisam de medição; difícil manter tema | Consolidar tokens da identidade verde existente, medir contraste, foco e alvos | Risco estático, verificar no navegador |
| UX-08 | P2 | `Field` em `ui.tsx:12` renderiza hint sem associação `aria-describedby` | Instruções podem não ser anunciadas com o campo | IDs e associação acessível; erro contextual próximo ao controle | Confirmado em código |
| UX-09 | P3 | `Products.tsx:19` usa o mesmo vazio para catálogo e pesquisa | Orienta cadastrar o primeiro produto mesmo com catálogo existente | Vazio por contexto e ação de limpar filtros | Confirmado em código |
| UX-10 | P3 | `task.ts:11` normaliza candidato e consulta a cada comparação | Possível custo proporcional ao catálogo | Medir primeiro, otimizar somente gargalo comprovado, sem mudar SQLite | Hipótese de desempenho |
| UX-11 | P2 | Modal possui trap global e seletor de controles que ignora links/elementos ocultos; não existe modo suspenso | Cadastro rápido exige tratamento de diálogo ativo e retorno de foco | Um diálogo ativo por vez; ocultar/inert o anterior sem perder estado; testar Tab/Shift+Tab/Escape | Confirmado em código |

## Estado funcional e limites anteriores

Documentação existente relata entregas V1.1 XML, V1.2 lotes e V1.3 inventário, com testes anteriores. Esses resultados históricos não são testes executados nesta alteração. Hardware HID/ABNT2, XMLs reais, queda física de energia, Windows limpo e distribuição assinada já são pendências de homologação.

O projeto de origem não contém `.git`. Preservar checkpoints históricos, sem commits automáticos. Preparação em cópia isolada dentro da raiz gravável; não copiar bancos, `target`, `.cache/test-data` ou dependências para distribuição.

## Ferramentas

Catálogo de ferramentas e skills desta sessão verificado: shadcn/ui MCP, 21st/Magic MCP, Frontend Design Skill, Web Design Guidelines e Chrome DevTools MCP não estão disponíveis. Nenhuma delas foi instalada ou anunciada como usada. Recursos disponíveis: PowerShell, ripgrep, TypeScript/ESLint/Vitest/Vite existentes, Playwright já instalado em outro projeto local e Chrome. O teste de navegador deve identificar esse caminho externo e permitir resolução/configuração explícita sem adicionar dependência de produção.

Referência de acessibilidade: [WCAG 2.2, W3C](https://www.w3.org/TR/WCAG22/). Inspeção estática, testes DOM e navegador não equivalem a certificação WCAG ou teste com leitor de tela real.

## Checkpoints desta alteração

1. Este diagnóstico foi salvo antes de modificar fontes da interface.
2. Baseline: executar verificações existentes, capturar navegador com dados sintéticos identificados e medir os mesmos cenários antes/depois.
3. Fundação: tokens semânticos, componentes acessíveis e descarte seguro; validar.
4. Operação: preservar recebimento, foco/leitor e filtros; validar comportamentos críticos.
5. Encerramento: registrar testes, medidas reais, arquivos e limitações no changelog.

## Resultado do primeiro ciclo

| Achado | Resultado | Evidência |
|---|---|---|
| UX-01 | Corrigido: editor preservado e suspenso durante cadastro rápido; novo SKU não redefine demais campos | `src/ux.test.tsx`: cancelar e salvar cadastro rápido preservando quantidade, conversão, preço, total e lote |
| UX-02 | Corrigido nos diálogos de produto, fornecedor, item/novo recebimento, estoque, regularização, nova sessão e correção de contagem | Escape/Cancelar/×, formulário intacto e beforeunload testados; fechamento da janela nativa ainda precisa de homologação |
| UX-03 | Corrigido: foco na montagem efetiva do leitor | Teste DOM e Chrome; leitor físico não testado |
| UX-04 | Corrigido: Enter resolve código exato único sem salvar, limpa seleção anterior em código desconhecido/ambíguo | Testes de seleção única, ambiguidade e código desconhecido; Chrome |
| UX-05 | Corrigido para busca/categoria/inativos/página de Produtos, com fallback sem storage | Testes ao navegar e armazenamento bloqueado; demais módulos ficam para próximo ciclo |
| UX-06 | Corrigido: inclusão sequencial de códigos, zeros/deduplicação local, aviso de compartilhamento permitido | Testes Enter, código pendente sem adicionar e cadastro com código compartilhado |
| UX-07 | Fundação consolidada: tokens verdes existentes, controles de 36 px ou mais, foco e movimento reduzido | 11 pares de contraste verificados em `evidencias/contrast.json`; borda de campo 2,56:1 → 5,15:1 |
| UX-08 | Corrigido: label textual separado da ajuda, IDs e descrição acessível | Teste `aria-describedby`; obrigatórios identificados sem contaminar nome acessível |
| UX-09 | Corrigido: vazio contextual e limpar filtros | Teste de catálogo com pesquisa sem resultado |
| UX-10 | Não alterado: não foi comprovado gargalo no cenário de 1.000 SKUs | Baseline e resultado de busca em `evidencias/before.json` e `after.json`; sem mudança no banco |
| UX-11 | Corrigido: apenas modal ativo participa de foco/teclado; editor suspenso fica oculto/inert | Testes de Tab/Shift+Tab incluindo links, descarte e cadastro rápido; capturas Chrome |

Achado adicional UX-12 (P1): gravação confirmada seguida de erro na atualização poderia incentivar repetir o cadastro. Produto, fornecedor, item, novo recebimento, novo inventário, saída, ajuste e regularização agora conservam a confirmação, bloqueiam edição e repetem somente a atualização. A revisão também corrigiu a guarda de descarte nas movimentações já aplicadas: a janela informa que a operação foi registrada e pode ser fechada, sem sugerir que descartar o formulário desfaz o estoque. Os testes cobrem uma única escrita após duas tentativas de atualização. Isso não cria garantia de idempotência para respostas incertas sem confirmação recebida; os contratos existentes foram preservados.

Foram executados 51 testes de interface e 32 testes do núcleo Rust/SQLite temporário. As dez páginas foram verificadas no Chrome em 1200/760/320 CSS px, além de teclado e formulários. Os estados finais de saída/ajuste/regularização após falha de atualização têm cobertura comportamental DOM adicional. A emulação de 320 px é um teste de reflow, não alteração do mínimo de 760 × 640 da janela Tauri.

Não houve estudo com operadores reais, leitor físico, leitor de tela real, desligamento de rede/energia ou operação mutável no banco principal. Testes com cliente sintético no navegador não equivalem a validar IPC/WebView2 nativo. Para esta entrega, o núcleo e contratos nativos permanecem idênticos; a homologação nativa das interações novas continua recomendada.

## Próximo ciclo

1. Homologar leitor HID/ABNT2 e fechamento da janela com formulário modificado em perfil debug isolado; validar NVDA/JAWS e zoom de 200% no WebView2.
2. Observar tarefas reais de cadastrar, localizar e receber para medir esforço, sem assumir que tempos sintéticos representam o estoque da operação.
3. Estender preferências de consulta para lotes/recebimentos/movimentos após confirmar uso; manter drafts no SQLite.
4. Mapear erros monetários e de quantidade diretamente aos campos, aproveitando `Field.error`, sem trocar os parsers do núcleo.
5. Avaliar idempotência de criação com resposta incerta apenas com diagnóstico e decisão arquitetural próprios; não adicionar retries automáticos de escrita nesta interface.
