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

| Entita             | Klíčová pole                                                                                                    | Vazby a pravidla                                                            | Podklad            |
| ------------------ | --------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- | ------------------ |
| Company            | name, registration_number?, vat_number?, country_code, base_currency, tax_profile                               | Daňový profil verzovat podle účinnosti; konkrétní režimy ještě specifikovat | FEATURES §14       |
| User               | id, email, authentication_subject                                                                               | Globální identita, bez company_id; autentizace není účetní agenda           | FEATURES §14       |
| Membership         | company_id, user_id, role_id                                                                                    | Unikátní dvojice firma/uživatel; oprávnění jsou vlastní návrh               | FEATURES §14, Q-05 |
| Contact            | name, registration_number?, vat_number?, billing_address, maturity_days?, discount_percent?, reminders_enabled? | Firma 1:N kontakty; IČO a DIČ textově, zachovat nuly                        | E-005              |
| ContactAddress     | contact_id, kind, address, is_default                                                                           | Kontakt 1:N dodací adresy                                                   | E-005              |
| ContactBankAccount | contact_id, account_number?, bank_code?, iban?, bic?                                                            | Validovat podporované kombinace; minimálně jeden identifikátor účtu         | E-005              |
| CompanyBankAccount | currency, account_number?, bank_code?, iban?, bic?, is_default                                                  | Výchozí účet podle upřesněných pravidel; nepřebírat tarifní limit           | FEATURES §10       |
| NumberSeries       | document_kind, format, period_key, next_value                                                                   | Atomické přidělení; rozsah unikátnosti období/formátu upřesnit Q-02         | E-001              |

Adresu tvoří `street`, `city`, `postal_code`, `country_code`; povinnost podle země a scénáře upřesnit. Přihlašovací email není automaticky kontaktním emailem firmy.

## Doklady

Navržená společná hlavička `Document` a typované rozšíření pro fakturu/nabídku/zálohu; nepředpokládat, že všechny typy mají shodné stavy a povinná data.

| Entita                  | Klíčová pole                                                                                                                                                                                                                    | Pravidla                                                                                                                                       |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Document                | kind, lifecycle, partner_id?, number_series_id?, number?, issued_on?, due_on?, tax_on?, currency, exchange_rate?, exchange_rate_amount?, description?, internal_note?, order_number?, variable_symbol?, payment_method?, locale | Návrh může být neúplný; povinnosti při vystavení určuje specifikace typu. Číslo a VS jsou text, ne integer. Kurz i jeho množství patří k sobě. |
| DocumentPartySnapshot   | document_id, role, name, registration_number?, vat_number?, address                                                                                                                                                             | Dodavatel/odběratel/dodací místo podle typu. Historický obsah nezávislý na živém kontaktu.                                                     |
| DocumentPaymentSnapshot | document_id, account_number?, bank_code?, iban?, bic?, constant_symbol?                                                                                                                                                         | Historické údaje pro úhradu; změna výchozího účtu je nepřepíše.                                                                                |
| DocumentLine            | document_id, position, kind, name, quantity, unit?, unit_price, price_mode, discount_percent, vat_category, vat_percent?, vat_code?, catalog_item_id?, source_document_id?                                                      | 1:N; unikátní pořadí v dokladu. `kind` rozlišuje normální a odpočtové položky; přesný seznam dle typu.                                         |
| DocumentTotals          | document_id, calculation_version, net, vat, gross, rounding, base_currency_totals?                                                                                                                                              | Uložení výsledku pro audit; soulad s položkami a rekapitulací ověřuje kalkulátor.                                                              |
| DocumentTaxSummary      | document_id, tax_key, net, vat, gross                                                                                                                                                                                           | Rekapitulace podle relevantní sazby a režimu; neagregovat pouze podle procenta.                                                                |
| DocumentRelation        | source_id, target_id, relation_type                                                                                                                                                                                             | Oprava, převod nabídky, vyúčtování; typovaná pravidla zabrání neplatným vazbám a cyklům.                                                       |
| AdvanceSettlement       | advance_id, final_invoice_id, amount, currency, tax_document_id?                                                                                                                                                                | Výše čerpání oddělená od pouhé vazby; souběžné čerpání nesmí překročit dostupnou částku. Pravidla čekají na Q-06.                              |

Podklad hlavičky/položek: E-001, E-002, E-003. Snapshoty, normalizace a verze kalkulátoru jsou náš návrh. Vazby záloh částečně podporuje E-007; nejsou zatím kompletně specifikované.

### Mapování SDK → interní návrh

| SDK                                         | Interní pole                          | Poznámka                                                          |
| ------------------------------------------- | ------------------------------------- | ----------------------------------------------------------------- |
| PartnerId                                   | partner_id přes ExternalIdentifier    | Externí integer není interní ID                                   |
| CurrencyId                                  | currency přes číselník                | Neodhadovat měnu z čísla                                          |
| DateOfIssue / DateOfMaturity / DateOfTaxing | issued_on / due_on / tax_on           | SDK má DateTime; převod na kalendářní datum ověřit                |
| ExchangeRate / ExchangeRateAmount           | exchange_rate / exchange_rate_amount  | Směr a zaokrouhlení Q-01, R-03                                    |
| Amount / UnitPrice                          | quantity / unit_price                 | `Amount` položky znamená množství                                 |
| PriceType                                   | price_mode                            | Explicitně WithVat / WithoutVat / OnlyBase; nepřevzít jen boolean |
| VatRateType / VatRate / VatCodeId           | vat_category / vat_percent / vat_code | Typ, procento a členění jsou různé údaje                          |
| NumericSequenceId / DocumentSerialNumber    | number_series_id / serial metadata    | Formátované číslo nelze získat pouhým přetypováním serialu        |
| ProformaInvoices / InvoiceProformaId        | DocumentRelation / source_document_id | Finanční čerpání specifikovat samostatně                          |

## Platby a oddělení stavů

`FinancialTransaction`: `account_id`, `direction`, `amount`, `currency`, `booked_on`, `external_id?`, `source`, `raw_reference?`. Zahrne banku/pokladnu pomocí typované evidence účtu; zdrojový importní identifikátor slouží k deduplikaci.

`PaymentAllocation`: `transaction_id`, `document_id`, `transaction_amount`, `document_amount`, `exchange_rate?`, `effective_on`, `reversal_of?`. Platba může pokrýt více dokladů, doklad více plateb. Transakce a alokace zachovávají firmu; součty alokací se kontrolují v transakci. Vratky a záporné doklady potřebují vlastní specifikaci před implementací.

- Životní cyklus: pracovní návrh `draft → issued`; opravy, koš a další přechody zatím Q-02. Není to doložený enum iDokladu.
- Úhrada: `unpaid`, `partially_paid`, `paid`, `overpaid`; externí hodnoty E-004 mapovat explicitně. Pro kladnou pohledávku návrh odvozuje stav z alokovaných částek a celkové částky. Nulové/záporné doklady zůstávají Q-04.
- Po splatnosti: samostatný odvozený údaj podle data, firemní zóny a neuhrazené částky.
- Odeslání a export: samostatné záznamy, nikoli stavy úhrady či vystavení.

## Navazující moduly — zatím pouze hranice modelu

| Modul          | Navržené entity                                      | Vazba / co ještě specifikovat                                                     |
| -------------- | ---------------------------------------------------- | --------------------------------------------------------------------------------- |
| Ceník a sklad  | CatalogItem, Warehouse, StockMovement                | Pohyb odkazuje na řádek dokladu; ocenění a opravy otevřené                        |
| Nákup a inbox  | InboxItem, ExtractionAttempt, Attachment             | Originál souboru zachovat; revize vytěžení a vazba na přijatý doklad              |
| Automatizace   | InvoiceTemplate, RecurrenceSchedule, JobRun          | Výskyt plánu má unikátní klíč; časová zóna a retry politika                       |
| Komunikace     | Message, DeliveryAttempt, SharedDocumentLink         | Revize dokladu, příjemci, odvolání odkazu, historie pokusů                        |
| Exporty a daně | ExportBatch, ExportEntry, TaxReport                  | Export obsahuje revizi dokladu; formulář má období a verzi schématu               |
| Integrace      | ExternalIdentifier, ImportRun, IntegrationConnection | Unikátní firma/zdroj/externí agenda/typ/externí ID; tajemství odděleně            |
| Infrastruktura | AuditEvent, IdempotencyRecord, OutboxEvent           | Audit s aktérem a revizí; idempotence v rozsahu firmy/operace/klíče a hash vstupu |

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

### ADVANCE-01 (migrace 018)

Sales_document rozšiřuje kind o advance; samostatně uložené součty/PDF a
advance_paid numeric(20,2). Advance_sequence má klíč (company_id,year).
Advance_payment ukládá původní příkaz, částku, datum, způsob, aktéra a případné
storno; složený FK do sales_document stejného profilu, unikátní request_id v profilu.
Zápis platby, součet a audit jednou transakcí; PDF se úhradou nemění. Žádné
záznamy v invoice_payment ani běžném přehledu. Daňové a vyúčtovací vazby doplňují níže popsané migrace 019/020.

### TAX-RECEIPT-01 (migrace 019)

Tax_receipt patří přes složený FK konkrétní advance_payment ve stejném profilu
(a tím i záloze). Unique(company_id,payment_id) brání dvojímu zdanění úhrady;
request_id s uloženým příkazem a aktérem zajišťuje idempotenci. JSON snapshot
obsahuje strany, data, původní číslo zálohy a skutečný daňový rozpis; PDF bytea
se už nepřepočítává. Tax_receipt_sequence je samostatná roční řada. Platba
s navázaným dokladem nejde stornovat; vystavení i storno zamykají stejnou zálohu.
Číslování, snapshot, PDF a sales_audit jsou atomické. Migrační dopad je aditivní,
ale před nasazením je nutné zastavit starý server neznající novou blokaci storna.

### ADVANCE-SETTLEMENT-01 (migrace 020)

Invoice_draft.advances a invoice_draft_audit.advances uchovávají verzovaný snapshot
záloh, zdrojových plateb, přijatých částek a původních Totals všech DDPP. Odpočty
se neodvozují z aktuálního kontaktu ani nové sazby. Advance_settlement_link má
PK(company_id,advance_id), složené profilové FK a index cílové faktury. Záloha
patří nejvýše jednomu návrhu/vystavenému dokladu; sales_document.invoice_id
ukazuje tutéž vazbu. Advance_settlement_command ukládá idempotentní příkaz,
aktéra a výsledek. Změny vazeb a obou auditů jsou atomické. View
invoice_with_settlement přidává advances při zachování starších sloupců.
Před migrací zastavit starý server; nový proces vytvoří i nové prepared statements.

## Dobropisy — P13-04

Migrace 021 přidává `credit_note` se snapshotem zdrojové faktury, zadáním korekce, zápornými součty a neměnným PDF, dále `credit_note_sequence`, `credit_note_command`, `credit_note_audit` a append-only `credit_note_event`. Události zápočtu/vratky mají kladnou částku; storno odkazuje na událost a historii nemaže. Všechny vazby ověřují company_id. Vystavení zamyká zdrojovou fakturu a kontroluje již dobropisované řádky. Pohled invoice_settlement zahrnuje aktivní zápočty, cash_paid zůstává samostatně pro příjmy. DDPP ani rezervace záloh se dobropisem neuvolňují. [Specifikace](specs/credit-notes.md).

## Pravidelné faktury — P14-01

Migrace 022: `recurring_rule` obsahuje snapshot vzoru, kalendář, pásmo, aktivujícího uživatele, revizi, stav a počet výskytů. `recurring_occurrence` má company-scoped vazbu na pravidlo a fakturu, unikátní index výskytu, neměnný obsah použitého vzoru a stav zařazení e-mailu. `recurring_command` a `recurring_audit` jsou append-only. Posun kurzoru a vystavení sdílí jednu transakci s existujícími příkazy faktur. E-mailová vazba vzniká idempotentně po commitu; výskyt drží stabilní UUID a snapshot požadavku. [Pravidla](specs/recurring-invoices.md).

## Komunikace faktur — P14-02

Migrace 023: profilová `communication_settings`, revize a aktér; `communication_pause`
s vazbou na fakturu; `communication_job` s unikátním slotem, stabilním UUID,
neměnným payloadem a vazbou na outbox; `communication_audit` změn. Outbox nese
`purpose`, revizi úhrad a volitelnou revizi automatizace. Komunikace nemění
částky, čísla, PDF ani účetní stav. Podrobnosti: [specifikace](specs/invoice-communication.md).

## Veřejné faktury — P14-03

Migrace 024: `invoice_share` váže profil, vystavenou fakturu, aktéra a UUID
příkazu; obsahuje hash tokenu, důvěrnou URL, expiraci a audit odvolání.
`invoice_view` ukládá čas/revizi a deduplikované UUID návštěvy bez identifikace
čtenáře. Složené FK brání propojení cizích profilů. `invoice_delivery.share_id`
rozlišuje odkaz od PDF přílohy, uložené PDF historie zůstává zachované.
Veřejný náhled používá původní cache PDF a současný odvozený stav úhrady.
[Specifikace](specs/invoice-sharing.md).

## Přijaté faktury — P15-01

Migrace 025 přidává `received_invoice` (profil, volitelný kontakt, PF číslo,
revize, stav, JSONB snapshot, data, total/paid NUMERIC(14,2), platební historie),
`received_series` po profilu/roce a transakční `received_command`/`received_audit`.
UUID příkazu drží aktéra, vstup a výsledek; audit uchovává snapshot revize.
Platby jsou oddělené od příjmů, storno doplňuje původní událost a další audit.
`document_attachment.received_id` má složený firemní FK a výlučnost jednoho
typu dokladu. [Specifikace](specs/received-invoices.md).

## Účtenky a přehled výdajů — P15-02

Migrace 026 rozšiřuje received_invoice o kind invoice/receipt; received_series
má PK company/year/kind. Stávající řady i JSON snapshoty zůstávají zachované,
serde default chápe staré doklady jako invoice. UUID vstupy starých příkazů
zachovávají původní serializaci, aby replay fungoval i po upgradu.
Účtenka má cash/card metodu v Data, plnou aktivní úhradu a nulový závazek.
Oprava/zrušení stornuje původní platbu; nevzniká příjmová invoice_payment.
Přehled je autorizovaná projekce dokladů a plateb, nikoli další zdroj peněz.
[Pravidla a období](specs/purchase-receipts.md).

## Banka — P16-01

Migrace 027: `bank_account` (profil, název, unikátní IBAN, CZK, revize, archiv),
`bank_movement` (profil/účet, B číslo, zdroj, stabilní external_id, podepsaná
NUMERIC(14,2), kalendářní datum a kanonická JSON data), `bank_series` po profilu/roce.
`bank_match` váže pohyb na vydanou/přijatou fakturu a ID její úhrady. Částečné
unikátní indexy chrání jednu aktivní vazbu pohybu i úhrady; složené FK chrání
profilovou izolaci. `created_payment` určuje, zda odpojení úhradu stornuje.
Historie uchovává autora/čas a důvod odpojení. `bank_command` ukládá UUID, autora,
vstup a výsledek pro atomické opakování a audit. Peníze se nepřidávají do druhé
nezávislé evidence úhrad. [Specifikace](specs/banking.md).

P16-02: migrace 028 rozšiřuje bank_movement.source o camt053, fio_json,
moneta_xml a rb_abo. Žádná změna uložených částek ani historických ID.
Bank_command audituje potvrzený import; návrhy shod jsou read-only.

P16-03 nemění schéma. `bank_command` uchovává batch_match se seznamem ID,
fingerprintem náhledu a celým výsledkem. Společná transakce vytváří bank_match
vazby a případné úhrady; při selhání se vrací celý zápis. Náhled se neukládá.

## Pokladna — P16-04

Migrace 029 přidává cash_account (profil, název, CZK, počáteční datum/stav
NUMERIC(14,2), archiv, revize), cash_series (profil/rok/směr), cash_movement
(účet, PP/VP číslo, JSON data, podepsaná NUMERIC(14,2), datum, storno/důvod),
cash_match a cash_command. Složené firemní FK a částečné unikátní indexy chrání
vazby a jednu aktivní vazbu pohybu/platby. Platby zůstávají v původní evidenci
invoice_payment / received_invoice, ledger je nezdvojuje. Přesné součty salda
počítá PostgreSQL; frontend pouze formátuje řetězce. Příkaz ukládá aktéra,
čas, vstup a výsledek jako audit a pro opakování. Po prvním dokladu nelze
měnit počáteční stav/datum. Starší data se automaticky nepřevádějí.
Před upgradem zastavit staré API; nová verze kontroluje cash_match při
opravách plateb a účtenek. [Specifikace](specs/cash-register.md).

## Automatické pokladní doklady a PDF — P16-05

Migrace 030 přidává `cash_settings` (profil, nullable hlavní pokladna, revize),
`cash_movement.automatic` a `issuer` snapshot a `cash_pdf` cache podle profilu,
dokladu, revize a jazyka. Složené FK zachovávají izolaci profilů. Staré
doklady získají aktuálního vystavitele při migraci; dřívější verzi nelze obnovit.
Automatický doklad nevytváří další úhradu: vzniká ve stejné transakci jako
zdrojová platba a jeho storno následuje zdroj. Nastavení i automatické změny
jsou auditované v `cash_command`. PDF uchovává lokálně renderované byty.

## Samostatná položka DPH v pokladně — P16-06

Migrace 031 přidává `cash_movement.tax` JSONB/null. Snapshot obsahuje
input_amount, tax(price_mode, vat_percent, tax_on), calculation_version a
amounts(net, vat, gross). Amounts jsou nezáporné; cash_movement.amount je
podepsané gross. Staré a automatické doklady mají null; DB zakazuje kombinaci
automatic=true a tax. Daňový pohyb nelze párovat s úhradou jiného dokladu.
Neměnnost a revizní PDF cache zůstávají, storno vyřadí celý pohyb ze salda.

## Bankovní počátek — P16-10

Migrace 032: bank_account.opening_on date/null a opening_balance NUMERIC(14,2)/null,
CHECK oba vyplněné nebo oba null. Existující revize účtu chrání změnu; příkaz
v bank_command nese důvod a aktéra. Žádná platba ani umělý bankovní pohyb
nevzniká. Historická data před počátkem zůstávají, agregace je explicitně vyloučí.

### P16-15a — bankovní přístupy a běhy

Migrace 033: `bank_connection` má profilovou FK na účet, revizi, provider ID
a AES-GCM ciphertext tokenu s AAD profil/účet. `bank_connection_run` uchovává
aktéra, request ID, revizi, operaci, časy, bezpečný kód chyby a počty pohybů.
Nikdy neobsahuje token, URL s tokenem ani syrovou bankovní odpověď.
Změny konfigurace jsou optimistické, běhy mají 60s rezervaci; síťové I/O je
mimo DB transakci. Podrobnosti v [bankovním napojení](specs/bank-connections.md).

P16-15b / migrace 034 rozšiřuje bank_connection o opt-in plán, interval,
aktéra aktivace, next_run_at a bezpečnou chybu plánovače. Claim termínu je
atomický FOR UPDATE SKIP LOCKED; konfigurace zachovává optimistickou revizi.
Síťové I/O neprobíhá pod SQL zámkem, import znovu ověřuje práva.

P16-14 / migrace 035: bank_connection má nezávislé matching_enabled,
matching_actor_id, matching_next_at, matching_cursor a matching_error.
Výchozí vypnutí zachovává dosavadní bankovní přístupy i plán synchronizace.
Bank_matching_run zaznamenává profil, účet, aktéra, revizi konfigurace, počty
a rozhodnutí pro jednotlivé pohyby. Worker sdílí profilový advisory lock
a platební příkazy ruční dávky; změna nastavení používá stávající revizi.

P16-11 / migrace 036: bank_match.amount NUMERIC(14,2) je kladná neměnná částka
vazby. Historické vazby přebírají celou absolutní částku pohybu. Aktivních
vazeb na pohyb může být více; unikátnost úhrady se zachovává. Pod profilovým
zámkem součet aktivních částek nesmí překročit abs(bank_movement.amount).
DTO Movement vrací allocated a remaining, Candidate.available rozlišuje
maximum nové platby od celé existující úhrady. Uložené idempotentní odpovědi
se obohatí z historických snapshotů. Žádné přepisování zdrojových úhrad.

### P16-12 — vazby pokladny/banky na zálohy a dobropisy

Migrace 037 přidává `advance_id` a `credit_id` do `bank_match` a `cash_match`.
Právě jeden ze čtyř cílových FK je vyplněn a každý FK zahrnuje firmu.
`payment_id` odkazuje na vlastní platební historii cílového typu; tyto úhrady
se nekopírují do `invoice_payment`. Aktivní unikátní indexy chrání jednotlivé
platby, společný firemní zámek a validace hlídají i vazby napříč evidencemi.
Částky a historie rozdělení banky z migrace 036 zůstávají zachované.

### P16-09a — konfigurace terminálů

Migrace 038 přidává sales_terminal: profil, kód, název, revize a archivace.
Unikátní (company_id, code), změny v existujícím cash_command auditu.
Není to bankovní terminál ani evidence úhrady; snapshot na prodejce navazuje.

## Prodejky a inbox — migrace 039–041

`retail_document` ukládá položky, přesné částky metod a kopii terminálu.
`retail_posting` sdružuje uložené rozpisy a má reverzní vazby na zdroje
a pokladní pohyby; prodejky nevytvářejí invoice_payment. Samostatné cash
operace nesmějí měnit doklad vzniklý z dávky. `retail_audit` uchovává revize.

`inbox_document` má neměnný originál a profilově unikátní hash. Převod
používá společnou transakční tvorbu přijatého dokladu a zkopíruje originál
do příloh ve stejné transakci. Archiv neodstraňuje přílohu ani doklad.
Každý příkaz má firemní autorizaci a UUID replay; audit zůstává v DB.

### Migrace 042–043: sklad a vzhled

`stock_card` ukládá revizi a přesné množství na ceníkovou položku/profil.
`stock_movement` je neměnná historie se snapshotem položky a vazbou reverzu;
`stock_command` řeší opakování. `user_preference` ukládá vzhled light/dark/system
a revizi pod uživatelem. Přehled P19-01a schéma nemění, čte společný snapshot
zdrojových dokladů.

### Migrace 044: skladové vazby dokladů

Volitelné `catalog_id` na fakturačním řádku zachovává explicitní identitu
ceníku; staré JSONy ji nemají. Nákupní `Line.stock` ukládá potvrzené
množství a jednotku. `Correction.return_quantity` odděluje vrácené zboží
od finanční opravy. `stock_movement` má firemní FK na invoice/received/credit/retail
a `source_line`. Změny zapisují přesné rozdíly, předchozí pohyby nemažou.
Vazby nejsou doplněny zpětně z názvů. Zápis dokladu a salda tvoří jednu transakci.

### Migrace 045: explicitní skladové ceny

`stock_valuation` je append-only historie ceny za jednotku CZK, data účinnosti,
důvodu a autora s revizí a UUID replay. Složený FK chrání profil/kartu.
Cena je nullable (neznámá), NUMERIC(18,4), bez automatického doplnění pro
starší zásoby. Výběr ceny k datu a součet množství se čtou ve společném
snapshotu. Zpětné opravy ceny mohou změnit historický přehled.

### Migrace 046: pořizovací cena příjmu

`stock_movement.acquisition_unit_cost` je nullable NUMERIC(18,4), pouze pro
příjmy nákupu, 0..1 miliarda CZK/jednotku. `acquisition_correction` označuje
opravu dřívějšího příjmu včetně nulového množstevního pohybu při změně ceny.
Obě hodnoty jsou snapshot, původní řádky se nepřepisují; staré ceny jsou NULL.
Oprava i pohyb vznikají ve stejné transakci jako nákup. REST/MCP report může
volit ruční ocenění nebo konzervativní průběžný průměr; opravy bez vyřešeného
rozúčtování zobrazí neznámou hodnotu. Před upgradem zastavit starý server.

### Migrace 047: rozvržení nákladů a přepočet oprav

`stock_movement.acquisition_quantity` ukládá celkové množství nákupního řádku
po revizi. Historická hodnota se doplní součtem jeho pohybů v pořadí ID.
`acquisition_extra_cost` NUMERIC(18,2), 0..1 miliarda, ukládá přidělenou část
vedlejších nákladů; pro staré pohyby 0. Data nákupu mají volitelný
`stock_incidental_cost` (vynechané/NULL = 0). Staré replay JSON zůstává platné.
Rozvržení i nové snapshoty vznikají s nákupem v jedné transakci.

Report přepočítává nákupní cykly s cenou poslední revize do požadovaného data;
po úplném odstranění a znovupřidání řádku začíná nový cyklus. Vratky navazují
na původní výdej přes již existující profilové vazby. `issued_value`,
`correction_stock_value`, `correction_issued_value` jsou odvozené nullable
částky, nikoli účetní zápisy. Chybějící ocenění se nevyplňuje odhadem.
Před migrací zastavit staré API; sestavy před upgradem se nerovnají uzávěrce.

P18-03e / migrace 048 rozšiřuje snapshot `stock_movement.acquisition_unit_cost`
na kladné pohyby opening/manual/count. Staré neznámé ceny zůstávají NULL.
Storno používá původní pohyb a jeho hodnotu, nevytváří novou pořizovací cenu.

P18-04 / migrace 049: read-only view `item_sales_line` čte uložené řádky
faktur/prodejek/dobropisů a firemně svázaný ceník. Bez duplikované součtové
tabulky; společný report používá repeatable-read transakci. `specs/item-sales.md`.

## Import ceníku Money S3 (050)

import_identity nově podporuje kind=catalog s catalog_id a vazbou na stejný profil. CHECK vyžaduje právě správný cílový typ. import_batch podporuje source=money_s3. Normalizované řádky zachovávají zdrojový klíč; zápis katalogu a auditu je transakční. Viz specs/catalog-import.md.

## P19-01d — Finanční přehled

Bez nové migrace. Read-only dashboard nad invoice_settlement, credit_note_event, received_invoice a sales_document; aggregate numeric, jeden repeatable-read snapshot a profilová autorizace. Specifikace: specs/dashboard-finance.md. Nejde o nový účetní deník ani historický stav úhrad k zadanému dni.

## P19-01e — Daňový přehled

Bez změny schématu. Agregace uložených amounts.vat a received lines.vat,
DDPP ze snapshotu podle vystavení. Vyúčtování nevrací odpočet daně zpět,
na rozdíl od orientačního obratu, který přičítá zpět uložené základy DDPP.
Součty a zdroje používají jeden repeatable-read snapshot. Specifikace:
`specs/tax-dashboard.md`. Žádný odhad nároku na odpočet či zákonného obratu.

P19-01f (17. 10. 2026): samostatná pokladní plnění v tax_report, bez migrace.
Specifikace docs/specs/tax-dashboard.md; test:period-report a dashboard.spec.ts.
Směr dle cash_movement.amount, částky z tax.amounts, datum z JSON cesty tax.tax_on ve snapshotu;
storna, automatika a úhrady bez tax snapshotu jsou vyloučené.

P19-01g / migrace 051: JSONB snapshot exchange_rate={rate,units} v Calculation a received Data. Kurz je CZK za dané jednotky, validovaný Decimal. SQL report_czk používá NUMERIC a identické zaokrouhlení. Stará CZK data zůstávají beze změny.

P19-01h: CreditNote.totals.currency odpovídá source.data.calculation.currency; kurz je neměnný source.data.calculation.exchange_rate. CreditNoteEvent.amount je v měně dobropisu. Žádná nová tabulka ani přepis historie.

P19-01i: AdvanceSettlement ukládá currency (u historických snapshotů default CZK) a volitelný exchange_rate. Všechny zdroje i cílový doklad musejí mít shodnou měnu a snapshot kurzu; DDPP dědí kurz zálohy. Stávající JSONB a audit, bez nové migrace.

P16-13a/P19-01k / migrace 052: bank_match.foreign_settlement je nullable JSONB snapshot currency/document_amount/document_rate/book_value_czk/difference_czk. bank_match.amount stále CZK; NULL je legacy CZK vazba. Faktura je uhrazena v měně dokladu, odpojení zachová snapshot. specs/bank-foreign-matching.md.

Migrace 053: foreign_settlement lze uložit také k bank_match.created_payment=false. Bankovní vazba odkazuje na původní úhradu, její záznam se nepřepisuje. Unikátní aktivní vazby, CZK alokace a snapshot rozdílu zůstávají. Příkaz foreign_match má volitelné payment_id, None se neserializuje kvůli starým fingerprintům.

P19-01m: bank_foreign_report čte bank_match.foreign_settlement a bank_movement.booked_on v jednom REPEATABLE READ snapshotu. Odpojené vazby vyloučené, součty NUMERIC, bez změny schématu a přepočtu uložených kurzů. specs/exchange-difference-report.md.

Migrace 054: bank_account.currency dovoluje měny domény exchange; po založení
je neměnná. bank_movement.currency je STORED generated z data.currency, NOT NULL
a FK (company_id,account_id,currency) na účet. Saldo vrací explicitní currency;
NUMERIC(14,2) zůstává v nativní měně. Historické CZK se nepřepočítávají.

P16-13d: bank_match.amount je nativní částka pohybu; foreign_settlement=NULL
znamená shodnou měnu (nově i non-CZK). Bez migrace. Explicitní foreign_settlement
nadále pouze pro CZK pohyb/cizoměnovou fakturu, report jej nesmí doplňovat odhadem.

P16-13f: existující bank_match.advance_id/credit_id podporují nativní měnu
pohybu shodnou s dokladem. foreign_settlement zůstává NULL; nic neodvozuje
kurzový rozdíl. Bez schématické změny, cash_match stále pouze CZK.

P16-13g / migrace 055: foreign_settlement rozšiřuje bank_currency, bank_rate
a bank_value_czk. Staré snapshoty zůstávají beze změny (CZK/None/None); nové
ukládají bankovní hodnotu v CZK explicitně. bank_match.amount je nadále
nativní alokace, nikoli CZK přepočet. Rozdíl je rozdíl uložených CZK hodnot
se znaménkem podle směru; nejde o účetní zápis. specs/bank-foreign-matching.md.

P16-13h (18. 10. 2026): `bank_account.iban` přijímá normalizovaný zahraniční
IBAN s validací země, struktury a checksumu. Textový sloupec, firemní unikátnost
a neměnnost po pohybu se nemění; bez migrace. Měna účtu se neodvozuje ze země.
Nejde o rozšíření platebních údajů dokladů ani QR; viz specs/bank-currencies.md.

### Doplnění 18. 10. 2026 — tisk zaúčtování a rozsah párování

Migrace 056: `retail_posting.issuer` je nullable JSON snapshot profilu při
zaúčtování, historické NULL se neodhadují. `retail_posting_pdf` ukládá byty
PDF pod (company_id, posting_id, revision, language), firemní FK a CS/EN.
Migrace 057: `bank_connection.matching_treasury` je boolean default false;
rozšíření automatiky vyžaduje samostatné explicitní nastavení.

### Doplnění P16–P20 — 18. 10. 2026

- 058: kurzové snapshoty bankovních vazeb také pro zálohy a vratky dobropisů;
  nativní částka a uložené CZK ocenění zůstávají oddělené.
- 059: vratný `invoice_trash` a neměnný `invoice_trash_command`. Skrytí návrhu
  zvyšuje revizi, neodstraňuje účetní data; vydaný doklad je chráněný triggerem.
- 060–063: konfigurace/audit vytěžení a příjmu dokladů, bankovní notifikační
  brána, IMAP UIDVALIDITY/kurzor/běhy a explicitně povolený plán. Tajemství
  poskytovatelů jsou instalační konfigurací, nikoli klientským DTO.
- 070: vlastní pořadí/viditelnost/varianta dashboardových widgetů uživatele.
- 080–081: profilová správa členství/audit/obnova a štítky s profilovými vazbami.
- 082: vzhled dokumentů a neměnný snapshot při vystavení faktury. Původní PDF
  cache se nepřepisuje po změně loga, podpisu nebo patičky.
- 083: neměnné exportní snapshoty, SHA-256, obsahové zámky a audit odemčení.
  Úhrady, saldo a vytvoření původního PDF nejsou obsahovou opravou dokladu.
- 084: snapshot stejného vzhledu v nabídkách, zálohách a pokladních dokladech;
  historické záznamy bez uloženého vzhledu se zpětně neobrandují.

Nové tabulky používají profilovou autorizaci společných příkazů. Přenosný ZIP
profilu je čitelný archiv s explicitním seznamem sloupců a odstraněnými tajemstvími,
nikoli obnovitelná PostgreSQL záloha. Před upgradem zastavit staré API a zálohovat DB.

### Daňová klasifikace a údaje subjektu (071–072)

`tax_evidence_classification` uchovává explicitní rozhodnutí účetního a fingerprint zdrojového plnění; audit a command tabulka zachovávají historii a idempotenci. Změněný zdroj klasifikaci zneplatní, provozní revize bez změny obsahu ji nemění.

`taxpayer_filing_profile` ukládá firemní identifikaci pro budoucí EPO výstupy, revizi a aktéra; audit je neměnný. Kódy finančních úřadů a jejich pracovišť ověřuje verzovaný oficiální číselník. Profil není změnou plátcovství na dokladech ani hotovým daňovým podáním.

### Oznámení a anonymizace (090, 100)

`inbox_notification_event` obsahuje pouze bezpečné metadata příjmu. Vlastní `inbox_notification_settings` drží opt-in a oddělené monotónní kurzory podle kategorie; čtení respektuje aktuální členství. GET bez uloženého nastavení vrací virtuální výchozí stav.

`contact.anonymized_at` označuje nevratný stav samostatného kontaktu. Příkaz a audit jsou atomické, čistí i adresářovou historii; původní finanční snapshoty a PDF nejsou měněny. DB trigger blokuje navrácení osobních údajů do anonymizovaného záznamu.

### Inbox předání a koš prodejních podkladů (101/103)

`inbox_delivery` uchovává samostatný snapshot původního souboru, potvrzené zprávy,
aktéra a stavů SMTP; žádná operace jej nezaměňuje za vytvoření účetního dokladu.
`sales_trash` a `sales_trash_command` ukládají vratné odstranění nepoužitých nabídek
nebo šablon. Zdrojová data a číslo se nemění, následné operace respektují stav koše;
navázané, sdílené, odeslané a exportem uzamčené zdroje jsou chráněné.
