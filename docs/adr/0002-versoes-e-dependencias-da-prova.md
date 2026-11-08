# ADR 0002 — Compatibilidade e dependências de desenvolvimento

Data: 05/11/2026. Status: adotada para a Fase 0.

## Verificação anterior à instalação

Foram consultados os registros npm (`npm view ... version engines peerDependencies`) e as fontes oficiais do Tauri, rusqlite e SQLite. O ambiente tem Node 24.16.0 e npm 11.13.0.

Versões selecionadas: Tauri Rust 2.12.5, tauri-build 2.7.4, CLI/API JS 2.12.4, React/ReactDOM 19.3.1, Vite 8.3.6, plugin React 6.2.1, Tailwind 4.3.4, TypeScript 6.0.3 e Vitest 5.1.0. As versões JS exatas e dependências transitivas estão no package-lock.json. As versões Cargo diretas críticas estão fixadas; a resolução transitiva ainda exige gerar Cargo.lock com Cargo instalado.

TypeScript 7.0.2 era o mais recente no registro, mas typescript-eslint 8.72.1 declara suporte a TypeScript `<6.1.0`. A tentativa inicial foi recusada pelo npm, sem instalação forçada. Adotou-se 6.0.3. Vite/plugin React, Vitest e jsdom também tiveram seus requisitos Node consultados e atendidos pelo Node disponível.

## Correção de dependência transitiva

O primeiro `npm audit` encontrou dez ocorrências moderadas na árvore de lint causadas pela mesma dependência `minimatch` 10.2.6. A árvore de produção (`npm audit --omit=dev`) não apresentou alertas.

Foi fixado um override transitivo em **minimatch 11.0.1**, versão corrigida indicada no [aviso do mantenedor GHSA-g3hv-75wg-m249](https://github.com/advisories/GHSA-g3hv-75wg-m249). Esta atualização ultrapassa a faixa major declarada por consumidores de lint; por isso requer nova validação de lint, tipos, testes, build e audit. O resultado real consta do relatório/evidências. Não foi usado `npm audit fix --force` nem `--legacy-peer-deps`.

Revisitar esse override quando ESLint e typescript-eslint adotarem a versão corrigida em suas faixas normais; removê-lo somente após atualizar o lockfile e repetir as verificações.

## Fontes

- [Tauri 2.12.5](https://docs.rs/crate/tauri/2.12.5)
- [tauri-build 2.7.4](https://docs.rs/crate/tauri-build/2.7.4)
- [Vite: requisitos Node](https://vite.dev/guide/)
- Registros consultados diretamente: `registry.npmjs.org` via npm, com cache local ao projeto.
