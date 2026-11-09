# Segredos, arquivos privados e Git

Data: 08/11/2026. Nenhum valor de credencial aparece neste relatório.

## Escopo e resultados

Gitleaks 8.30.1 executado em `git . --log-opts=--all --redact` e em `dir .` com configuração local: **nenhum vazamento encontrado**. O bundle inicial também foi clonado em mirror local e examinado com --all, sem vazamentos. Um commit está nas branches/tags disponíveis; main e origin/main foram examinados, além dos objetos disponíveis localmente pelo scanner complementar. Não foi feito fetch de branches privadas/ausentes e não houve reescrita, exclusão de commits ou push.

O scanner complementar examina arquivos atuais não ignorados, todos os blobs Git locais (inclusive sem caminho atual), dist/public, strings diretas dos executáveis e, na conferência ampliada, textos diagnósticos/logs locais. O primeiro resultado teve apenas quatro caminhos .sql das migrações, revisados como falsos positivos. A exceção foi restringida às migrações versionadas. Os resultados não indicam token/chave/senha real ou necessidade concreta de revogação/rotação.

Inspecionados: fontes Rust/TS, JSON/TOML, scripts, CI, configuração npm/Git sem imprimir valores, bundles frontend, arquivos/nomes sensíveis e artefatos. Não foi encontrado serviço de terceiros que exija segredo em produção. Não há chave de assinatura de updater configurada. Histórico e screenshots existentes contêm diagnósticos locais e exemplos/testes, não foram reconhecidos como credenciais; publicação desses diagnósticos deve continuar sujeita a revisão de privacidade.

## Prevenção aplicada

- .gitignore ampliado para SQLite/sidecars, backups, dumps, arquivos de chave e arquivos privados comuns; arquivos públicos de exemplo precisam conter apenas placeholders.
- Hook `.githooks/pre-commit` **ativado localmente** com core.hooksPath. Examina o conteúdo staged, não apenas a cópia em disco. Não cria commits. Novos clones devem executar `git config core.hooksPath .githooks`.
- CI com histórico completo no checkout, credenciais de checkout não persistidas, scanner e verificações. O workflow não foi publicado/disparado nesta execução.
- Prefixo de frontend ASTOCK_PUBLIC_ explícito. Todo valor nesse prefixo é público. VITE_* não é um cofre; não colocar API keys, senhas ou chaves privadas em qualquer bundle, ainda que o nome da variável pareça privado.
- Banco, backups e cópias diagnósticas locais conhecidas protegidos por ACL. Backup real da auditoria está em diretório privado ignorado, nunca no relatório ou assets.

## Tratamento futuro

Segredo real identificado exige retirada do código atual e avaliação prioritária de revogação/rotação no serviço responsável. Não se deve testar uma credencial encontrada contra terceiros. Apagar histórico não revoga segredo. A reescrita, se necessária, exige autorização separada, coordenação de clones/backups e revisão; não foi feita.

Chave **pública** de verificação de atualização pode acompanhar o aplicativo. Chave **privada** de assinatura fica fora de código/instalador, em infraestrutura segura com acesso mínimo. Segredos necessários só no servidor Axis permanecem no servidor. Credenciais locais futuras exigem Windows Credential Manager/DPAPI e avaliação de recuperação; o aplicativo atual não precisa criar um cofre sem uso.

## Limitações

Scanners não provam ausência absoluta: credenciais ofuscadas, formatos não reconhecidos, OCR de imagens, blobs binários arbitrários e seções comprimidas do PE não têm cobertura completa. O scanner local adicional é baseado em padrões, não substitui sozinho um produto especializado; Gitleaks foi usado como segunda verificação. Objetos/refs nunca obtidos ou apagados no remoto não foram abrangidos. O estado atual e histórico devem ser reavaliados após novas integrações.
