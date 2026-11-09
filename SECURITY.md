# Segurança do Astock

Consulte [auditoria e modelo de ameaça](docs/security/SECURITY_AUDIT.md), [achados](docs/security/VULNERABILITIES.md) e [testes](docs/security/SECURITY_TESTS.md).

Não publique bancos, backups, dados comerciais, credenciais ou chaves privadas em issues/logs. Relate possíveis vulnerabilidades pelo canal privado do projeto com reprodução sintética e versões; não teste serviços de terceiros ou credenciais encontradas.

Para novos clones: `git config core.hooksPath .githooks`. Execute os checks npm/Rust e `npm run security:check` após gerar o frontend. Todo valor ASTOCK_PUBLIC_* do Vite é público. Chaves privadas de assinatura e segredos de servidor não pertencem ao aplicativo cliente.

O release usa o banco real mesmo com AXIS_ESTOQUE_TEST_PROFILE. Testes mutáveis exigem debug com perfil isolado/SQLite temporário. Preserve o diretório do banco e seus sidecars; backup com aplicativo aberto deve usar a API SQLite, não copiar só o .db.
