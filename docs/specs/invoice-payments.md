# P08 — ruční úhrady a základní přehled

Implementováno 11. 10. 2026, INV-003 a FEATURES §10/12. Podklad E-018 dokládá
stavy a ruční evidenci v iDokladu. Níže uvedené datové struktury, zámky, opravy
a pravidla nulových dokladů jsou vlastní návrh Open Faktury, nikoli tvrzení
 o úplné referenční shodě. Pilotní rozsah: vystavené tuzemské CZK faktury z P07,
ručně přijaté kladné platby, hotově nebo převodem, bez bankovního importu.

## PAY-01 — příjem platby

Owner/editor zadá částku, datum, metodu a volitelnou poznámku. Viewer smí pouze
číst. Návrh nelze uhradit. Částka je desetinný řetězec, >0, maximálně
999999999999.99 a nejvýše 2 desetinná místa; bez automatického zaokrouhlení.
Datum je od data vystavení do dneška v časové zóně profilu. Budoucí plánované
platby a platby před vystavením (zálohy) nejsou tento příkaz. Přijetí hotovosti
samo negeneruje pokladní doklad. Jedna evidovaná platba náleží jedné faktuře.

Příkaz obsahuje UUID request_id a očekávanou revizi evidence úhrad. Transakce
zamkne profil/členství a vystavenou fakturu, zkontroluje stav a revizi, vloží
úhradu a audit příkazu. Částky počítá PostgreSQL NUMERIC, vstup validuje Rust
Decimal. JavaScript částky nesčítá. Revize úhrad je počet událostí příjmu/storna;
je nezávislá na neměnné revizi faktury. Souběžné doplacení stejné revize jednou
uspěje, podruhé vrátí 409 místo nechtěného přeplatku.

Stejný profil/UUID/aktér/faktura/vstup vrací uložený výsledek i po dalších změnách.
Jiný vstup se stejným klíčem vrací konflikt. UI po operaci načte aktuální stav.
Po síťové chybě opakuje nezměněný příkaz se stejným klíčem. Při konfliktu zachová
rozpracovaný formulář a vyžaduje načtení a kontrolu aktuálního stavu.

## PAY-02 — stav a zůstatek

Součet přijatých = součet nestornovaných kladných úhrad. `outstanding` je
max(celkem−přijato,0), `overpayment` je max(přijato−celkem,0).
- unpaid: nenulový doklad bez úhrad;
- partial: přijato >0 a méně než celkem;
- paid: přijato = celkem;
- overpaid: přijato > celkem.

Nulový doklad má paid a nulový zůstatek, nevyžaduje ani nepřijímá platbu.
Záporné faktury nejsou podporovány P07. `overdue` je samostatný boolean: zbývá
kladná částka a splatnost je před dneškem v zóně profilu. V den splatnosti není
faktura po splatnosti. Změna úhrady nikdy nemění issued, číslo, obsah nebo PDF.
Na UI se stav rozlišuje textem, ikonou a sémantickou barvou, ne jen barvou.

## PAY-03 — oprava a storno

Úhrada se nepřepisuje ani nemaže. Storno ukládá důvod, aktéra a čas; původní
řádek zůstává v historii. Oprava atomicky uloží novou úhradu, storno původní
a vazbu replacement_id. Selhání validace náhradní platby nestornuje původní.
Důvod je povinný, 1–500 znaků. Stejnou úhradu nelze stornovat podruhé; opakování
stejného příkazu však vrací původní výsledek. Opravy lze řetězit přes nové řádky.
Storno je oprava evidence, nikoli skutečná bankovní vratka. Skutečné refundace,
kurzové rozdíly, rozdělení jedné bankovní platby a párování patří do dalších etap.

## PAY-04 — přehled a filtry

`/companies/:companyId/overview` je aktivní položka Přehled v sidebaru.
Zobrazuje celou historii aktivního profilu v CZK: zbývá uhradit, část zůstatku
po splatnosti, přijaté úhrady, přeplatky a počty dokladů. Návrhy a stornované
úhrady se nezapočítají do částek. Přijaté částky zahrnují i přeplatky. Přehled
není výpočet zisku ani daňový report. Výpočet probíhá jedním SQL dotazem nad
aktuálním snapshotem DB; prázdný profil má nuly.

Karty vedou do filtrovaného seznamu faktur. Filtry all/draft/issued zachovány;
přibývají outstanding (zahrnuje partial), overdue, partial, paid a overpaid.
Seznam uvádí stav a zbývající částku. UI přehledu a úhrad obnovuje data při
návratu focusu a po minutě; během otevřeného formuláře úhrad automaticky
nepřepisuje revizi. Data i chyby mají CS/EN variantu.

## API, MCP a data

Kontrakt `contracts/invoice.openapi.json`, generování `pnpm generate:api`:
- GET/POST `/api/companies/{company}/invoices/{id}/payments`;
- POST `/api/companies/{company}/invoices/{id}/payments/{payment}/reverse`;
- GET `/api/companies/{company}/overview`.

MCP: invoice_payments, invoice_register_payment, invoice_reverse_payment,
company_overview volají stejné aplikační operace. HTTP zápisy ověřují relaci,
origin a X-Faktura-Request. 403 read-only; 404 cizí profil/doklad/úhrada; 409
revize/klíč/stav; 422 neplatná částka, datum, důvod. Žádné externí odeslání.

Migrace 009 přidává invoice_payment, invoice_payment_reversal a
invoice_payment_command. Všechny jsou append-only včetně DB triggerů a mají
složené firemní FK. NUMERIC(20,2) uchovává jednotlivé platby, agregace používají
NUMERIC(30,2). Pohledy invoice_settlement a invoice_with_settlement odvozují
aktuální stav bez změny vydaného snapshotu. Draft DTO má nullable settlement;
u návrhu a odpovědí ze zápisu může být null, GET/list vystaveného dokladu jej
vracejí. Úhrady mají samostatný autoritativní endpoint.

Upgrade je aditivní: existující faktury, PDF a revize zůstávají beze změny;
nenulové vystavené doklady začnou bez úhrad. Při upgrade nasadit backend a klient
společně. Žádná domnělá historická platba se nedovozuje ze způsobu úhrady cash.

## PAY-05 — ověření a obnova

`DATABASE_URL=... pnpm test:payments` používá izolované databáze:
částečná/plná/přeplatek, audit oprav, atomický rollback, opakování, souběh,
403/404, neplatné částky/datum, nuly, splatnost, filtry, REST/MCP, CSRF a upgrade
z P07. Obnovovací scénář přes pg_dump/pg_restore do nové čisté DB porovná
historii úhrad, zůstatky a přesné PDF bytes; zruší obnovené relace. Souborový
archiv se ověřuje syntetickou přílohou (produkt zatím přílohy negeneruje).
Používá stejné formáty jako provozní backup/restore skripty, neprovádí odstávku
produkčního Compose. Provozovatel stále musí ověřit vlastní infrastrukturu.

`tests/e2e/invoices.spec.ts`: celý tok od adresáře přes návrh/vystavení/PDF po
úhrady, opravu, přeplatek, storno, refresh a přehled v CS/EN desktop/mobil,
přístupnost a izolace druhého profilu. Výsledky jsou testy vlastních pravidel;
Q-04 pro ostatní měny, refundace a referenční detaily zůstává otevřené.
