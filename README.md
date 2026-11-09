# Astock — V1.3.0 integrada

Auditoria de segurança de 08/11/2026: [relatório e riscos residuais](docs/security/SECURITY_AUDIT.md). Execute `npm.cmd run test:security` e, após o build, `npm.cmd run security:check`. Em novos clones, ative `git config core.hooksPath .githooks`. O checkpoint atual da auditoria é `security-20261108`; checkpoints anteriores permanecem históricos.

Aplicativo Windows com Tauri 2, React/TypeScript e núcleo Rust/SQLite local. Inclui cadastros, recebimentos manuais, importação de XML de compra (V1.1), lotes/validade/FEFO (V1.2) e inventário físico por leitura de códigos (V1.3). Não requer servidor nem assinatura. A versão compilada incorpora a interface e o SQLite; o Windows precisa do WebView2.

O banco e o histórico da Fase 0 continuam no mesmo local. Os documentos antigos são registros históricos; o estado atual está no [relatório integrado](docs/relatorio-v1-integrada.md). Nenhum dado sintético é criado automaticamente no banco principal.

## Abrir e desenvolver

O executável de distribuição fica em `target/release/astock.exe` após `npm.cmd run desktop:build`. Ele usa `%LOCALAPPDATA%\AxisEstoque\phase0\prova.db`, inclusive após a mudança de nome para Astock. O identificador interno e o diretório de dados foram mantidos para preservar os cadastros e o histórico existentes. Faça backup também em outro dispositivo antes de utilizar dados importantes.

Para desenvolver: Node 24.15+ da linha 24, Rust/MSVC, Visual Studio C++ Build Tools, Windows SDK e WebView2. No PowerShell:

```powershell
$env:Path = 'C:\Users\R\.cargo\bin;' + $env:Path
npm.cmd ci
npm.cmd run desktop:dev
```

`npm.cmd run dev` inicia apenas a prévia web; as operações de banco exigem Tauri. O servidor Vite é exclusivo do desenvolvimento. Use `npm.cmd` para evitar o wrapper PowerShell bloqueado, sem mudar a política global.

## Conferir os módulos com segurança

Siga o [roteiro manual separado por módulo](docs/validation-checklist.md). Os XMLs em `crates/axis-estoque-core/tests/fixtures` são exemplos sintéticos sem valor fiscal.

Um executável **debug** permite um banco de homologação separado e persistente:

```powershell
# Terminal 1
npm.cmd run dev
# Terminal 2, após cargo build -p axis-estoque-desktop
$env:AXIS_ESTOQUE_TEST_PROFILE = 'manual-v13'
.\target\debug\astock.exe
```

Confira a faixa **Homologação** antes de inserir exemplos. O banco fica em `.cache/test-data/manual-v13/estoque.db`; reutilizar o nome preserva as contagens. Escolha outro nome para começar outro ensaio. O executável release ignora essa variável e usa sempre o banco principal.

## Verificar e compilar

```powershell
npm.cmd run typecheck
npm.cmd run lint
npm.cmd test
npm.cmd run build
cargo test -p axis-estoque-core --offline
cargo fmt --all --check
cargo clippy --workspace --all-targets --offline -- -D warnings
npm.cmd run desktop:build
node scripts/checkpoint.mjs --name astock --verify
```

O modo `--offline` do Cargo exige dependências já disponíveis. A distribuição atual é o executável sem instalador; assinatura, atualização automática e instalação em Windows limpo ainda precisam de homologação. Leitor USB/celular HID também exige teste físico.

## Documentação

- [Axis Design Standard v1.0](docs/ux-ui/AXIS_DESIGN_STANDARD.md), [auditoria UX/UI](docs/ux-ui/ASTOCK_UX_AUDIT.md) e [alterações e validação](docs/ux-ui/ASTOCK_UX_CHANGELOG.md).
- [Arquitetura](docs/architecture.md), [banco e migrações](docs/database.md), [decisões](docs/adr/0003-nucleo-unico-e-migracao-preservada.md).
- [XML](docs/v1.1-xml.md), [lotes](docs/v1.2-lotes.md), [inventário](docs/v1.3-inventario.md).
- [Validação manual](docs/validation-checklist.md), [resultados e limites](docs/relatorio-v1-integrada.md), [changelog](CHANGELOG.md).

Importar XML verifica estrutura e chave; não comprova assinatura, autenticidade ou autorização fiscal. Não há emissão de NF-e, consulta SEFAZ, TEF nem aplicativo móvel nesta entrega.

## Verificação UX/UI

Os testes em `src/ux.test.tsx` usam somente clientes sintéticos. `node scripts/ux-contrast-check.mjs` verifica pares de tokens. O script `scripts/ux-browser-check.mjs` abre Chrome headless e um Vite separado na porta 1431, injeta dados identificados exclusivamente no navegador de teste e não usa Tauri/SQLite.

O executável preparado para esta entrega UX/UI fica em `target/release/astock-ux-v1.exe`. O `astock.exe` que estava aberto foi preservado. Para usar a nova interface, feche a instância anterior antes de abrir esse arquivo. Builds futuros pelo comando habitual continuam gerando `target/release/astock.exe`.

O teste de navegador requer Playwright já disponível no ambiente. Se estiver em outro projeto local, informe o caminho do módulo, sem instalar dependência de produção:

```powershell
$env:PLAYWRIGHT_MODULE_PATH = 'C:\caminho\node_modules\playwright\index.mjs'
node scripts/ux-browser-check.mjs
```

Resultados, método e limitações ficam em `docs/ux-ui/evidencias`. Os testes simulam Enter de teclado; um leitor físico e o fechamento da janela nativa ainda exigem homologação no perfil debug isolado.
