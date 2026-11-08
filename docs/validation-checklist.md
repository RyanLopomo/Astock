# Validação por módulo — V1.3.0

## Preparação

Use o perfil debug `manual-v13` descrito no README e confira a faixa **Homologação**. Assim os exemplos não entram no banco principal. O executável release usa o banco principal e ignora o perfil. Não apague bancos para reiniciar um ensaio; use outro nome de perfil. Faça backup em Configurações antes de trabalhar com estoque real.

Os exemplos abaixo usam datas de novembro/2026; se a data atual for outra, adapte as validades para testar as faixas locais. XMLs em `crates/axis-estoque-core/tests/fixtures` são sintéticos, sem valor fiscal.

## 1. Importação XML — V1.1

1. Em Produtos, crie “Água 500 ml”, SKU `AGUA500`, código `7890000000017`, embalagem 12, compra R$ 10,00, venda R$ 20,00 e controle de validade habilitado. O saldo deve começar em zero.
2. Em Importar XML, selecione `nfe-compra-exemplo.xml`. Confira fornecedor sintético, nota 123/série 1, chave, 5 CX, preço R$ 120,00 e total R$ 600,00. O estoque deve continuar zero.
3. Selecione/confira o SKU e fator 12: resultado 60 unidades e custo R$ 10,00. O dado qTrib é informativo. Marque salvar associação para o fornecedor. Salve e confira a revisão.
4. No item, aplique/confira o rastro ou preencha `TESTE-LOTE-01`, fabricação 01/11/2026 e validade 15/12/2026. Distribua **60 unidades**; a quantidade comercial do XML é em caixas. Não informe safra sem dado real.
5. Confirme o recebimento no diálogo final. Confira saldo total 60, lote 60 e um movimento de entrada. A venda deve continuar R$ 20,00.
6. Importe novamente o mesmo arquivo e depois `nfe-prefixado-exemplo.xml`: a mesma chave deve abrir o recebimento existente, sem nova entrada. Tentar confirmar novamente não pode aumentar o saldo.
7. Feche/reabra o mesmo perfil. Confira produto, fornecedor, associação, recebimento/lote e saldo. A reutilização da associação em uma segunda chave é coberta também pelo teste nativo automatizado.
8. Em perfil separado, importe antes de cadastrar o produto: o item deve ficar sem associação. Use “Criar produto”, escolha o SKU e salve. Cadastre duas variantes com o mesmo código e confira que uma importação sem associação salva exige seleção explícita.
9. Tente arquivo inválido e arquivo maior que 8 MiB. Confira erro sem entrada de estoque. Distribuição de lotes diferente de 60 também deve impedir a confirmação.

## 2. Lotes e validade — V1.2

1. Após o roteiro XML, registre recebimento manual adicional de 1 caixa × 12 do mesmo SKU, custo total R$ 120,00, lote `TESTE-LOTE-02` e validade 20/02/2027. Confirme: saldo 72, dividido em 60 + 12.
2. Em Produtos, registre saída de 10 unidades. Confira o plano FEFO usando o lote 01, motivo e confirmação. Resultado esperado: total 62, lote 01 com 50 e lote 02 com 12.
3. Tente saída maior que o disponível e ajuste negativo maior que o lote. Deve ocorrer erro sem alteração parcial. Confira movimentos e suas referências.
4. Em outro SKU, receba sem lote/validade. Deve aparecer “Sem lote identificado”, sem data inventada. Use “Identificar saldo sem lote” com quantidade parcial; confira que o total não muda e que existem movimentos de reclassificação.
5. Em outros SKUs, receba lotes com validade ontem, hoje+7, hoje+30 e hoje+31, além de um sem data. Confira Vencidos, Curto, Longo, Dentro do prazo e Sem validade. Altere os limites nas configurações e reabra para verificar persistência.
6. Confira filtros de categoria/período, quantidade afetada e custo estimado. Uma saída de vencido ou inclusão de saldo sem validade deve exigir a opção explícita antes da confirmação.
7. Divida uma nova linha entre dois lotes e confirme. Soma diferente do total deve bloquear. Histórico confirmado deve oferecer correção por ajuste, sem editar/apagar movimentos antigos.

## 3. Inventário por bipagem — V1.3

1. Com o SKU do roteiro anterior em saldo 62, inicie inventário parcial incluindo somente `AGUA500`. Confira referências: sem lote 0, lote 01 com 50 e lote 02 com 12. O saldo continua 62.
2. Tente saída, ajuste e confirmação de recebimento desse SKU: devem ser bloqueados com motivo do inventário. Outro SKU fora do parcial pode movimentar normalmente. Não pode iniciar uma segunda sessão.
3. Bipe/digite `7890000000017` com Enter e escolha explicitamente lote 01: contado 1. Repita: 2. Informe quantidade 12 e leia de novo: 14. Confira foco, feedback e histórico. O lote selecionado permanece indicado; “Trocar lote” permite escolher outro.
4. Use “Informar / corrigir” no lote 01 para substituir por **48**, registrando antes/depois. Conte o lote 02 com **12**. Confira divergência −2 no lote 01 e zero no lote 02. O estoque ainda deve continuar 62.
5. Leia um código desconhecido e uma URL/QR não associada. Deve haver mensagem/evento sem incremento nem abertura de navegador. Em perfil separado, teste variantes com código compartilhado; a escolha precisa ser explícita. Teste códigos com zeros à esquerda.
6. Feche o Tauri durante a sessão ativa e reabra o mesmo perfil. Confira sessão, contagens, eventos e bloqueio preservados. Não pode haver fechamento automático.
7. Na conferência, teste os quatro filtros. Deixe a linha sem lote não contada e tente finalizar: deve exigir decisão. Escolha **preservar saldo dos não contados**, confira o resumo e confirme. Resultado: total **60**, lote 01 com 48 e lote 02 com 12, com um ajuste −2 “Ajuste por inventário”.
8. Reabra a sessão finalizada; não pode editar contagens nem gerar outro ajuste por repetição do fechamento. Confira histórico e saldos depois de reiniciar o processo.
9. Em outra sessão/perfil, deixe item com saldo não contado e escolha explicitamente **zero** no fechamento. Confira ajuste e indicação de zero por decisão, distinto de uma contagem física. Teste também cancelamento: preserva histórico e libera SKUs sem alterar o saldo.
10. Repita com inventário completo, scanner USB HID e celular configurado como teclado HID. Configure sufixo Enter; aguarde feedback entre leituras. Confira layout ABNT2, foco após diálogos, repetição rápida, quantidade, redimensionamento mínimo e maximização.

## Evidências e pendências

- Automatizado: SQLite real com migração/backup/rollback, XML seguro, lotes/FEFO, sessões/contagem/reabertura, escala 1000 SKUs/5000 movimentos; testes React de Enter/foco/reenvio e escolhas.
- Nativo: fluxo Tauri → IPC → Rust → SQLite em banco separado, fechamento/reabertura real do processo e backup. [Resultados](evidencias/native-v1-integrada.json).
- Dependem de vocês: scanner/celular físicos, ABNT2 real, ritmo operacional, usabilidade, instalação em outra máquina Windows e recuperação após queda de energia. Nenhum teste automatizado é apresentado como homologação desses itens.
- Release offline é conferido separadamente no relatório; não confundir prévia Vite com distribuição final.
