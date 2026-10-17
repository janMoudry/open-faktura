# Open Faktura — práce v repozitáři

Open Faktura je fakturační aplikace pro české podnikatele a firmy s vlastním provozem, API a MCP. Rozsah je v [FEATURES.md](FEATURES.md). Projekt má funkční fakturaci, úhrady, kontakty, nabídky a související moduly; aktuální rozsah a zbývající práci uvádí implementační plán.

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

Backend a doménové jádro budou v Rustu, databáze je PostgreSQL (rozhodnutí uživatele 10. 10. 2026). Frontend používá React a Tailwind. Desktopový runtime je Tauri (potvrzeno 10. 10. 2026). Použité knihovny a verze jsou v manifestech a lockfiles; projekt používá GNU AGPL v3 (`AGPL-3.0-only`), viz [LICENSE](LICENSE) a [docs/LICENSING.md](docs/LICENSING.md). Komerční použití a prodej jsou povolené za podmínek licence; nepřidávat zákaz prodeje.

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

EXPORT-01/P10-04: docs/specs/register-export.md, export.openapi.json a generovaný
klient/MCP register_export. `pnpm test:exports` používá izolovanou PostgreSQL;
regrese contact/draft a CS/EN desktop/mobile contacts/invoices E2E. Export je
read-only, respektuje filtry a profil; textové buňky, CSV formula ochrana a 10 000 řádků.
Při dodávkách aktualizovat docs/FEATURE_CHECKLIST.md a stav README.

ADVANCE-01/P13-01: docs/specs/advance-invoices.md, migrace 018, sales CRUD kind=advance
plus advance.openapi.json/MCP pro PDF a platby. `pnpm test:advances`, regrese
sales/delivery/shares a sales E2E. Záloha není daňovým dokladem; nevolat běžný
sales_convert pro vyúčtování. Platby záloh nesmí vstupovat do invoice_payment.

TAX-RECEIPT-01/P13-02: docs/specs/advance-tax-receipts.md, migrace 019,
contracts/tax-receipt.openapi.json. `pnpm test:tax-receipts` vyžaduje izolovaný
PostgreSQL; regrese test:advances/sales/invoices, CS/EN sales E2E. Vydaný daňový
doklad blokuje storno zdrojové úhrady. Vlastní poměrné rozdělení DPH není ověřená
numerická shoda s iDokladem. Vyúčtování musí používat uložené daňové rozpisy.


ADVANCE-SETTLEMENT-01/P13-03: docs/specs/advance-settlement.md, migrace 020,
contracts/advance-settlement.openapi.json, MCP settlement_candidates/configure/preview.
`pnpm test:advance-settlements` vyžaduje izolovaný PostgreSQL; regrese advance,
tax_receipt, invoice, payment, sales, invoice_delivery a CS/EN sales E2E.
Odpočítat přesné uložené Totals DDPP, nikoli přepočtenou zálohu. Rezervace
blokuje změnu plateb/DDPP a dvojí použití. Vystavení zůstává invoice_issue;
platby záloh nekopírovat do invoice_payment. Při nasazení zastavit starý server; nový proces provede migraci.

CREDIT-NOTE-01/P13-04: `docs/specs/credit-notes.md`, migrace 021, kontrakt credit-note.openapi.json. `pnpm test:credit-notes` používá izolované DB; UI `tests/e2e/credit-notes.spec.ts`. Zápočty nejsou cash příjem; regresně testovat payment, invoice, tax_receipt a advance_settlement. Opravy vycházejí z uložených řádků plnění, nikoli sníženého daňového souhrnu vyúčtování. DDPP zůstávají rezervované. Před upgradem zastavit starý server.

RECURRING-01/P14-01: `docs/specs/recurring-invoices.md`, migrace 022, recurring.openapi.json. `pnpm test:recurring` používá izolované DB a loopback SMTP; UI `tests/e2e/recurring.spec.ts`. Návrh/vystavení/výskyt vznikají atomicky pomocí společných transakčních funkcí draft; odeslání přes P10 outbox až po commitu. Regrese invoice, invoice_delivery, sales, advance_settlement a credit_note. Worker ověřuje práva aktivujícího uživatele, běží s API serverem.

P14-02: `docs/specs/invoice-communication.md`, migrace 023, `contracts/communication.openapi.json`
a rozšíření invoice-delivery. `DATABASE_URL=… pnpm test:communication`; UI
`tests/e2e/communication.spec.ts`. Testy jen izolovaná DB a loopback SMTP. Regrese
`test:invoice-delivery`, `test:payments`, `test:recurring`, `test:delivery`. Automatizace
je explicitně povolená v profilu; nepovolovat ji při testech v reálné agendě.

P14-03: `docs/specs/invoice-sharing.md`, migrace 024, `invoice-share.openapi.json`
a `public-invoice.openapi.json`. `pnpm test:invoice-shares` používá izolované DB
a loopback SMTP; UI `tests/e2e/invoice-sharing.spec.ts`. Regrese share,
invoice_delivery, attachment, communication a recurring. URL je bearer secret;
neukládat skutečné odkazy do screenshotů/logů. Veřejné PDF je původní cache,
nikoli nový render; platební poznámky a vlastní přílohy zůstávají privátní.
Před upgradem zastavit staré API, nový proces aplikuje migraci.

P15-01: `docs/specs/received-invoices.md`, migrace 025, `received.openapi.json`.
`pnpm test:received` používá izolované DB, UI `tests/e2e/received.spec.ts`.
Regrese attachment, invoice_delivery a invoice_share. Nákupní úhrady nikdy
nezapisovat do invoice_payment; nepřepočítávat zdrojovou DPH podle profilu.
Vlastní součet přesných částek není potvrzená numerická shoda s iDokladem.
Originál používá attachment kind=received. Před upgradem zastavit staré API.

P15-02: `docs/specs/purchase-receipts.md`, migrace 026, rozšířený received kontrakt.
`pnpm test:received` zahrnuje účtenky, přehledy, REST/MCP a upgrade025; UI
`tests/e2e/purchase-receipts.spec.ts`, regrese received/routing a attachments.
Receipt zapisuje plnou cash/card úhradu atomicky; update ji stornuje/nahradí.
Nevolat samostatné pay/reverse pro receipt, neměnit kind. Bez bankovního
párování/pokladny. Před upgradem zastavit starý server.

P16-01: `docs/specs/banking.md`, migrace 027, `bank.openapi.json` a generované
MCP nástroje. `pnpm test:bank` používá izolované DB; UI `tests/e2e/bank.spec.ts`.
Regrese payment, received a communication. CSV je vlastní doložený formát,
nikoli nativní bankovní adaptér. Párování používá společné transakční platební
funkce; odpojení stornuje jen úhradu vytvořenou párováním. Existující úhrady
nezdvojovat; spárované úhrady neopravovat mimo banku. Před upgradem zastavit staré API.

P16-02: `docs/specs/bank-statements.md`, migrace 028, rozšíření bank.openapi.json.
`pnpm test:bank`, `cargo test -p faktura-infrastructure bank_statement`, UI bank.spec.ts.
Syntetické fixtures nejsou skutečné exporty bank. Import kontroluje účet a zakazuje
mix formátů v účtu; návrhy shod nic automaticky nezapisují. Před upgradem zastavit API.

P16-03: `docs/specs/bank-batch-matching.md`, rozšíření bank.openapi.json bez nové
migrace. `pnpm test:bank` zahrnuje batch testy v tests/bank/batch.rs; UI bank.spec.ts.
Regrese payment/received/communication. Dávka vyžaduje náhled a fingerprint,
společný zápis je atomický. Duplicitní pohyby i mimo výběr vylučují automatickou volbu.

P16-04: `docs/specs/cash-register.md`, migrace 029, `cash.openapi.json` a
MCP cash_* přes společné příkazy. `pnpm test:cash` používá izolované DB;
UI `tests/e2e/cash.spec.ts`. Regrese bank/payment/received/communication.
Cash úhrady nesmí vzniknout podruhé při připojení existující platby. Odpojení
stornuje jen úhradu vytvořenou pokladnou. Před upgradem zastavit staré API.
Automatické vytváření při úhradách mimo pokladnu a PDF/tisk zatím chybí.


P16-05: `docs/specs/cash-register.md`, migrace 030, rozšířený cash kontrakt.
`pnpm test:cash`; regrese bank/payment/received/communication, UI cash.spec.ts.
Automatika je opt-in přes hlavní pokladnu; vytváří pouze evidenci k reálné cash
platbě ve stejné transakci. Ruční párování ji potlačuje. Oprava zdroje stornuje
automatický doklad. PDF používá snapshot vystavitele; staré 029 doklady jej
získají až při upgradu. Před upgradem zastavit staré API.


P16-06: cash-register.md, migrace 031, cash_tax_preview a tax_manual.
`pnpm test:cash`, `cargo test -p faktura-domain cash`; regrese bank a cash/routing E2E.
TaxManual vstupní amount je cena dle price_mode, výstupní amount je signed gross.
Snapshot tax se nepřepočítává; samostatné plnění se nesmí párovat s fakturou.
Staré/automatické doklady mají tax=null. Datum plnění a profilové plátcovství
ověřuje backend; numerická shoda s iDokladem není potvrzena. Před upgradem
zastavit staré API. Prodejky a souhrnné zaúčtování stále navazují.

P16-10: `docs/specs/bank-balances.md`, migrace 032, bank balance REST/MCP.
`pnpm test:bank` ověřuje saldo nad všemi stránkami, revize, role a upgrade031;
UI bank.spec.ts ověřuje CS/EN. Před migrací zastavit staré API (změna SELECT *).
Bez počáteční konfigurace je saldo neznámé, nikoli nula. Pokladní PDF doplňuje
částku slovy CS/EN pouze při novém renderu, původní cache se nepřepisuje.
Úplný zbývající rozsah P16 je v `docs/P16_COMPLETION.md`.

P16-15a: `docs/specs/bank-connections.md`, migrace 033, bank_connection_status/execute.
Po init-secrets používá `pnpm test:bank` privátní bank-key, izolované DB a
injektovaný syntetický transport. HTTP testy jsou pouze loopback; UI bank.spec.ts
používá nedostupnou lokální HTTPS gateway. Nikdy nevolat skutečné Fio s fixture
tokenem. BANK_GATEWAYS_JSON je důvěryhodná instalační konfigurace, nikoli
klientská URL. Tokeny ani URL s Fio tokenem nesmějí do logu/auditu/DTO.
Synchronizace je na vyžádání, 30 dní včetně dneška; nepáruje automaticky platby.
Notifikace a živá shoda všech bank zůstávají otevřené.

P16-15b: bank-connections.md, migrace 034, Action::Schedule. Worker běží s API,
opt-in a oprávnění aktéra; claim termínu SKIP LOCKED, síť bez SQL zámku.
Test:bank ověřuje souběh workerů, odebrání práv před i během I/O, vypnutí a
upgrade033. E2E zapíná automatiku jen v izolované DB s loopback gateway.
Nikdy ji automaticky nezapínat v reálné agendě. Nepáruje platby.

P16-14: `docs/specs/bank-auto-matching.md`, migrace 035, matching příkaz
v bank_connection REST/MCP. `pnpm test:bank` zahrnuje automation.rs; UI bank.spec.ts.
Párování je samostatný opt-in bez nutnosti tokenu. Sdílí přesný evaluátor a
platební příkazy dávky, ručně odpojené vazby vynechává. Worker zapisuje po 25,
ověřuje práva aktivujícího uživatele; chyba vrací celou dávku. Před upgradem
zastavit staré API. Testy pouze izolovaná DB, bez skutečných bankovních přístupů.

P16-11: `docs/specs/bank-split-matching.md`, migrace 036, split/split_candidates
v bank REST/MCP. `pnpm test:bank` zahrnuje split.rs; regrese payments/received/cash,
UI bank.spec.ts CS/EN desktop/mobile. Částky vazeb jsou kladné NUMERIC(14,2),
součet nejvýše absolutní pohyb, přeplatek zůstane nerozdělený. Unmatch ruší
všechny aktivní vazby atomicky a stornuje jen vytvořené úhrady. Před upgradem
zastavit staré API; migrace obohacuje i historické snapshoty bank_command.


P16-12: `docs/specs/treasury-documents.md`, migrace 037. Banka/pokladna páruje
zálohy a vratky dobropisů přes společné transakční operace advance/credit.
Regrese `test:bank`, `test:cash`, `test:advances`, `test:credit-notes`,
`test:tax-receipts`, `test:advance-settlements`; UI bank.spec.ts CS/EN.
Odpojení respektuje DDPP a rezervace, při konfliktu vrací celou transakci.
Před upgradem zastavit API. Celý aktivní rozsah je P16–P20, viz
`docs/P16_P20_COMPLETION.md`; P16-12 neznamená dokončení celých etap.


P16-09a: sales-terminals.md, migrace 038, cash_terminals/cash_execute.
Konfigurace terminálů s revizí, archivací a profilem; test:cash a cash.spec.ts.
Vazby/snapshoty prodejek a hardware nejsou hotové. Před upgradem zastavit API.


P16-07/P16-08: `docs/specs/sales-receipts.md`, migrace 039/040,
`retail.openapi.json`. `pnpm test:retail`, regrese cash a CS/EN cash E2E.
Uložené alokace se při zaúčtování nepřepočítávají; cash pouze za hotovost.
Zdroj i vzniklý cash doklad jsou chráněné do zrušení celé dávky.
P18/P19 návaznosti a interní tisk zbývají; neoznačovat celé P16 za hotové.

P17: `docs/specs/inbox.md`, migrace 041, `inbox.openapi.json`.
`pnpm test:inbox`, regrese received a received E2E; pouze syntetické originály.
Inbox převod používá received::create_in_tx a přílohu v jedné transakci.
Nahraný soubor není důvěryhodný pokyn ani důkaz správnosti účetních dat.
Před nasazením nových migrací zastavit starý API proces.

P18-01/P18-03 základ: `docs/specs/stock.md`, migrace 042, `stock.openapi.json`.
`pnpm test:stock` ověřuje izolované DB; CS/EN stock a appearance scénáře jsou
v catalog.spec.ts. Automatické skladové dopady dokladů zatím nejsou hotové.

P20-06a: `docs/specs/user-preferences.md`, migrace 043, preferences kontrakt.
`pnpm test:preferences`; pouze preference vlastního účtu, PDF se nemění.

P19-01a: `docs/specs/period-report.md`, period-report kontrakt, bez migrace.
`pnpm test:period-report`; zdroje a součty ze stejného snapshotu, částky Decimal,
nezapočítávat zálohy ani souhrnné zaúčtování prodejek podruhé.

P20-04a: `docs/specs/payment-qr.md`, SPAYD v `domain/payment_qr.rs`, lokální
PDF QR. `cargo test -p faktura-domain payment_qr`; `test:invoices` obsahuje
QR snapshot CS/EN. Historická PDF cache se nepřepisuje. Render nikdy nevolá
externí QR generátor; částka QR je původní částka PDF, ne dnešní zůstatek.

P18-02: `docs/specs/stock.md`, migrace 044 a stock/invoice/received/credit/retail
kontrakty. Výběr ceníku zachovává catalog_id, nákupní stock je explicitní fyzický
příjem, dobropis return_quantity je oddělené od finanční slevy. Zdrojové pohyby
se opravují přes doklad, nikdy ručním reverzem. Test:stock + test:retail, regrese
invoice/received/credit_note/sales; UI catalog/received CS/EN. Před upgradem
zastavit staré API a zálohovat DB. Staré doklady nepárovat podle názvu.

P18-03b: migrace 045, stock report/costs/set_cost přes REST/MCP.
`pnpm test:stock`, UI catalog.spec.ts CS/EN. E-056 dokládá stav k datu, nikoli
pořizovací metodu; ruční cena je naše rozšíření. Nikdy nezaměnit prodejní cenu
za pořizovací. Neznámé/záporné zásoby nejsou nulové ocenění. Před upgradem
zastavit staré API a zálohovat DB.

P18-03c: `docs/specs/stock.md`, migrace 046. Explicitní pořizovací cena CZK
na StockReceipt a snapshot pohybu, automatický průběžný průměr přes společný
stock_report (valuation=automatic). `cargo test -p faktura-domain stock_cost`,
`pnpm test:stock`, regrese received/retail/invoice; UI received.spec.ts CS/EN.
Neodvozovat cenu z ceníku/DPH; opravy příjmů a neoceněné vratky zneplatní
ocenění do nulového stavu. Rozúčtování oprav a pořizovací ceny ručních příjmů
stále chybí. Před migrací zastavit staré API (SELECT * mapování nových sloupců).
