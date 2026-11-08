# ADR 0004 — Valores exatos e operações auditáveis

Data: 07/11/2026. Estado: adotada.

## Decisão

Usar um único livro de movimentos imutável, com quantidades inteiras, centavos e referências de origem. XML mantém os valores comerciais originais como texto decimal e faz cálculos com inteiro de escala fixa, nunca float. A conversão física é conferida pelo operador e não é deduzida de qTrib. Arredondar custo unitário em centavos e mostrar o resíduo preserva a explicação do total comprado.

Sessões de inventário capturam referência e bloqueiam os SKUs por toda a duração. Isso evita exigir reconciliação de movimentações concorrentes nesta aplicação de computador único. O bloqueio é reforçado em SQL; a transação de fechamento permite exclusivamente os ajustes da própria sessão. Não contado é NULL e exige decisão explícita. Referências, eventos e finalização sobrevivem ao processo.

Chaves persistidas de solicitação distinguem repetição intencional de um novo evento de reenvio da mesma operação. Reutilização com conteúdo diferente é rejeitada. Confirmações finais usam revisão e transação atômica.

## Consequências

Não há quantidades fracionárias no estoque nesta versão; uma quantidade comercial decimal só é aceita quando a conversão resulta em unidades inteiras. Recontagens são registradas como eventos. Saídas e entradas antigas não são reescritas. Uma única sessão ativa pode bloquear vários SKUs até finalizar/cancelar, inclusive após reiniciar. Lotes sem data precisam de tratamento explícito.

Homologação usa bancos temporários e perfil debug separado, sinalizado visualmente. O release sempre escolhe o banco original. Isso permite testar o fluxo nativo sem introduzir exemplos na operação real.
