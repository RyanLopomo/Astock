# Aceitação manual da Fase 0

Status inicial de todos os testes abaixo: **pendente**. Não preencher como aprovado apenas pela passagem dos testes automatizados de interface.

Registrar: data, operador, Windows/build/arquitetura, hardware (CPU/RAM/disco), WebView2, versões Rust/Tauri/SQLite, modelo e firmware do leitor, layout do teclado e configuração de terminador. Não registrar códigos comerciais sensíveis nas evidências compartilhadas.

| Cenário | Procedimento | Critério de aceite |
|---|---|---|
| Janela nativa | Instalar toolchain; `npm.cmd ci`; `npm.cmd run desktop:dev` | Janela Tauri abre, sem erros de IPC/CSP |
| Configuração SQLite | Clicar em Verificar banco | SQLite >=3.54.0, WAL, FULL, chaves ativas, 5000 ms, quick_check OK |
| Caminho persistente | Conferir rodapé | `%LOCALAPPDATA%\AxisEstoque\phase0\prova.db`; nenhum dado no diretório de instalação |
| Texto e zeros | Digitar `0012345678905` e Enter | Exatamente o mesmo texto gravado e relido; foco volta ao campo |
| HID real | Usar leitor USB em modo teclado, terminador Enter | Uma leitura completa por acionamento; feedback visível |
| Formatos | Testar EAN-8, EAN-13 e Code128 do equipamento | Texto íntegro, incluindo símbolos compatíveis; esta fase não valida dígito EAN |
| Layout e ritmo | Testar ABNT2, sequências repetidas e Tab/Enter | Sem perda/supressão silenciosa; aguardar confirmação antes da próxima leitura |
| Foco | Clicar em observação, depois Focar campo de leitura | Leitura vai para o campo correto; sem captura global que interrompa edição |
| Falha recuperável | Provocar falha controlada em banco descartável | Mensagem compreensível, sem falso sucesso; código permanece para conferência |
| Reabertura | Gravar, fechar e abrir | Histórico preservado e integridade OK |
| Offline desktop | Desconectar rede e iniciar executável de release | Janela, consulta e gravação funcionam sem servidor ou internet |
| Banco incompatível | Usar arquivo descartável com outro application_id/user_version | Abertura é recusada; nenhum esquema é migrado |

Executar também `cargo test -p axis-estoque-core`, `cargo fmt --all --check` e `cargo clippy --workspace --all-targets -- -D warnings` antes de aprovar. Os testes de integração usam arquivos temporários reais, jamais o banco principal.

## Limites explícitos

- O teste com `userEvent` simula teclado; não testa USB, firmware, ABNT2 ou WebView2.
- O histórico aceita códigos repetidos, intencionalmente. Não representa seleção de SKU ou idempotência de recebimento.
- Não há fila de leitura durante gravação: aguardar feedback antes da próxima bipagem. Medir a latência no hardware-alvo antes de definir a solução da Fase 3.
- Encerramento abrupto, falta de energia, disco cheio, lock entre processos e durabilidade real precisam de ensaios controlados adicionais. Não foram homologados por um teste de rollback de transação.
- Cenários de 1.000 SKUs, CSV, movimentações, recebimentos, backups e atualização não pertencem à prova atual.

Aprovação: preencher resultados, anexar evidências e resolver bloqueios antes de iniciar a Fase 1.
