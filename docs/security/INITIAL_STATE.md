# Estado inicial da auditoria

Data: 08/11/2026. Produto: Astock 1.3.0. HEAD: `23f89161cbdbf3754fafb6b44825d2e2b6938fb8`. Git inicialmente limpo; branches locais/remotas disponíveis: main/origin/main. Nenhum commit/push autorizado nesta auditoria.

Checkpoint recuperável: `.cache/security-before/repository.bundle`, criado com `git bundle create --all` e verificado. Lockfiles e árvore instalada preservados na mesma pasta. Backup do banco **de desenvolvimento sintético** por SQLite Online Backup API, origem somente leitura: `.cache/security-before/development-online.db`; integrity_check ok e foreign_key_check sem erros. Nenhum teste mutável no banco principal.

Baseline executado nesta sessão: Vitest 51/51; Rust 5 legados + 27 integração, incluindo 1000 SKUs e 5000 movimentos; npm audit 0 vulnerabilidades. Node 24.16.0, npm 11.13.0, Cargo 1.99.1, Git 2.53.0.windows.2. cargo-audit, Gitleaks e OSV-Scanner não estavam instalados. RustSec será conferido por ferramenta equivalente local sobre a base oficial.

Arquitetura observada: Tauri 2, React/TypeScript/Vite/Tailwind, Rust/rusqlite, SQLite 3.54.1 incorporado, catálogo, compras manuais/XML, lotes/FEFO e inventário. Não há CSV, serviço HTTP/API remota de produção, plugin shell/filesystem/SQL, updater ou assinatura de distribuição configurados. O Vite existe apenas no desenvolvimento.

Diagnóstico antes das correções: comandos próprios não cadastrados no AppManifest ACL; cada IPC cria spawn_blocking sem limite de admissão; backups repetidos não têm limite de frequência; parser XML tem limites e rejeita DTD/entidades, porém File.text substitui bytes UTF-8 inválidos; SQLite usa defaults de confiança/limites; caminhos fixos não rejeitam reparse points/hard links; diretório principal herda leitura para grupo local adicional. `.gitignore` protege .db e alguns segredos, mas não todas as extensões/cópias/arquivos de assinatura. Não há scanner/hook/CI de segurança. Estes achados serão reproduzidos/classificados nos relatórios finais; ausência de criptografia/assinatura será tratada com suas condições de exploração, sem ativação destrutiva.

A regra AGENTS.md e o Design Standard foram lidos. A alteração de interface prevista é estritamente a rejeição de arquivo XML malcodificado, preservando fluxo, foco e mensagens de confirmação existentes. Relatórios anteriores e seus checkpoints continuam históricos.
