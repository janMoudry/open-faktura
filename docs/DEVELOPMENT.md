# Lokální vývoj

Kostra obsahuje společné React UI pro web/Tauri, Tailwind tokeny, Rust API a PostgreSQL. Fakturace, firemní agenda, MCP, PWA offline režim, worker a generování OpenAPI zatím nejsou implementované. API klient má zatím ručně udržovaný, runtime kontrolovaný health kontrakt.

## Požadavky

- Node.js 22.22.0 nebo kompatibilní novější verze; pnpm 10.29.3.
- Rust přes rustup; `rust-toolchain.toml` připíná 1.99.0 a komponenty.
- Běžící Docker s Compose pro PostgreSQL.
- Pro Tauri nativní build nástroje podle OS: na macOS Xcode Command Line Tools, na Windows MSVC a WebView2, na Linuxu WebKitGTK a další [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

Pokud Rust není v PATH, v aktuálním shellu spusť `source "$HOME/.cargo/env"`. Instalace provedená při založení repa shellový profil neměnila.

## Web a API

Z kořene repozitáře:

```sh
pnpm install --frozen-lockfile
cp .env.example .env
node scripts/init-secrets.mjs
pnpm db:up
pnpm dev
```

Při existujícím `.env` ho nepřepisuj. Web běží na http://127.0.0.1:5173, API na http://127.0.0.1:3001. Server při startu ověří připojení a aplikuje migrace; nedostupná databáze ukončí start chybou. `GET /api/health` ověřuje živý proces, `GET /api/ready` dostupnost DB (503 při výpadku). Úvodní migrace zakládá pouze technická metadata, nikoli neověřené účetní entity.

`pnpm db:down` zastaví lokální DB a zachová volume. Vývojové přihlašovací údaje jsou veřejné a porty jsou vázané na loopback; tato konfigurace není produkční nasazení. API má od P02 bootstrap správce a serverové relace; viz [instalace a přístup](INSTALLATION.md).

## Desktop

Spusť DB a API, pokud neběží:

```sh
pnpm db:up
pnpm dev:api
```

Ve druhém terminálu:

```sh
pnpm dev:desktop
```

Tauri spouští vlastní Vite dev server, proto současně nespouštěj `pnpm dev` nebo `pnpm dev:web` na stejném portu. V produkčním balíčku Tauri přímo vloží výstup `apps/web/dist`; React UI je společné. Výchozí API adresa je loopback. Změnu lze zatím udělat build proměnnou `VITE_API_URL`; pro desktop současně uprav `connect-src` v Tauri CSP a povol odpovídající origin na serveru. Výběr serveru v UI bude navazující práce.

```sh
pnpm build:desktop
```

Tento příkaz sestaví nativní balíčky aktuálního OS do `target/release/bundle`. Vývojový macOS app bundle bez release optimalizace lze sestavit:

```sh
pnpm --filter @open-faktura/desktop build --debug --bundles app
```

Podpisy, notarizace, automatický updater a publikace GitHub Releases ještě nejsou nakonfigurované. Lokálně vytvořený balíček není vydaná podepsaná aplikace. Desktop neinstaluje backend ani PostgreSQL.

## Kontroly

```sh
pnpm build:web
pnpm check
pnpm test
```

`check` zahrnuje typy, Prettier, omezenou statickou kontrolu tokenů/závislostí UI, rustfmt a Clippy. Kontrola konvencí je počáteční pojistka, nikoli kompletní analýza importního grafu. Rust workspace zahrnuje i Tauri, takže vyžaduje nativní desktopové závislosti. Pro samotný backend použij `cargo test -p faktura-server` a `cargo build --locked -p faktura-server`.

E2E používá skutečný lokální API server a DB; dvě chybové situace simuluje přes síťovou interceptaci. S běžícím API:

```sh
pnpm exec playwright install chromium
pnpm test:e2e
```

Playwright spustí Vite, pokud ještě neběží. Testuje desktopový a mobilní viewport v Chromiu, nikoli instalaci PWA nebo nativní Tauri WebView.

CI má macOS kontrolu celého workspace a debug app bundlu, Linux integrační test proti PostgreSQL. Workflow je připravené; skutečné spuštění na GitHubu ověří až push. Release automatizace bude navazovat na podpisy a podporované cíle.

## Organizace

`apps/web/src/pages`, `components`, `hooks`, `services` jsou funkční části úvodní obrazovky. `utils`, `routes`, `types`, worker a další moduly přibudou s prvním použitím. Nevytváříme prázdné balíčky. Sdílené UI je v `packages/ui`, tokeny v `packages/design-tokens`, HTTP klient v `packages/api-client`. Rust má vrstvy domain/application/infrastructure/server, desktopový crate je v `apps/desktop/src-tauri`.

Migrace jsou v `migrations`; změna schématu vyžaduje nové migrační soubory. Lockfiles obou workspace patří do Gitu. Konkrétní formátovací a kontrolní příkazy jsou centrálně v root package.json.

## Ověřeno při založení (10. 10. 2026)

Na macOS Apple Silicon prošel webový produkční build, `pnpm check`, Rust workspace test (1 test) a šest Playwright testů proti lokálnímu API/PostgreSQL. Migrace byla úspěšně aplikovaná. Tauri debug `.app` se sestavila a byla spuštěna; v nativním WebView zobrazila připojené API i databázi. Windows/Linux balíčky, release optimalizovaný build, podpisy a GitHub CI běh zatím nebyly ověřeny.

## Produkt a vývojový sandbox (P01)

`/` spouští produktový vstup. `/#/setup` a `/#/login` od P02 používají skutečný backend a DB.
`/#/companies` a `/#/workspace` vyžadují platnou relaci; firemní agenda čeká na P03.
Health/readiness nejsou autentizace ani důkaz nově instalované aplikace.
`/#/system` zobrazuje diagnostiku spojení.

Katalog je na `/sandbox.html#/design-system`, referenční návrh faktur na
`/sandbox.html#/invoices`. Oba jsou dostupné jen přes Vite dev server.
Samostatný HTML vstup není součástí produkčního buildu; build kontroluje importní
graf a odmítne zahrnutí demo modulů. Tauri release používá stejný produktový build.
Vývojový server nevystavovat jako produkční hosting.

Sandbox zachovává syntetické scénáře a účetní demo výpočty, nikoli skutečnou
API agendu. Jeho testy používají explicitně sandbox URL. Produktové testy jsou
v `tests/e2e/product.spec.ts`; žádný výpadek se nesmí maskovat demo daty.

## Návrhy faktur (P06)

Produktové routy: `/#/companies/:companyId/invoices` a `…/invoices/new`.
API ukládá návrhy do PostgreSQL, migraci 007 spouští standardní start serveru.
DB regrese: `DATABASE_URL=... pnpm test:invoices`; UI: `pnpm exec playwright test tests/e2e/invoices.spec.ts`.
Specifikace a omezení: [invoice-drafts.md](specs/invoice-drafts.md).
