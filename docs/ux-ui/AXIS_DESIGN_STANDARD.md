# Axis Design Standard v1.0

Data: 08/11/2026. Referência de UX/UI para o Astock e para futuras adoções nos softwares Axis. A adoção em outros produtos exige diagnóstico próprio; este documento não autoriza sua remodelação automática.

## Princípios oficiais

1. Eficiência: aproximar ação e contexto, reduzir repetição, permitir teclado e leitura sequencial. Conservar etapas que previnem erros relevantes.
2. Clareza: uma ação principal por tarefa, informação essencial primeiro, opções avançadas progressivas, erro recuperável e conclusão verificável.
3. Consistência: reutilizar componentes e tokens existentes, com os mesmos estados e termos para a mesma operação.
4. Desempenho: responder visualmente ao início da operação e medir tempos reais. 400 ms é uma referência perceptiva, não garantia de conclusão ou timeout.
5. Segurança operacional: validar entrada, impedir envio simultâneo, aguardar confirmação real, manter dados após falha e preservar idempotência/revisões do núcleo.
6. Identidade: preservar marca e paleta do produto. No Astock, verde existente, superfícies neutras, tipografia local e geometria discreta. Sem dependências de rede para fontes e operações essenciais.

Não usar gradiente roxo genérico, botões em pílula, animação de cursor, emojis como ícones, fotos artificiais como reais, métricas fictícias, marketing vago, selos decorativos ou travessões nos textos de interface novos.

## Fundação visual e tokens

Os tokens são contratos semânticos. Os valores devem vir da identidade inspecionada no produto; a base verde abaixo pertence ao Astock e não substitui a paleta institucional de outros aplicativos.

| Grupo | Tokens | Regra |
|---|---|---|
| Superfícies | `--axis-bg`, `--axis-surface`, `--axis-surface-muted` | Fundo, painel e superfície de apoio |
| Conteúdo | `--axis-text`, `--axis-text-muted` | Texto principal/secundário legíveis; não reduzir legibilidade para criar hierarquia |
| Marca/ação | `--axis-brand`, `--axis-primary`, `--axis-primary-hover`, `--axis-on-primary` | Preservar verde `#102f28`/`#1b5342` existente |
| Controles | `--axis-border`, `--axis-control-border`, `--axis-focus` | Separar borda decorativa de limite perceptível do controle |
| Estados | `--axis-success-*`, `--axis-warning-*`, `--axis-danger-*` | Fundo, texto e borda; acompanhar cor com texto/ícone acessível |
| Medidas | `--axis-space-*`, `--axis-radius-*`, `--axis-control-height` | Escala curta de espaçamento e cantos discretos; mínimo operacional de 36 px em botões |
| Tipografia | `--axis-font`, `--axis-font-code` | Segoe UI/Arial/system-ui e Consolas/monospace; números tabulares em valores |

Temas futuros devem substituir valores destes tokens, sem introduzir um segundo conjunto de componentes ou cores fixas nos componentes. O Astock não possuía seletor de tema; não criar um tema escuro incompleto apenas para demonstrar os tokens.

Espaçamento: 4, 8, 12, 16, 24 e 32 px; preservar exceções operacionais justificadas. Raio: 4/8/12 px. Sombras apenas em sobreposições; painéis usam bordas discretas. Animações curtas e dispensáveis, respeitando movimento reduzido. Valores são decisões de projeto, não resultados medidos de usabilidade.

## Componentes e interações

| Componente | Contrato |
|---|---|
| `Button` / `Primary` | Ação principal única no contexto; `type=button` por padrão; carregamento e disabled reais; duplo envio bloqueado também no handler |
| `Field` | Label persistente, indicação de obrigatório quando declarada, ajuda/erro associados por ID e preservação de valor após falha |
| `Modal` / `Confirm` | Nome acessível, foco inicial adequado, Tab/Shift+Tab contidos, Escape seguro, restauração do foco. Formulário modificado pede descarte; intacto fecha imediatamente. Um diálogo ativo por vez |
| `Table` / `Pager` | Cabeçalhos semânticos, valores legíveis, rolagem interna de tabela, estado vazio específico e paginação existente; sem virtualização sem necessidade |
| `SkuPicker` | Nome/código na busca. Enter seleciona somente identificação exata única e não salva; ambiguidades sempre exigem decisão |
| `TaskStatus` | Erro em `role=alert`, confirmação real em `role=status`; não anunciar sucesso antes de receber resposta |
| Navegação | Página atual identificada; acesso ao conteúdo por teclado; consulta preservada quando apropriado, sem transformar drafts em preferências |

Cadastro não movimenta saldo. Recebimento e movimentação mantêm conferência/confirmação próprias. Preço e quantidade seguem os parsers existentes; não adaptar regras monetárias por conveniência visual. Códigos permanecem texto, com zeros iniciais. Código compartilhado é aviso informativo, não erro proibitivo.

Atalhos documentados no contexto: Enter no leitor registra/seleciona somente sua tarefa; Tab/Shift+Tab navegam; Escape fecha diálogo intacto ou abre confirmação de descarte. Não adicionar atalhos globais de uma tecla que interfiram com leitores ou digitação.

Persistir apenas preferências de consulta versionadas e validadas, com fallback quando storage estiver bloqueado. O banco local continua sendo a fonte dos cadastros, rascunhos salvos, contagens e histórico.

## Heurísticas aplicadas com contexto

Hick e carga cognitiva orientam opções relevantes e divulgação progressiva; Fitts aproxima e dimensiona ações; Jakob usa padrões familiares; proximidade, semelhança e conectividade agrupam informação; Miller sugere blocos compreensíveis, sem limite arbitrário de itens; Doherty orienta feedback perceptível; Von Restorff destaca a ação principal; posição serial prioriza informação essencial; pico-fim orienta conclusão verdadeira; Zeigarnik e gradiente de meta tornam pendências/progresso reais visíveis; Occam reduz complexidade dispensável; Tesler mantém a complexidade inevitável em conferências; Postel aceita formatos válidos sem adivinhar quantidades; Murphy exige recuperação segura; Parkinson orienta reduzir esforço observado, sem retirar proteções.

## Acessibilidade

Referência: [WCAG 2.2, W3C](https://www.w3.org/TR/WCAG22/). Meta AA; conformidade só pode ser declarada após avaliação completa.

Verificar texto normal com contraste mínimo 4,5:1, texto grande 3:1 e controles/estados visuais relevantes 3:1. As regras completas incluem exceções, por exemplo para elementos inativos. Alvos: adotar 36 px ou mais onde possível; avaliar o critério AA de 24 × 24 CSS px e suas exceções de espaçamento. Oferecer foco visível e não totalmente encoberto, labels e mensagens acessíveis, teclado sem armadilha e estados compreensíveis sem depender exclusivamente de cor.

Verificar 200% de texto/zoom e reflow em 320 CSS px, com rolagem própria nas tabelas que exigem duas dimensões. A janela nativa possui tamanho mínimo existente de 760 × 640; emulação de viewport menor não comprova suporte da janela a esse tamanho. Não confundir simulação de leitor HID com teste físico nem inspeção de ARIA com leitura por NVDA/JAWS.

## Desempenho, validação e aprovação

Medir antes/depois com mesmo ambiente, volume de dados, viewport e tarefa; registrar amostras, p50/p95, execução a frio/aquecida e limitações. Não apresentar duração do build ou de um teste como tempo de operação real. Dados sintéticos devem ficar restritos aos testes e claramente identificados.

Aceitar uma etapa quando: fluxos e regras preservados; menor esforço comprovado ou justificativa de segurança; erros e sucesso corretos; teclado e redimensionamento verificados; testes relevantes aprovados; funcionamento local preservado; documentos refletem código e resultados reais. Riscos/pendências bloqueantes devem estar explícitos.

Sequência: diagnóstico salvo, baseline, fundação visual, correções operacionais, testes e comparação. Sem migrações destrutivas, custos recorrentes, troca de stack sem evidência ou commits automáticos.
