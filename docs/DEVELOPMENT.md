# Lokální vývoj

Monorepo obsahuje web/Tauri UI, Rust API a PostgreSQL. Aktuální implementované moduly a jejich omezení uvádí [checklist](FEATURE_CHECKLIST.md). API klienti a MCP schémata se generují přes `pnpm generate:api`; lokální MCP a fakturační moduly jsou implementované, PWA/offline režim zatím ne.

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

Při existujícím `.env` ho nepřepisuj. Web běží na http://127.0.0.1:5173, API na http://127.0.0.1:3001. Server při startu ověří připojení a aplikuje migrace; nedostupná databáze ukončí start chybou. `GET /api/health` ověřuje živý proces, `GET /api/ready` dostupnost DB (503 při výpadku). Migrace zahrnují účty, firemní agendu i implementované moduly dokladů.

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

`apps/web/src/pages`, `components`, `hooks`, `services` a `routes` obsahují produktové obrazovky, jejich stav a navigaci. Nevytváříme prázdné balíčky. Sdílené UI je v `packages/ui`, tokeny v `packages/design-tokens`, HTTP klient v `packages/api-client`. Rust má vrstvy domain/application/infrastructure/server, desktopový crate je v `apps/desktop/src-tauri`.

Migrace jsou v `migrations`; změna schématu vyžaduje nové migrační soubory. Lockfiles obou workspace patří do Gitu. Konkrétní formátovací a kontrolní příkazy jsou centrálně v root package.json.

OpenAPI kontrakty a MCP schémata jsou v `contracts/`, typovaný klient v `packages/api-client`. Po změně kontraktů spusťte `pnpm generate:api`. Databázové regresní sady jednotlivých modulů jsou uvedené v jejich [specifikacích](README.md#specifikace-modulů); vyžadují izolovaný testovací PostgreSQL s oprávněním CREATEDB. Nikdy je nespouštějte proti produkční databázi.

## Ověřeno při založení (10. 10. 2026)

Na macOS Apple Silicon prošel webový produkční build, `pnpm check`, Rust workspace test (1 test) a šest Playwright testů proti lokálnímu API/PostgreSQL. Migrace byla úspěšně aplikovaná. Tauri debug `.app` se sestavila a byla spuštěna; v nativním WebView zobrazila připojené API i databázi. Windows/Linux balíčky, release optimalizovaný build, podpisy a GitHub CI běh zatím nebyly ověřeny.

## Produkt a vývojový sandbox (P01)

`/` spouští produktový vstup. `/#/setup` a `/#/login` od P02 používají skutečný backend a DB.
Firemní profily a jejich agenda vyžadují platnou relaci a členství v dané firmě. Aktuální cesty registruje `apps/web/src/routes/ProductRoutes.tsx`.
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

P13-03: `DATABASE_URL=… pnpm test:advance-settlements` spouští izolované scénáře vyúčtování, rezervací, odpočtů DDPP, doplatku, souběhu a upgradu z 019. UI je součástí `tests/e2e/sales.spec.ts`. [Specifikace](specs/advance-settlement.md).

P13-04: `DATABASE_URL=… pnpm test:credit-notes`, doménové hranice `cargo test -p faktura-server --test credit_note`, UI `pnpm exec playwright test tests/e2e/credit-notes.spec.ts --workers=1`. Testy vytvářejí izolované DB; migrace 021 mění pohled úhrad, před upgradem zastavit starý server. [Specifikace](specs/credit-notes.md).

P14-01: `DATABASE_URL=… pnpm test:recurring`; kalendář `cargo test -p faktura-domain recurring`; UI `pnpm exec playwright test tests/e2e/recurring.spec.ts --workers=1`. Scénáře mají izolované DB a lokální SMTP. Worker běží v API každé 2 sekundy, generuje nejvýše jeden výskyt za iteraci. [Specifikace](specs/recurring-invoices.md).

P14-02: `docs/specs/invoice-communication.md`, migrace 023, `contracts/communication.openapi.json`
a rozšíření invoice-delivery. `DATABASE_URL=… pnpm test:communication`; UI
`tests/e2e/communication.spec.ts`. Testy jen izolovaná DB a loopback SMTP. Regrese
`test:invoice-delivery`, `test:payments`, `test:recurring`, `test:delivery`. Automatizace
je explicitně povolená v profilu; nepovolovat ji při testech v reálné agendě.

P14-03: `DATABASE_URL=… pnpm test:invoice-shares`; UI
`pnpm exec playwright test tests/e2e/invoice-sharing.spec.ts --workers=1`.
Izolované DB, anonymní kontext pro veřejný náhled a lokální SMTP capture.
Regrese `share`, `invoice_delivery`, `attachment`, `communication`, `recurring`.
[Specifikace](specs/invoice-sharing.md), migrace 024; před upgradem zastavit API.

P15-01: `DATABASE_URL=… pnpm test:received`, UI
`pnpm exec playwright test tests/e2e/received.spec.ts --workers=1`.
Syntetické profily, izolovaná DB; přílohy bez externího odesílání. Regrese
`attachment`, `invoice_delivery`, `invoice_share`, navigace `routing.spec.ts`.
Kontrakt received se generuje přes `pnpm generate:api`. Migrace 025.

P15-02: `DATABASE_URL=… pnpm test:received` ověřuje i účtenky, přehledy,
hranice období a upgrade025; `pnpm test:e2e tests/e2e/purchase-receipts.spec.ts`
používá izolované DB a syntetická data. API/MCP generuje `pnpm generate:api`.

P16-01: `DATABASE_URL=… pnpm test:bank`, UI
`pnpm exec playwright test tests/e2e/bank.spec.ts --workers=2`.
Regrese sdílených plateb: `test:payments`, `test:received`, `test:communication`.
Migrace 027; `pnpm generate:api` generuje bank klienta a MCP nástroje.
Testy vytvářejí izolované databáze, UI používá syntetické CSV a lokální SMTP capture.

P16-02: `pnpm test:bank` zahrnuje nativní výpisy a návrhy shod;
`cargo test -p faktura-infrastructure bank_statement` ověřuje syntetické kontrakty.
Migrace 028 přidává importní zdroje. Před upgradem zastavit API; původní data se
nemění. UI: `tests/e2e/bank.spec.ts`. [Podporované varianty a limity](specs/bank-statements.md).

### Pokladna P16-04

`DATABASE_URL=… pnpm test:cash` spustí izolované PostgreSQL testy (vyžaduje
CREATEDB); `pnpm exec playwright test tests/e2e/cash.spec.ts --workers=2`
ověřuje CS/EN desktop i mobil s vlastní dočasnou DB/API. Regrese: bank,
payment, received a communication. [Výsledky](design/cash-validation.md).

### Přehledy a QR (P19-01a / P20-04a)

`DATABASE_URL=… pnpm test:period-report` ověřuje oddělené databáze, přesné
součty s/bez DPH, dataci, stránky, oprávnění a REST/MCP. Uživatelské scénáře
přehledů, skladu a vzhledu jsou v `tests/e2e/catalog.spec.ts` (CS/EN, desktop/mobile).

`cargo test -p faktura-domain payment_qr` ověřuje SPAYD a účty.
`DATABASE_URL=… pnpm test:invoices` zahrnuje QR PDF CS/EN v ignorovaném
adresáři `tmp/pdfs`. Kontrola skenování 16. 10. 2026 použila Poppler
150 DPI a zxing-cpp 2.3.0 v dočasném Python prostředí; nejde o povinnou
produkční závislost. Generování QR probíhá lokálně v Rustu.

P19-01j: `DATABASE_URL=… pnpm test:exchange-rates` ověřuje izolovanou
PostgreSQL a injektovaný kurzový provider bez živé sítě. Parser:
`cargo test -p faktura-infrastructure exchange_rate`; CS/EN desktop/mobile:
`pnpm test:e2e tests/e2e/dashboard.spec.ts`. [Specifikace](specs/exchange-rates.md).
