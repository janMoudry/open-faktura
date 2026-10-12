# Datový model Open Faktury

Konceptuální návrh v0.1, 10. 10. 2026. **Není to rekonstruované databázové schéma iDokladu ani hotová migrace.** Externí kontrakty E-001 až E-007 jsou v [evidenci](research/EVIDENCE.md). Všechny interní entity, typy a omezení níže jsou návrhem; u tabulek uvádíme podklad pro jejich funkční potřebu.

Databáze: **PostgreSQL**, zvoleno 10. 10. 2026. Konceptuální model bude rozpracován do jejího schématu; konkrétní verze, přesnosti čísel a migrace zatím nejsou určeny.

## Typy a společné sloupce

`Id` je neprůhledný interní identifikátor; konkrétní formát zvolíme se stackem. `Decimal` je přesné desetinné číslo; přesnost a scale určit podle Q-01 a importních limitů. API může desetinné hodnoty přenášet řetězcem, aby nedošlo ke ztrátě přesnosti. `Date` je kalendářní datum, `Instant` UTC okamžik. `CurrencyCode` je kód měny; externí `CurrencyId` se mapuje přes číselník.

Firemní entity nesou `id`, `company_id`, `created_at`, `updated_at`, `revision`. Vazby mezi nimi musí zachovat stejnou firmu, v DB ideálně složeným cizím klíčem. Povinné sloupce v následujících tabulkách nemají `?`; `?` znamená možnou absenci. Doménově podmíněná povinnost je uvedena zvlášť. Unikátnost IČO partnera nepředpokládáme.

## Instalace a identity

Bootstrap, globální přihlašovací údaje, relace a průběh založení firmy rozpracovává
[návrh instalace a onboardingu](specs/installation-onboarding.md#model-a-migrační-dopad--návrh).
Instalační oprávnění je oddělené od firemního členství. P02 již implementuje
`installation_state`, `app_user`, `auth_session`, `auth_rate_limit` a
`installation_audit` v migraci `202610100002_auth.sql`; P03 přidává níže popsané firemní tabulky; ostatní účetní entity zůstávají návrhem.

`app_user.id` je interní bigint (API jej vrací jako řetězec), e-mail je normalizovaný
a unikátní; heslo je pouze PHC hash. `credential_version` serializuje obnovu hesla
se souběžným přihlášením. `installation_admin` není firemní role. Relace má hash
náhodného tokenu, FK na uživatele a absolutní expiraci; při recovery se maže.
Stav bootstrapu je jediný zamykatelný záznam. První účet, jeho audit a uzavření
instalace se commitují společně. Model žádná skutečná ani demo účetní data nezakládá.

## Jádro a kontakty

| Entita | Klíčová pole | Vazby a pravidla | Podklad |
| --- | --- | --- | --- |
| Company | name, registration_number?, vat_number?, country_code, base_currency, tax_profile | Daňový profil verzovat podle účinnosti; konkrétní režimy ještě specifikovat | FEATURES §14 |
| User | id, email, authentication_subject | Globální identita, bez company_id; autentizace není účetní agenda | FEATURES §14 |
| Membership | company_id, user_id, role_id | Unikátní dvojice firma/uživatel; oprávnění jsou vlastní návrh | FEATURES §14, Q-05 |
| Contact | name, registration_number?, vat_number?, billing_address, maturity_days?, discount_percent?, reminders_enabled? | Firma 1:N kontakty; IČO a DIČ textově, zachovat nuly | E-005 |
| ContactAddress | contact_id, kind, address, is_default | Kontakt 1:N dodací adresy | E-005 |
| ContactBankAccount | contact_id, account_number?, bank_code?, iban?, bic? | Validovat podporované kombinace; minimálně jeden identifikátor účtu | E-005 |
| CompanyBankAccount | currency, account_number?, bank_code?, iban?, bic?, is_default | Výchozí účet podle upřesněných pravidel; nepřebírat tarifní limit | FEATURES §10 |
| NumberSeries | document_kind, format, period_key, next_value | Atomické přidělení; rozsah unikátnosti období/formátu upřesnit Q-02 | E-001 |

Adresu tvoří `street`, `city`, `postal_code`, `country_code`; povinnost podle země a scénáře upřesnit. Přihlašovací email není automaticky kontaktním emailem firmy.

## Doklady

Navržená společná hlavička `Document` a typované rozšíření pro fakturu/nabídku/zálohu; nepředpokládat, že všechny typy mají shodné stavy a povinná data.

| Entita | Klíčová pole | Pravidla |
| --- | --- | --- |
| Document | kind, lifecycle, partner_id?, number_series_id?, number?, issued_on?, due_on?, tax_on?, currency, exchange_rate?, exchange_rate_amount?, description?, internal_note?, order_number?, variable_symbol?, payment_method?, locale | Návrh může být neúplný; povinnosti při vystavení určuje specifikace typu. Číslo a VS jsou text, ne integer. Kurz i jeho množství patří k sobě. |
| DocumentPartySnapshot | document_id, role, name, registration_number?, vat_number?, address | Dodavatel/odběratel/dodací místo podle typu. Historický obsah nezávislý na živém kontaktu. |
| DocumentPaymentSnapshot | document_id, account_number?, bank_code?, iban?, bic?, constant_symbol? | Historické údaje pro úhradu; změna výchozího účtu je nepřepíše. |
| DocumentLine | document_id, position, kind, name, quantity, unit?, unit_price, price_mode, discount_percent, vat_category, vat_percent?, vat_code?, catalog_item_id?, source_document_id? | 1:N; unikátní pořadí v dokladu. `kind` rozlišuje normální a odpočtové položky; přesný seznam dle typu. |
| DocumentTotals | document_id, calculation_version, net, vat, gross, rounding, base_currency_totals? | Uložení výsledku pro audit; soulad s položkami a rekapitulací ověřuje kalkulátor. |
| DocumentTaxSummary | document_id, tax_key, net, vat, gross | Rekapitulace podle relevantní sazby a režimu; neagregovat pouze podle procenta. |
| DocumentRelation | source_id, target_id, relation_type | Oprava, převod nabídky, vyúčtování; typovaná pravidla zabrání neplatným vazbám a cyklům. |
| AdvanceSettlement | advance_id, final_invoice_id, amount, currency, tax_document_id? | Výše čerpání oddělená od pouhé vazby; souběžné čerpání nesmí překročit dostupnou částku. Pravidla čekají na Q-06. |

Podklad hlavičky/položek: E-001, E-002, E-003. Snapshoty, normalizace a verze kalkulátoru jsou náš návrh. Vazby záloh částečně podporuje E-007; nejsou zatím kompletně specifikované.

### Mapování SDK → interní návrh

| SDK | Interní pole | Poznámka |
| --- | --- | --- |
| PartnerId | partner_id přes ExternalIdentifier | Externí integer není interní ID |
| CurrencyId | currency přes číselník | Neodhadovat měnu z čísla |
| DateOfIssue / DateOfMaturity / DateOfTaxing | issued_on / due_on / tax_on | SDK má DateTime; převod na kalendářní datum ověřit |
| ExchangeRate / ExchangeRateAmount | exchange_rate / exchange_rate_amount | Směr a zaokrouhlení Q-01, R-03 |
| Amount / UnitPrice | quantity / unit_price | `Amount` položky znamená množství |
| PriceType | price_mode | Explicitně WithVat / WithoutVat / OnlyBase; nepřevzít jen boolean |
| VatRateType / VatRate / VatCodeId | vat_category / vat_percent / vat_code | Typ, procento a členění jsou různé údaje |
| NumericSequenceId / DocumentSerialNumber | number_series_id / serial metadata | Formátované číslo nelze získat pouhým přetypováním serialu |
| ProformaInvoices / InvoiceProformaId | DocumentRelation / source_document_id | Finanční čerpání specifikovat samostatně |

## Platby a oddělení stavů

`FinancialTransaction`: `account_id`, `direction`, `amount`, `currency`, `booked_on`, `external_id?`, `source`, `raw_reference?`. Zahrne banku/pokladnu pomocí typované evidence účtu; zdrojový importní identifikátor slouží k deduplikaci.

`PaymentAllocation`: `transaction_id`, `document_id`, `transaction_amount`, `document_amount`, `exchange_rate?`, `effective_on`, `reversal_of?`. Platba může pokrýt více dokladů, doklad více plateb. Transakce a alokace zachovávají firmu; součty alokací se kontrolují v transakci. Vratky a záporné doklady potřebují vlastní specifikaci před implementací.

- Životní cyklus: pracovní návrh `draft → issued`; opravy, koš a další přechody zatím Q-02. Není to doložený enum iDokladu.
- Úhrada: `unpaid`, `partially_paid`, `paid`, `overpaid`; externí hodnoty E-004 mapovat explicitně. Pro kladnou pohledávku návrh odvozuje stav z alokovaných částek a celkové částky. Nulové/záporné doklady zůstávají Q-04.
- Po splatnosti: samostatný odvozený údaj podle data, firemní zóny a neuhrazené částky.
- Odeslání a export: samostatné záznamy, nikoli stavy úhrady či vystavení.

## Navazující moduly — zatím pouze hranice modelu

| Modul | Navržené entity | Vazba / co ještě specifikovat |
| --- | --- | --- |
| Ceník a sklad | CatalogItem, Warehouse, StockMovement | Pohyb odkazuje na řádek dokladu; ocenění a opravy otevřené |
| Nákup a inbox | InboxItem, ExtractionAttempt, Attachment | Originál souboru zachovat; revize vytěžení a vazba na přijatý doklad |
| Automatizace | InvoiceTemplate, RecurrenceSchedule, JobRun | Výskyt plánu má unikátní klíč; časová zóna a retry politika |
| Komunikace | Message, DeliveryAttempt, SharedDocumentLink | Revize dokladu, příjemci, odvolání odkazu, historie pokusů |
| Exporty a daně | ExportBatch, ExportEntry, TaxReport | Export obsahuje revizi dokladu; formulář má období a verzi schématu |
| Integrace | ExternalIdentifier, ImportRun, IntegrationConnection | Unikátní firma/zdroj/externí agenda/typ/externí ID; tajemství odděleně |
| Infrastruktura | AuditEvent, IdempotencyRecord, OutboxEvent | Audit s aktérem a revizí; idempotence v rozsahu firmy/operace/klíče a hash vstupu |

Základní indexy: firemní číslo dokladu v rozhodnutém rozsahu, firma/partner, firma/splatnost, firma/datum vystavení, externí identifikátor, plán/výskyt. Přesné indexy, mazací politiky, částkové scale a kompletní slovník polí všech modulů vzniknou až při jejich specifikaci. Tento návrh vědomě neuzavírá neověřené části.

## Implementované firemní nastavení P03

Migrace `202610100003_companies.sql`: `company` (bigint ID, creator, UUID request,
initial/settings JSONB, revision, step, completed, timestamps), `membership`
(složený PK company/user, owner/editor/viewer, jediný owner), `company_tax_profile`
(company/revision PK, režim a DATE účinnosti) a `company_audit` (company/revision PK,
actor, action, settings snapshot, timestamp). Prázdné volitelné texty jsou `""`;
číselné parametry jsou integer, ne peněžní hodnoty. Detaily validace a transakcí
v [P03 specifikaci](specs/company-setup.md), rozhraní v [provozním návodu](COMPANIES.md).
Jediný výchozí bankovní účet a konfigurace řady jsou zatím součástí typovaného
snapshotu; samostatné entity výše jsou cílový model pro více účtů a doklady P07.
Historie DPH je append-only dle revize; při stejné účinnosti vyhrává novější revize.
Výběr profilu pro konkrétní daňové datum a vystavení doplní P05/P07.

P03 aktualizace 11. 10. 2026: `company.deleted_at timestamptz NULL` označuje logicky
smazané profily. Členství i audit zůstávají; aktivní čtení a zápisy včetně MCP
filtrují smazané profily. Smazání inkrementuje revision a zapíše company_audit
s action=delete ve stejné transakci. Kontrakt v UX-P03-04, nová migrace 202610110004.

ACCOUNT-01 (11. 10. 2026): `app_user.name` je globální zobrazované jméno,
NOT NULL DEFAULT '', max. 120 znaků. Prázdná hodnota zachovává dosavadní účty.
Změny přihlašovacích údajů zvyšují credential_version, ruší ostatní relace
v transakci a auditují account_credentials_changed; změna samotného jména
používá account_updated. Viz [specifikace účtu](specs/account-settings.md).

P04 / CONTACT-01–09 (11. 10. 2026): implementovaný `contact` obsahuje company_id,
creator_id, request_id, initial_data, data JSONB, revision a archived. Adresy,
osoby a příjemci jsou součástí jednoho verzovaného agregátu; koncepční řádky
ContactAddress/ContactBankAccount výše nejsou tvrzením o samostatných SQL tabulkách.
`contact_audit` ukládá každou revizi včetně archivace/obnovy a aktéra; složený FK
vynucuje profilovou vazbu. Kontrakt, povinnosti a nulové hodnoty:
[specs/contacts.md](specs/contacts.md). Žádné fyzické mazání a žádné přečíslování
externích identifikátorů. Budoucí doklad musí uložit historický snapshot kontaktu.

## P05 — implementovaný doménový základ

`crates/domain/src/invoice.rs` obsahuje Calculation/Line/Totals a vlastněné
PartySnapshot/PaymentSnapshot/DraftData. Decimal výpočty a omezení viz
[specifikace](specs/invoice-calculation.md). Desetinné hodnoty jsou na hranici
API řetězce; uložení návrhů přidává P06. Nová DB migrace není součástí P05.

## Implementované návrhy P06

`invoice_draft` a `invoice_draft_audit` (migrace 007) ukládají revizované
snapshoty stran, položek a platebních údajů, nullable vypočtené součty, autora
a časové údaje. UUID příkazu je unikátní v profilu, vazba na kontakt je
složená přes profil. Detaily a hranice: [specifikace návrhů](specs/invoice-drafts.md).

## Implementované vystavení P07

Migrace 008 rozšiřuje agregát `invoice_draft` o number, issued_at, issue_request_id,
issue_actor_id, issue_revision a nullable PDF bytea cache. Unikátní profil/číslo
 a profil/klíč vystavení; DB CHECK drží konzistenci nullable stavových polí.
`invoice_number_series` (profil/prefix/rok, next_value 1–1000000) chrání historicky
spotřebovaná čísla. Audit rozšířen o action=issue a number. DB trigger zakazuje
změnu vydaného obsahu i jeho smazání. Kontrakt a akceptace: [P07](specs/invoice-issue.md).

## Implementované úhrady P08

Migrace 009: invoice_payment (profil/faktura, NUMERIC(20,2), datum, metoda,
poznámka, aktér/čas), invoice_payment_reversal (původní úhrada, důvod, aktér/čas,
nullable replacement_id), invoice_payment_command (profil/UUID, faktura, aktér,
vstup a přesný výsledek idempotentního příkazu). Tabulky jsou append-only a FK
udržují shodný profil i fakturu. SQL pohledy invoice_settlement a
invoice_with_settlement počítají aktuální zůstatek, stav a splatnost; nemění
historický doklad/PDF. Payment revize je počet příjmů a storen, nikoliv revize
dokladu. Detaily, nuly, opravy a podporovaný rozsah: [P08](specs/invoice-payments.md).

## P09 — staging a původ importovaných dat

Migrace 010 přidává import_batch (profil, autor, zdroj, agenda, neměnný vstup),
import_row (pořadí, normalizovaný snapshot, stav, chyby, cílové ID) a
import_identity (unikátní profil/zdroj/agenda/typ/původní ID, původní snapshot,
FK na kontakt nebo fakturu). Kompozitní FK brání vazbě do jiného profilu.
Identita je append-only; běžná pozdější úprava kontaktu nemění zdrojový snapshot.
Historické doklady vstupují přímo jako neměnné vystavené invoice_draft se
zdrojovým číslem a dodanými součty `import-v1`; nečerpají číselnou řadu. Auditní
čas importu není původní čas vystavení. Případné jednotlivé úhrady se uloží ve
stejné transakci. Detaily a limity: [specifikace P09](specs/data-migration.md).

## P12-01 — ceník

`catalog` (migrace 011): ID a company_id, creator_id, unikátní request_id v rámci
profilu, normalizovaný initial_data pro idempotenci, aktuální data JSON, revize,
archived a časové údaje. `catalog_audit` má složený FK company_id/catalog_id,
unikátní revizi a atomický záznam create/update/archive/restore s aktérem.
Data: název, kód, EAN, jednotka, výchozí množství, jednotková cena jako desetinný
řetězec, CZK, net/gross, sazba. Převzetí do dokladu kopíruje hodnoty přes
aplikační příkaz; nejde o živou vazbu. Skladový identifikátor a pohyby jsou P18.
Podrobná pravidla: [catalog.md](specs/catalog.md).


## P12-02/P12-03 — šablony a cenové nabídky

Migrace 012: `sales_document` rozlišuje template/quote, vlastní snapshot
DraftData a název, splatnost a obchodní stav. `quote_sequence` čísluje v profilu
po letech. `sales_audit` zaznamenává každou revizi a aktéra. `sales_conversion`
uchovává idempotentní příkaz, aktéra a vazbu na výsledný návrh faktury.
Vazby mají složený FK `(company_id,id)`. Šablona může založit více návrhů;
nabídka nejvýše jeden. Kopie a audit vznikají v jedné transakci. Odkaz neznamená
vystavení: konečné číslo a PDF faktury stále vytváří samostatný P07 příkaz.

### Historie odesílání nabídek

Migrace 013: `quote_delivery` patří firmě a nabídce přes složený cizí klíč.
Uchovává idempotentní příkaz, autora, revizi, snapshot, neměnné PDF a adresovaný
obsah zprávy. Mění se pouze stav pokusu a jeho čas/výsledek. Index fronty pro
queued a index historie podle firmy/dokladu. SMTP nikdy neběží pod transakčním
zámkem; nejistý výsledek nelze automaticky opakovat. Viz QUOTE-DELIVERY-01.

### QUOTE-SHARE-01 (migrace 014)

quote_share: profil/doklad/autor, idempotentní request_id, revize při vytvoření,
expires_days, SHA-256 token_hash, důvěrná URL, created_at/expires_at/revoked_at,
revoked_by. URL zpřístupňuje aktuální obsah; revize není obsahový snapshot.
quote_view: FK (company_id,document_id,share_id), jedinečné (share_id,visit_id),
klientem hlášená revize a DB viewed_at. Bez IP a osobního identifikátoru.
quote_delivery.share_id je nullable složený FK do stejného profilu/dokladu.
NULL znamená původní PDF přílohu; hodnota znamená link e-mail.
Statistika návštěv vychází z quote_view, ne ze SMTP stavu.

### INVOICE-DELIVERY-01 (migrace 015)

invoice_delivery ukládá poštovní frontu a historii vystavených faktur. Složený FK
(company_id, document_id) odkazuje na invoice_draft; request_id je unikátní v profilu.
Obsahuje neměnný snapshot, přesné uložené PDF, obsah zprávy, adresy, Message-ID,
stav a časy. SMTP běží mimo transakci. Stavy odpovídají quote_delivery; žádné
změny vydané faktury ani jejích úhrad. Osobní data a PDF jsou součástí DB záloh.

### SMTP-SETUP-01 (migrace 016)

smtp_settings je singleton instalace, pouze pro installation_admin. Revize brání
ztrátě souběžných změn. Tajemství je AEAD ciphertext; klíč je mimo DB. Audit zápisu
v installation_audit bez tajemství. smtp_test ukládá request_id, aktéra, revizi,
příjemce, jazyk a výsledek; žádná faktura nebo úhrada se testem nemění.
Migrace přidává tabulky; dosavadní doručovací fronty a SMTP z prostředí zachovává.

### ATTACHMENTS-01 (migrace 017)

Document_attachment patří přes složený FK buď invoice_draft, nebo sales_document;
pro nabídky aplikace vyžaduje kind=quote. Obsah bytea má limit 2 MiB, neměnná
metadata SHA-256 a idempotentní request_id. Soft-delete uvolní byty pouze bez
historické vazby. Delivery_attachment váže soubor na invoice_delivery nebo
quote_delivery; firemní FK brání odkazu mimo profil. Historie navíc ukládá JSON
metadata výběru. Attachment_audit eviduje upload/delete s aktérem. Quota 256 MiB
zahrnuje všechny uchované byty profilu. Záloha DB obsahuje i soubory.
