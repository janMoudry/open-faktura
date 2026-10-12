# Open Faktura — práce v repozitáři

Cílem je plná otevřená alternativa českého iDokladu včetně self-hostingu, API a MCP. Rozsah je v [FEATURES.md](FEATURES.md). Projekt má funkční fakturaci, úhrady, kontakty, nabídky a související moduly; aktuální rozsah a zbývající práci uvádí implementační plán.

## Postup

- Vycházej z referenčního chování. Před implementací funkce najdi její specifikaci a důkazy; chybějící pravidla nejdřív ověř nebo výslovně označ jako návrh.
- Rozlišuj veřejnou dokumentaci, staticky přečtený SDK kontrakt, pozorování aplikace/API a naše návrhová rozhodnutí. SDK neodhaluje databázi ani všechna serverová pravidla.
- U zdrojů zapisuj datum, URL a verzi/commit, u pozorování vstupy, nastavení firmy a výstupy. Rozpory eviduj; nepřepisuj je odhadem.
- Referenční číselníky a ID patří do importních adaptérů. Nejsou automaticky interním modelem Open Faktury.
- Funkční shoda neznamená převzetí tarifních limitů nebo historických funkcí. Odchylky zaznamenej s důvodem.
- Nevkládej do repa přihlašovací údaje ani neanonymizovaná účetní data. Pro nové pokusy používej testovací agendu; nepředpokládej oprávnění odesílat zákazníkům nebo měnit skutečné doklady.

## Lokální skills

Při odpovídající práci načti příslušný soubor; cesty jsou od kořene repozitáře:

- Průzkum iDokladu, API a exportů: [.agents/skills/idoklad-research/SKILL.md](.agents/skills/idoklad-research/SKILL.md).
- Datový model, peníze, stavy a vazby: [.agents/skills/faktura-domain/SKILL.md](.agents/skills/faktura-domain/SKILL.md).
- Implementace konkrétního modulu a ověření shody: [.agents/skills/faktura-implementation/SKILL.md](.agents/skills/faktura-implementation/SKILL.md).

- repo-structure: [.agents/skills/repo-structure/SKILL.md](.agents/skills/repo-structure/SKILL.md).
- brand-redesign: [.agents/skills/brand-redesign/SKILL.md](.agents/skills/brand-redesign/SKILL.md).
- react-development: [.agents/skills/react-development/SKILL.md](.agents/skills/react-development/SKILL.md).
- rust-backend: [.agents/skills/rust-backend/SKILL.md](.agents/skills/rust-backend/SKILL.md).
- database-migrations: [.agents/skills/database-migrations/SKILL.md](.agents/skills/database-migrations/SKILL.md).
- api-contracts: [.agents/skills/api-contracts/SKILL.md](.agents/skills/api-contracts/SKILL.md).
- faktura-testing: [.agents/skills/faktura-testing/SKILL.md](.agents/skills/faktura-testing/SKILL.md).
- desktop-release: [.agents/skills/desktop-release/SKILL.md](.agents/skills/desktop-release/SKILL.md).

- installation-setup: [.agents/skills/installation-setup/SKILL.md](.agents/skills/installation-setup/SKILL.md). Instalace, první správce a připojení klienta.

## Pravidla návrhu

React, Tailwind, atomic design, tradiční pages/components/utils struktura a monorepo jsou přijatá rozhodnutí. Autoritativní layout, tokeny a distribuci desktopu určuje [DEVELOPMENT_CONVENTIONS.md](docs/architecture/DEVELOPMENT_CONVENTIONS.md); má přednost před starším průzkumem. Desktopové instalátory plánujeme přes GitHub Releases.

Vývoj a úpravy mají provádět převážně agenti. Udržuj explicitní hranice modulů, typované kontrakty, reprodukovatelné příkazy a automatické kontroly. Rust je zvolený jazyk backendu; nepřehodnocuj jej jen kvůli snazšímu vstupu lidských přispěvatelů. Agentní autorství nemění požadavky na správnost, testy a kontrolu změn.

Peníze, množství a kurzy počítej v desetinné aritmetice. Konkrétní přesnost a zaokrouhlení musí mít specifikaci. Odděl kalendářní data dokladů od časových okamžiků auditu. Každá operace nad firemními daty ověřuje přístup k dané firmě, včetně příloh a návazných entit.

Web, API a MCP používají společné příkazy, validace a výpočty. MCP nesmí obcházet oprávnění ani přímo zapisovat do DB. Vystavení a odeslání jsou oddělené operace. Opakování požadavku nesmí vytvářet duplicitní doklady ani úhrady.

Změna kontaktu nebo ceníku nesmí zpětně měnit historický obsah vystaveného dokladu. Úpravy dokladů eviduj v auditu; přesná pravidla oprav a mazání vycházejí ze specifikací.

## Navigace a ověření

- [Implementační plán](docs/IMPLEMENTATION_PLAN.md)
- [Datový model](docs/DATA_MODEL.md)
- [Zdroje a zjištění](docs/research/EVIDENCE.md)
- [Průzkumné scénáře](docs/research/SCENARIOS.md)
- [Specifikace faktur](docs/specs/issued-invoices.md)

Backend a doménové jádro budou v Rustu, databáze je PostgreSQL (rozhodnutí uživatele 10. 10. 2026). Frontend používá React a Tailwind. Desktopový runtime je Tauri (potvrzeno 10. 10. 2026). Použité knihovny a verze jsou v manifestech a lockfiles; licence projektu zatím není zvolena.

Lokalizace je součástí každé nové produktové funkce: čeština (`cs`) a angličtina
(`en`), výchozí čeština. Texty včetně validací a přístupných názvů patří do
katalogů; formátování podle locale. Základ L10N-01 je implementovaný; postup v `docs/LOCALIZATION.md`.
Pravidla a rozlišení jazyka UI/dokladu jsou v
[konvencích lokalizace](docs/architecture/DEVELOPMENT_CONVENTIONS.md#lokalizace).

## Vývojové příkazy

Podrobnosti a požadavky: [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md). Z kořene repa: `pnpm db:up`, `pnpm dev`, `pnpm build:web`, `pnpm check`, `pnpm test`. E2E: s běžícím API `pnpm test:e2e`. Desktop: s běžícím API `pnpm dev:desktop` (spouští vlastní Vite). Build desktopu: `pnpm build:desktop`. Rust musí být v PATH; workspace kontroly vyžadují Tauri systémové závislosti. Dokumentaci piš česky, identifikátory modelu a kódu anglicky.

P02: auth kontrakt `contracts/auth.openapi.json`, generování `pnpm generate:api`.
Databázové auth testy `DATABASE_URL=... pnpm test:auth` potřebují testovací PostgreSQL
s CREATEDB. Instalační návod, recovery a zálohy: `docs/INSTALLATION.md`. Před prvním
spuštěním `node scripts/init-secrets.mjs`; tajemství z `.secrets` nikdy necommitovat.

P03: `docs/COMPANIES.md`, specifikace `docs/specs/company-setup.md`, kontrakt
`contracts/company.openapi.json`. `pnpm generate:api` generuje všechny API kontrakty.
`DATABASE_URL=... pnpm test:companies` vyžaduje izolovaný PostgreSQL s CREATEDB.
MCP stdio: `faktura-server mcp SESSION_FILE`; relaci vytváří `mcp-login` podle návodu,
nikdy nevypisovat token. Instalační admin nesmí obcházet firemní členství.

P04: `docs/specs/contacts.md`, `contracts/contact.openapi.json`; `pnpm generate:api`
regeneruje klienta i MCP schémata (`contracts/contact.tools.json`). PostgreSQL
regrese: `DATABASE_URL=... pnpm test:contacts`. UI regrese: `tests/e2e/contacts.spec.ts`.

P05: `docs/specs/invoice-calculation.md`, `contracts/invoice.openapi.json`;
`pnpm generate:api` generuje klienta i invoice.tools.json. Doménové příklady:
`cargo test -p faktura-domain invoice`; REST/MCP shodu a profilová oprávnění
ověřuje `pnpm test:companies`. Numerická shoda s iDokladem není potvrzena.
UX podklad P06: `docs/design/invoice-editor-ux.md`.

P06: `docs/specs/invoice-drafts.md`, `contracts/invoice.openapi.json`, generovaný
`contracts/draft.tools.json`. DB regrese: `DATABASE_URL=... pnpm test:invoices`;
UI: `tests/e2e/invoices.spec.ts`. Návrhy nepřidělují konečné číslo ani nevystavují PDF.

P07: `docs/specs/invoice-issue.md`, migrace 008, REST/MCP vystavení a PDF.
`pnpm test:invoices` ověřuje transakční číslování, opakování, neměnnost, oprávnění,
PDF a upgrade P06. PDF renderer/fonty v `crates/infrastructure/src/invoice_pdf.rs`
a `crates/infrastructure/assets`; žádné externí I/O při renderu.

P08: `docs/specs/invoice-payments.md`, migrace 009. `pnpm test:payments`
ověřuje úhrady, opravy, souběh, zůstatky, MCP/REST a obnovu izolované DB/PDF.
Obnovovací test vyžaduje místní dev Compose; nikdy jej nepoužívat proti produkci.

P09: `docs/specs/data-migration.md`, `contracts/import.openapi.json`, migrace 010.
`pnpm test:imports` vyžaduje izolovaný PostgreSQL; parsery také běžné cargo test.
UI: `tests/e2e/import.spec.ts`. Předvolby služeb nejsou ověřené adaptéry;
žádnou syntetickou fixture neoznačovat za skutečný export. `pnpm generate:api`
regeneruje importní klienta i MCP tools. Historické součty se nepřepočítávají.

P12-01: `docs/specs/catalog.md`, `contracts/catalog.openapi.json`, migrace 011.
`pnpm test:catalog` používá izolované DB; `tests/e2e/catalog.spec.ts` ověřuje CS/EN,
revizní konflikt, archiv/obnovu, role a uloženou kopii položky ve faktuře.
Ceník nedělá skladové pohyby; výběr ceny provádí Rust přes REST/MCP catalog_line.
Všechny chybějící moduly P10–P20 jsou uživatelem zahrnuté do aktivního rozsahu.


P12-02/P12-03: `docs/specs/sales-documents.md`, `contracts/sales.openapi.json`,
migrace 012. `pnpm test:sales` ověřuje izolované DB, souběh, role, REST/MCP,
upgrade a převod do vystavitelné faktury. UI: `tests/e2e/sales.spec.ts`.
Převod vytváří návrh, nic neodesílá; nabídka má jeden navázaný návrh,
šablonu lze použít opakovaně. Důkazy E-025/E-026 mají popsané limity shody.

QUOTE-DELIVERY-01: nabídky PDF/e-mail, migrace 013, specifikace
`docs/specs/sales-documents.md`. `pnpm test:delivery` používá izolovanou DB a
SMTP capture na loopback. Nikdy testy neposílat skutečným zákazníkům.
MCP sales_send je externí akce, nikoli preview. SMTP setup v instalačním návodu.

QUOTE-SHARE-01: veřejné aktuální nabídky, migrace 014, contracts/public-quote.openapi.json.
pnpm test:shares ověřuje izolovanou DB a loopback SMTP. PUBLIC_WEB_URL nastavuje
instalace, nikdy jej neodvozovat z nedůvěryhodného Host. Celá URL je bearer secret;
nepřidávat tokeny do logů ani skutečných referenčních screenshotů. E-028 odlišuje
doložené chování od našich pravidel a limitů sledování.

INVOICE-DELIVERY-01/P10-01: docs/specs/invoice-delivery.md, migrace 015,
contracts/invoice-delivery.openapi.json a generované invoice-delivery.tools.json.
pnpm test:invoice-delivery vyžaduje izolovanou DB a používá loopback SMTP;
regrese sdílené vrstvy: test:delivery + test:shares, invoice/sales E2E.
MCP invoice_send je externí akce. Žádné testování odesláním zákazníkům.

SMTP-SETUP-01/P10-02: docs/specs/smtp-settings.md, migrace 016 a kontrakt smtp.openapi.json.
Po init-secrets ověřuj `DATABASE_URL=... pnpm test:smtp` a tests/e2e/smtp.spec.ts.
Pouze správce instalace; nikdy nevracet ani logovat heslo. Testovací odeslání je
externí akce; automatické testy smí použít pouze lokální SMTP se syntetickými adresami.

ATTACHMENTS-01/P10-03: docs/specs/document-attachments.md, migrace 017,
contracts/attachment.openapi.json a generované tools/klient. Test:attachments
vyžaduje izolovanou PostgreSQL a loopback SMTP. Regrese delivery/invoice_delivery/share;
UI invoices/sales. Byty jsou v DB; neodesílat přílohy skutečným zákazníkům v testech.
