# Fase 0 — plano de validação técnica

Data: 05/11/2026. Escopo autorizado: somente a Fase 0.

## Entregas e arquivos

- `src/`: interface React/TypeScript/Tailwind em português, comandos tipados, campo HID por teclado com Enter e histórico de teste.
- `src-tauri/`: shell Tauri 2, comandos fechados e configuração Windows/NSIS.
- `crates/axis-estoque-core/`: prova SQLite independente do Tauri; abertura, gravação transacional e leitura.
- `scripts/`: diagnóstico reproduzível do ambiente e checkpoint por hashes.
- `docs/adr/`, `docs/fase-0-relatorio.md`, `docs/aceitacao-fase-0.md`: decisões, evidências e homologação pendente.

## Sequência

1. Inventariar ferramentas sem modificar a instalação do Windows.
2. Consultar fontes oficiais e registros de pacotes antes de instalar dependências do projeto.
3. Montar uma prova isolada: `%LOCALAPPDATA%\AxisEstoque\phase0\prova.db`. Reservar `data\estoque.db` para a aplicação futura.
4. Validar interface, tipos, lint e testes automatizados disponíveis.
5. Tentar o diagnóstico/build nativo; registrar impedimentos sem simular sucesso.
6. Criar documentação e checkpoint verificável. Não iniciar a Fase 1.

## Riscos conhecidos antes da implementação

- Rust/Cargo, MSVC e Windows SDK não encontrados: testes Rust e abertura Tauri dependem dessas instalações.
- WebView2 detectado no registro ainda exige verificação com janela nativa.
- Leitor físico não disponível à automação: Enter simulado não homologa USB HID, layout ABNT2 ou configuração do equipamento.
- SQLite anterior às correções WAL não será aceito; usar versão incorporada e verificar em runtime.
- SQLite com WAL nunca será copiado enquanto aberto para produzir um backup.

## Critérios de aceite

- Janela Tauri abre em Windows e informa versão real do SQLite, WAL, foreign_keys, synchronous e caminho persistente.
- Gravar e reler texto preserva zeros iniciais e persiste após reinício.
- Entrada inválida falha sem gravar; falha transacional deixa o banco íntegro.
- Leitura HID com Enter e digitação manual retornam feedback, mantêm foco previsível e não duplicam submissão enquanto uma operação está em andamento.
- Banco de outra aplicação ou de esquema incompatível é recusado antes de alterar sua configuração.
- Testes disponíveis e limitações constam do relatório; testes Rust e físicos pendentes mantêm a Fase 0 sem homologação.

Não se exige nesta prova cadastro, movimentação de estoque, backups operacionais, atualização ou instalador publicado. Esses módulos pertencem às fases posteriores.
