# Astock

- Antes de mudar a interface, leia `docs/ux-ui/AXIS_DESIGN_STANDARD.md`, a auditoria e a documentação do módulo; registre diagnóstico e evidência antes de implementar.
- Preserve Tauri/React/TypeScript/Rust/SQLite, IPC, regras monetárias, códigos como texto, FEFO, idempotência, revisões, histórico e bloqueios de inventário.
- Cadastro não altera saldo. Nunca anuncie sucesso antes da confirmação real. Códigos compartilhados são permitidos; sinalize ambiguidade sem proibir.
- Reutilize `src/ui.tsx` e tokens semânticos de `src/styles.css`; mantenha identidade verde, fontes locais, teclado, foco e descarte seguro.
- Testes mutáveis só com cliente injetado de teste, SQLite temporário ou perfil debug explicitamente isolado. Release usa banco real mesmo com `AXIS_ESTOQUE_TEST_PROFILE`; não o abra para ensaios mutáveis.
- Execute typecheck, lint, Vitest e build para mudanças de interface; teste núcleo quando afetado. Registre navegador, dados sintéticos, medidas e limitações reais. Não invente benchmarks/conformidade WCAG.
- Atualize auditoria/changelog; preserve checkpoints históricos. Sem mudança externa ao escopo, dependências pesadas, migração destrutiva, custos ou commits automáticos.
