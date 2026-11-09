# Dependências e cadeia de suprimentos

Data: 08/11/2026. npm/package-lock.json e Cargo/Cargo.lock são os gerenciadores/locks reais. Baseline instalado, árvores e locks originais em `.cache/security-before`. Não foi executado audit fix --force nem atualização geral.

| Verificação | Resultado |
|---|---|
| npm audit no registro público | 0 vulnerabilidades reportadas, 289 dependências no relatório de metadados (5 de produção) |
| cargo-audit 0.22.2 portátil | 0 vulnerabilidades na lista do utilitário; 2 avisos informativos no lockfile multiplataforma |
| Conferência equivalente TOML/semver | 1377 advisories da base oficial confrontados com 420 entradas do lockfile; confirmou os dois avisos |
| Árvore Windows normal/build | glib e proc-macro-error não aparecem |
| Bibliotecas não usadas | Nenhuma dependência direta comprovadamente removível sem afetar código/configuração/testes ou a prova histórica |

O cargo-audit foi executado com `--db .cache/security-before/rustsec-advisory-db --no-fetch --no-yanked --json`. A base foi clonada do RustSec e fixada no commit `309392b18f6b17022f82c701f300433126901068`. O clone inicial via Schannel falhou no ambiente; a repetição usou o backend TLS OpenSSL do Git, com verificação de certificado mantida. Yanked crates não foram consultados nessa execução offline. Zero ocorrências conhecidas não é garantia de segurança.

| Pacote | Instalada | Advisory / categoria | Correção publicada | Ação e contexto |
|---|---|---|---|---|
| glib | 0.18.5 | RUSTSEC-2024-0429 / GHSA-wrw7-89jp-8q8g, unsound; sem CVSS publicado nesse registro | >=0.20.0 | Mantido na cadeia GTK/Tauri de Linux. Não é compilado para o Windows auditado; atualização isolada quebraria compatibilidade da cadeia |
| proc-macro-error | 1.0.4 | RUSTSEC-2024-0370, unmaintained | Nenhuma | Dependência transitiva de Linux; monitorar upstream. Não confundir falta de manutenção com CVE explorável no executável Windows |

Fontes: [RustSec glib](https://rustsec.org/advisories/RUSTSEC-2024-0429.html), [RustSec proc-macro-error](https://rustsec.org/advisories/RUSTSEC-2024-0370.html). Os registros não fornecem severidade CVSS; a classificação informativa neste relatório refere-se ao alvo Windows observado.

## Alterações realizadas

- Nenhuma versão de pacote foi atualizada por vulnerabilidade confirmada no alvo e nenhuma biblioteca foi removida artificialmente para melhorar números.
- rusqlite 0.41.1 ganhou a feature limits. windows-sys 0.61.2 no núcleo, e serde_json 1.0.154/windows 0.62.2/webview2-com 0.39.1 no shell foram declarados diretamente para usar APIs já presentes transitivamente na árvore/lock. Não são novos serviços, plugins genéricos ou custos recorrentes.
- Cargo.lock foi atualizado para refletir os usos diretos. Dependências npm não mudaram; seus scripts de segurança foram acrescentados sem alteração de APIs de produto.
- As dependências de desenvolvimento não entram automaticamente no aplicativo: Vite/ESLint/Vitest/TypeScript/CLI são ferramentas de build/teste. O frontend distribuído contém o código React/Tauri necessário; GTK/glib de Linux não integram o PE Windows.
- Gitleaks 8.30.1 e cargo-audit 0.22.2 são ferramentas portáteis locais, fora do artefato. Downloads oficiais com SHA-256 conferido; não foram instalados no runtime. CI referencia checkout/setup-node por commits fixos e cargo-audit por versão.

Imports, tipos, scripts, configuração Vite/ESLint, prova ProbeApp, macros e features nativas foram revisados antes de decidir não remover dependências. Tailwind continua necessário à prova preservada e aos estilos existentes. Não foram introduzidas bibliotecas de sanitização porque HTML externo não é necessário nem renderizado.

O tamanho do PE antes/depois, hashes e validação do bundle final constam da evidência de build. Diferença de tamanho reflete hardening/SDK e não é benchmark de segurança/desempenho. Atualizações futuras devem repetir auditoria e testes; a CI foi configurada, mas não executada remotamente nesta sessão.
