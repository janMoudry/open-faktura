# P06 — uložené návrhy faktur

Implementováno 11. 10. 2026. Vlastní datový a autorizační kontrakt Open Faktury;
UX vychází z [pozorování FakturyOnline](../design/invoice-editor-ux.md), E-014.
Výpočty jsou `of-czk-v1` z P05, numerická shoda s iDokladem zůstává neověřená.

## Chování

Seznam pod aktivním fakturačním profilem nabízí hledání podle odběratele,
objednávky a ID, filtr odběratele, řazení a serverové stránkování. Editor umožňuje
uložit neúplný návrh, znovu jej otevřít, upravit a zkopírovat do nového návrhu.
V seznamu je názvem popis první položky (po oříznutí okolních mezer), při
prázdném popisu či chybějící položce „Bez názvu“ / „Untitled“. Vedle je badge
„Rozpracováno“ / „Draft“; interní ID se v seznamu nezobrazuje. Editor uloženého návrhu má
nadpis „Rozpracovaná faktura“ / „Invoice draft“, bez interního ID.

Dodavatel se při založení kopíruje z profilu. Výběr odběratele z adresáře kopíruje
údaje a předvolí splatnost a způsob platby. Změny v adresáři ani v profilu již
uložený snapshot nepřepisují. Ručního odběratele lze výslovně uložit do adresáře;
ARES doplňuje prázdná pole stejným postupem jako P04. Bankovní účet na faktuře
patří dodavateli, z kontaktu se nekopíruje.

Data, platba, položky, procentní slevy, objednávka, poznámka a jazyk dokladu jsou
uložené na serveru. CS/EN rozhraní používá vlastní volbu jazyka; přepnutí UI
nepřepisuje jazyk rozpracovaného dokladu. Přepočet běží na backendu s debounce,
rušením předchozího požadavku a kontrolou aktuálních vstupů. Neúplné číselné
vstupy lze uložit, součet je v takovém případě `null`, nikoli nula. Neplatné
neprázdné hodnoty (NaN, záporné ceny, neznámá sazba atd.) server odmítá.

Odchod s neuloženými změnami používá společnou navigační ochranu. Konflikt
revize zachová uživatelovy vstupy; načtení serverové verze vyžaduje potvrzení.
Automatické sloučení změn ani tiché přepsání nejsou součástí P06.

## INV-DUE-DAYS-01 — zadání splatnosti ve dnech

Vlastní UX pravidlo Open Faktury podle požadavku uživatele, 17. 10. 2026;
nejde o tvrzení shody s iDokladem. Editor nabízí současně datum splatnosti
a výběr předvoleb 7, 14, 21, 30 a 60 kalendářních dní od vystavení.
Volba předvolby dopočítá `due_on`. Ruční změna data přepne výběr na
„Vlastní datum“, i když datum odpovídá předvolbě. Volba „Vlastní datum“
ponechá aktuální datum a umožní jeho ruční úpravu. Změna data
vystavení při vyplněných obou datech zachová jejich odstup. Výběr kontaktu
nebo šablony dál použije jejich stávající předvolbu.

Splatnost v den vystavení i jakýkoli jiný odstup se zadá vlastním datem.
Datum lze vymazat a neúplný návrh nadále uložit. Bez data vystavení je
výběr předvoleb nedostupný, datum lze stále zadat přímo.
Předvolba je odvozená v UI; API, databáze i PDF používají dosavadní datum,
serverová pravidla vystavení se nemění. Po načtení návrhu se předvolba znovu
odvodí z uložených dat; při jiném odstupu je vybrané vlastní datum.
Volba režimu se zvlášť neukládá. Počítají se kalendářní dny bez vlivu letního času.

Akceptační regrese v `tests/e2e/redesign.spec.ts` se syntetickým API:
CS/EN, nabídka pěti předvoleb a vlastní volby, změny obou dat a přepnutí na vlastní
datum, přechod měsíce/roku, přestupný rok a přechod letního času,
zápis existujícího `due_on` a obnovení stránky.
Ověřeno v Chromiu a WebKitu i mobilním viewportu; tato UI regrese nenahrazuje
databázové testy návrhů P06.

## Persistence a oprávnění

Migrace `202610110007_invoice_drafts.sql` přidává `invoice_draft` a
`invoice_draft_audit`. Ukládá snapshot JSONB, vypočtené součty, revizi a audit
v jedné transakci. Vazba na kontakt je složená přes `company_id`; nepovoluje
odkaz na odběratele jiného profilu. Archivovaný kontakt zůstává použitelný
v existujícím návrhu. Návrh přežije změnu kontaktu i restart procesu.

Čtení: owner/editor/viewer daného aktivního profilu. Zápis: owner/editor.
Správce instalace nemá automatický přístup. Každý dotaz je omezen profilem;
soft-deleted profil a cizí návrh jsou nedostupné. Vytvoření používá UUID
`request_id`; opakování shodného příkazu stejným aktérem nevytváří duplicitu,
změněný obsah pod stejným klíčem vrací 409. Aktualizace kontroluje revizi.

REST: GET/POST `/api/companies/{company}/invoices`, GET/PUT
`/api/companies/{company}/invoices/{id}`. MCP: `invoices_list`, `invoice_get`,
`invoice_create_draft`, `invoice_update_draft`. Oba adaptéry používají společné
aplikační příkazy. Kontrakt: `contracts/invoice.openapi.json`, generování
`pnpm generate:api`; JSON schema nástrojů je `contracts/draft.tools.json`.

## Ověření a hranice

`DATABASE_URL=... pnpm test:invoices` vytváří izolované testovací DB:
idempotence, souběžné revize, role a profily, neměnnost snapshotu při změně
kontaktu, stránkování a filtry, neúplné hodnoty, REST/MCP shoda a CSRF.
`tests/e2e/invoices.spec.ts` ověřuje CS/EN na desktopu a mobilu, návrat po
refreshi, přepočet, konflikt, kopii, výběr profilu a režim pouze pro čtení.
Používá vlastní syntetické DB, nikdy uživatelské doklady.

Vystavení, konečné číslování a PDF doplňuje implementované [P07](invoice-issue.md).
Odeslání a úhrady patří do P08/P10.
P06 nepřidává mazání návrhů ani další měny. Vlastní návrh účetních pravidel
není potvrzením referenční ani právní shody.

## RE-01 — seznam a rozšířené filtry (11. 10. 2026)

Seznam vychází z E-021, screenshotů iDoklad 04/05. Čtyři hlavní pohledy
Všechny / Uhrazené / K úhradě / Po splatnosti doplňuje vlastní pohled
Rozpracované. Další stavy jsou v rozšířeném filtru. Hlavičky řadí číslo,
odběratele, vystavení, splatnost a cenu v obou směrech; UI začíná číslem
sestupně. API zachovává původní default updated_desc kvůli kompatibilitě.

Filtry REST i MCP: date_field (issued_on/due_on), date_from/date_to jako
kalendářní ISO data a amount_min/amount_max jako desetinné řetězce CZK.
Hranice jsou včetně; všechny filtry se kombinují AND, před počítáním počtu
výsledků a stránkováním. Cena znamená uložené totals.payable; neúplné návrhy
bez součtu do cenového filtru nespadají. Neplatný či obrácený interval je 422.
Částky se na serveru validují Decimal, v SQL porovnávají numeric. UI nabízí
10/25/50/100 položek; backend nadále povoluje 1–100. URL zachovává filtry,
řazení a stránku. Změna filtru resetuje stránku na první.

Datum úhrady, štítky, opakované vzory, export a hromadné operace nejsou
implementované, proto je formulář nenabízí jako funkční. Toto je vědomá
odchylka od reference, nikoli tvrzení o úplné shodě iDokladu.

Editor RE-01 (11. 10. 2026) přebírá strukturu přímo pozorovaných screenshotů
E-021 01/02. Nový doklad a uložený návrh mají totožný formulář: odběratel,
objednávka/popisek, datumy a platební údaje, další nastavení dodavatele a jazyka,
položky a součet. InvoiceSummary sdílí editor s issued detailem. Starší
popis interního ID v nadpisu výše je nahrazen „Rozpracovaná faktura“ /
„Invoice draft“; interní ID se v nadpisu již nezobrazuje. Stále jde o návrh,
nikoli rezervované či přidělené konečné číslo.
