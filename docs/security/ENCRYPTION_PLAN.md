# Criptografia em repouso e recuperação

Estado: **não ativada**. O banco/backups permanecem SQLite não criptografado; ACL não equivale a criptografia. Nenhuma chave foi criada, embutida no PE ou gravada em texto ao lado do banco. A alteração automática foi deliberadamente evitada porque perder a chave ou o vínculo Windows pode tornar o estoque irrecuperável.

SQLCipher é tecnicamente compatível com a família de APIs SQLite usada pelo núcleo e pode cifrar páginas do banco e WAL. Exige validar a versão incorporada, build Windows, provedor criptográfico, temporários em memória e comportamento de backup; isso ainda não foi homologado neste projeto. [Projeto e limitações de temporários](https://www.zetetic.net/sqlcipher/design/).

DPAPI CurrentUser é candidato para proteger uma chave aleatória local, não para cifrar o SQLite por si só. Normalmente vincula a recuperação às credenciais e ao computador/perfil; reinstalação e troca de máquina exigem estratégia de recuperação distinta. Não usar o modo LocalMachine para prometer isolamento entre usuários. [Documentação Microsoft](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata).

## Decisões bloqueantes

1. Quem é dono/custodiante da chave e quem pode recuperar dados depois de perder o computador/conta?
2. Como produzir e guardar material de recuperação fora da máquina, sem depender de um único segredo embutido ou do próprio disco?
3. Qual procedimento de restauração em Windows novo deve ser aprovado e testado antes de habilitar criptografia?
4. Qual política trata cópias antigas não cifradas, retenção e descarte de backups em SSD/mídias externas?

## Migração proposta, ainda sem execução

1. Definir modelo de ameaça e recuperação; verificar opção de criptografia de disco gerenciada pelo estabelecimento, sem habilitá-la automaticamente.
2. Criar protótipo isolado com bancos sintéticos, chave aleatória por banco e API criptográfica auditada; proteger material local com DPAPI e ensaiar recuperação independente.
3. Compilar/provar SQLCipher com constraints, triggers, WAL, parser, 1000 SKUs/5000 movimentos, perda de energia simulada, backup/restore e tempos medidos. Não transportar cegamente a exigência de versão SQLite para outra build.
4. Exportar uma cópia consistente do banco atual para **outro arquivo**, cifrado; conferir integridade, FKs e digests lógicos completos, mantendo o original intacto e acessível pela versão anterior.
5. Ensaiar restauração numa máquina/perfil Windows novo usando o procedimento de recuperação aprovado. Só depois autorizar troca de caminho e ativação por usuário, com rollback e backup verificado.
6. Rever exposição de WAL, SHM, journal, temporários, crash dumps, backups e material de recuperação. Criptografia do arquivo principal sozinha não basta.

Não há benchmark, integração DPAPI ou teste SQLCipher executado para afirmar neste momento. O bloqueio é de decisão/recuperabilidade e homologação, não falta de uma senha fixa para colocar no código.
