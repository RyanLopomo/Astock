# Changelog UX/UI do Astock

## 08/11/2026 | Axis Design Standard v1.0

### Diagnóstico e preparação

- Identificado Astock 1.3.0 em `C:\Users\R\Documents\PDV`, fora do workspace AXIS inicialmente aberto.
- Inspecionados README, arquitetura, relatório integrado, interface, testes e comandos IPC, sem abrir o banco real.
- Criada cópia isolada de fontes para preparação e validação. Nenhum banco, diretório de dados ou executável de produção foi copiado ou aberto.
- Salvo `ASTOCK_UX_AUDIT.md` antes de alterar a interface e estabelecido `AXIS_DESIGN_STANDARD.md` a partir da identidade verde e componentes existentes.
- Ferramentas específicas ausentes registradas na auditoria; nenhuma dependência instalada.

### Alterações efetivamente implementadas

| Arquivos | Alteração e justificativa |
|---|---|
| `src/ui.tsx` | Tokens/componentes existentes reutilizados; label e hint separados com IDs; obrigatório visível; ModalCancel/dirty/suspended; Tab/Shift+Tab com links; beforeunload de formulário modificado; cabeçalhos e região de rolagem da tabela; SkuPicker resolve código exato único com Enter e limpa seleção em ambiguidade/desconhecido |
| `src/styles.css` | Tokens semânticos da paleta verde, contraste de controles, foco, alvos mínimos operacionais de 36 px, tipografia numérica tabular, geometria discreta, movimento reduzido e reflow de 320 px |
| `src/Products.tsx`, `src/preferences.ts` | Leitor opcional com inclusão por Enter sem salvar, códigos compartilhados sinalizados e permitidos, proteção de código pendente e descarte, filtros de sessão validados, vazio contextual e confirmação real após atualização; resposta confirmada reutilizada se atualização falhar. Saída, ajuste e regularização confirmados bloqueiam edição e repetem apenas atualização; fechar não sugere descartar o movimento já aplicado |
| `src/Receipts.tsx` | Editor preservado/suspenso durante cadastro rápido; SKU novo não redefine quantidade/custo/lotes; descarte protegido; item e rascunho confirmados não são gravados de novo se apenas a atualização falhar |
| `src/Inventory.tsx` | Foco na montagem real do leitor; criação/correção protegidas; sessão já criada é retomada sem repetir a criação após falha de atualização |
| `src/Suppliers.tsx` | Descarte protegido e resposta confirmada reutilizada após falha de atualização |
| `src/WorkspaceApp.tsx` | Acesso direto ao conteúdo por teclado |
| `src/Views.tsx` e módulos operacionais acima | Textos de ausência/separação claros, sem travessões novos; sem alteração dos dados apresentados |
| `src/ux.test.tsx` | 29 casos comportamentais novos para perda de trabalho, confirmação, duplicidade, leitor, teclado e filtros |
| `scripts/ux-browser-check.mjs`, `scripts/ux-contrast-check.mjs` | Cenários Chrome com cliente sintético restrito ao teste, capturas, medidas e contraste calculado; baseline protegido contra sobrescrita |
| `README.md`, `AGENTS.md`, `docs/ux-ui/*` | Padrão, diagnóstico anterior, resultado, decisões, instruções futuras, testes, evidências e limites |

Preservados: dependências, arquitetura, comandos IPC, fontes locais, SQLite, esquema, migrações, livro imutável, FEFO, idempotência, revisões e regras de dinheiro/quantidade. Não foi criado tema escuro parcial nem segunda biblioteca de componentes. Não houve commits ou instalação de dependências.

### Testes executados

| Verificação | Resultado |
|---|---|
| Baseline TypeScript, ESLint, Vitest, build web | Aprovados; 22 testes de interface antes da alteração |
| `npm.cmd run typecheck` | Aprovado após implementação |
| `npm.cmd run lint` | Aprovado sem avisos após implementação |
| `node node_modules/vitest/vitest.mjs run --configLoader native` | 51/51 testes em quatro arquivos: 22 anteriores e 29 novos |
| `cargo test -p axis-estoque-core --offline` | 32/32 testes: cinco unitários e 27 integrações com SQLite temporário, incluindo 1.000 SKUs/5.000 movimentos, rollback, reabertura, XML, FEFO e inventário |
| `cargo fmt --all --check` | Aprovado; fontes Rust não modificadas |
| `node node_modules/vite/bin/vite.js build --configLoader native` | Aprovado; assets locais gerados |
| Tauri `build --no-bundle --ci --config .cache/tauri-ux-build.json -- --offline` | Release aprovado, sem instalador; override de preparação apenas para usar o loader nativo na cópia isolada |
| `node scripts/ux-contrast-check.mjs` | 11 pares aprovados; borda de controle: 2,56:1 antes e 5,15:1 depois |
| Chrome/Playwright headless já existentes | Dez páginas × três viewports, teclado, foco, filtro preservado, Enter sem envio e preservação do item no cadastro rápido; JSON/capturas em `evidencias` |

No staging, `node_modules` referencia as dependências já instaladas do Astock. O loader padrão do Vite tentou escrever cache fora da raiz gravável e retornou EPERM; `--configLoader native` e cache local resolveram a limitação. Isso não exigiu mudar o `vite.config.ts` entregue ao Astock. Uma rodada de navegador durante a compilação Rust concorrente atingiu timeout de navegação; a repetição final, após a queda da carga, passou em aproximadamente 14 segundos. Não se atribui esse timeout a falha funcional da interface.

### Desempenho e comparação

Método de busca: 1.000 SKUs sintéticos, cinco aquecimentos e 30 amostras, evento de input até a linha correspondente mais dois animation frames no Chrome headless de desenvolvimento. Não inclui IPC ou SQLite. Amostras e versão exata do navegador estão em `evidencias/before.json` e `after.json`.

Não houve alteração no mecanismo de busca ou no banco, porque a medição inicial não demonstrou gargalo. Não se declara ganho de velocidade. O esforço reduzido tem evidência funcional: consulta preservada ao voltar, código exato selecionado com Enter sem submit e cadastro rápido sem redigitar quantidade/custo/lotes.

| Busca no Chrome | Antes | Depois |
|---|---:|---:|
| Mediana (p50) | 33,40 ms | 33,50 ms |
| p95 | 33,60 ms | 33,60 ms |

Essas diferenças não demonstram melhoria ou regressão relevante no cenário medido. Não são latências de banco, IPC ou ambiente de produção.

Bundle web: antes 306,44 kB JS / 90,81 kB gzip e 22,66 kB CSS / 6,08 kB gzip; depois 315,33 kB JS / 93,39 kB gzip e 26,77 kB CSS / 6,88 kB gzip. O aumento acompanha validação, proteção e tokens, sem biblioteca nova. Isso não representa tempo de operação.

### Pacote e integridade

Executável: `target/release/astock-ux-v1.exe`, 11.328.000 bytes, SHA-256 `957772a08a12ed8ccf38fad44df2a30e8d9cb96a7f06b9c2b091389c51cb8c55`. A compilação final incorpora `index-CGrUwFR1.js` e `index-JEZ_0sYh.css`; seus nomes foram conferidos no binário. O bundle não contém a fixture de navegador. Detalhes em `evidencias/validation.json`.

O nome separado preserva o `astock.exe` que estava em execução. Não foi encerrada a sessão do usuário nem iniciado o release novo, que usaria o banco principal. Para utilizar esta interface, feche a instância antiga e abra `astock-ux-v1.exe`. O comando habitual de build continuará gerando `astock.exe`.

Pacote validado em cópia isolada e aplicação em `C:\Users\R\Documents\PDV` autorizada explicitamente pelo usuário. A entrega conserva backup dos arquivos substituídos em `.cache/ux-ui-v1-original-backup` e um checkpoint próprio em `docs/checkpoints/ux-ui-v1.sha256`, sem regravar os manifestos históricos. Os hashes dos originais são conferidos antes da cópia e os arquivos entregues são conferidos novamente após a aplicação. São backups de arquivos do projeto, não do banco de estoque.

### Limites e próxima versão

- Homologar HID/ABNT2, NVDA/JAWS, fechamento da janela nativa com formulário modificado e zoom no WebView2 em perfil debug isolado. Reflow emulado a 320 px não altera o tamanho mínimo nativo de 760 × 640.
- Não foi aberto o banco principal nem executada operação mutável em dados reais. Testes de navegador usam cliente sintético; testes de persistência usam SQLite temporário. Desligamento físico de rede/energia não foi simulado.
- A proteção de retries novos cobre resultados confirmados com ID. Uma resposta original incerta não ganha garantia nova de idempotência; investigar como decisão de arquitetura separada se necessário.
- Estender preferências para outros módulos e mapear mensagens monetárias aos campos após observação de uso real. `Field.error` está disponível, sem alteração dos parsers.
- Ferramentas MCP/skills específicas ausentes foram registradas na auditoria. Efetivamente utilizados: PowerShell, ripgrep, TypeScript, ESLint, Vitest/Testing Library, Vite, Cargo/Rust/SQLite temporário, Chrome/Playwright local e referência WCAG 2.2 da W3C.
