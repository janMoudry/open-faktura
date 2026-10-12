# P09 / MIG-01 — migrační průvodce

Stav 11. 10. 2026: implementované společné jádro a pilotní průvodce. **P09 není
uzavřené jako ověřená migrace z obou služeb.** Chybí autentické anonymizované
exporty iDokladu a FakturyOnline a end-to-end porovnání agendy s každým zdrojem.
E-019 potvrzuje dostupnost exportů, nikoli jejich přesné schéma. Předvolby názvů
sloupců jsou vlastní návrhy, nikoli certifikované adaptéry. V UI je to uvedeno.

## Podporovaný pilot

- CSV UTF-8 (BOM, uvozovky, středník/čárka/tabulátor) a XLSX: kontakty,
  jeden list, první řádek záhlaví, mapování sloupců a ukázka tří řádků.
  Původní ID, jméno a ISO dvoupísmenná země povinné; nevyplňujeme neznámou zemi.
  Národní názvy zemí musí uživatel převést na ISO kódy. Úvodní nuly uložit jako text.
  Dodací adresy a osoby se přenášejí jen úplným JSON, ne plochým CSV.
- JSON pole záznamů podle `contracts/import.openapi.json`, příklad
  `tests/fixtures/import-records.json` a stažení `/import-format.json`.
  Podporuje kontakty, položkové historické faktury a skutečné jednotlivé úhrady.
  **Není to nativní export služby.** Faktury vyžadují samostatný převod do tohoto
  formátu; tabulkový seznam s částkou nestačí k rekonstrukci položek a snapshotů.
- Faktury v rozsahu P07/P08: běžné tuzemské CZK, nezáporné ceny, sazby 0/12/21 %,
  zákazník i dodavatel, vydání/splatnost/DUZP podle daňového režimu, vlastní
  historické částky a rekapitulace. Cizí měna se ukáže jako blokující chyba,
  její částka zůstává v reportu pod vlastní měnou. Žádný převod měn.
- Chybějící originální PDF/přílohy jsou uvedeny před potvrzením i v reportu.
  Importované faktuře negenerujeme náhradní PDF, které by se tvářilo jako originál.
  Zálohy, dobropisy, přijaté doklady, vratky, API konektory ani přílohy nejsou
  podporované; neznámá pole ve vstupním záznamu se odmítnou.
- 2 MB vstup, nejvýše 500 záznamů a 40 tabulkových sloupců. XLSX rozbalení
  max. 8 MB/100 ZIP entries, souřadnice max. 501×40 před alokací, bez maker,
  vzorců a externích odkazů. Dva souběžné blokující parsery. XLS/XLSM nepodporováno.

## MIG-P09-01 — náhled a rozhodnutí

`PrepareImport` obsahuje UUID requestu, source (`idoklad`, `fakturaonline`,
`generic`), neměnné ID zdrojové agendy a pole záznamů. Namespace je
**cílový profil + služba + zdrojová agenda + typ + původní ID**. Řádkové číslo
ani název nejsou bezpečný implicitní identifikátor. Při dalších dávkách použít
stejný namespace; jiná agenda musí mít jiný.

Náhled zapisuje pouze staging/audit dávky, ne cílové kontakty/faktury.
Validace, kolize v souboru a vůči DB, chybějící odkazy a opakované zdrojové ID
se zobrazí před potvrzením. Stejný namespace + stejný normalizovaný obsah je
`duplicate`; změněný obsah je `conflict`. Shoda názvu, IČO nebo e-mailu kontaktu
je konflikt vyžadující rozhodnutí, nikdy automatické sloučení/přepsání.

Řešení pilotu: opravit zdroj a vytvořit nový náhled, nebo explicitně vynechat
řádek. Ruční přiřazení k existujícímu cíli/sloučení zatím není implementováno.
Nevyřešená chyba blokuje další zápis dávky. Validace běží i při potvrzení,
protože cílová agenda se od náhledu může změnit. Nezpracovatelný JSON či XLSX
vrací chybu celého souboru; strukturálně platný záznam má řádkový report.

## MIG-P09-02 — bezpečné pokračování

Každé potvrzení/continue zpracuje nejvýše 25 řádků v jedné PostgreSQL transakci.
Kontakty předcházejí fakturám. Aplikace automaticky pokračuje po dávkách,
po chybě/výpadku nabídne obnovení z historie (posledních 20; API get podle ID
funguje i pro starší). Atomicky se uloží cíl, audit, zdrojová identita a výsledek.
Výpadek před commitem nevytvoří nic z dané části; po commitu opakování přeskočí
hotové řádky. Souběžné potvrzení i různé dávky téže agendy serializuje profilový
zámek, ve stejném pořadí jako vystavení a změny nastavení. Unikátní indexy jsou
poslední ochrana. POST prepare se stejným UUID a jiným vstupem/aktérem je 409.

Originální číslo se uloží přímo, **nepoužívá se příkaz issue ani alokace čísla**.
Kolize se nepřečísluje. Číselná řada se nemění; UI upozorňuje na kontrolu nastavení
před další fakturací. Účetní datum zůstává ve snapshotu; technické `issued_at`
je okamžik zápisu do tohoto systému, nikoli tvrzení o původním čase vystavení.
`calculation_version=import-v1` označuje původ částek a zakazuje běžný renderer PDF.

Součty se nepřepočítávají: kontroluje se formát/přesnost, net + VAT = gross,
součet položek a rekapitulace, gross + rounding = payable. Validátor P07 slouží
pro kontrolu podporovaného tvaru a mezí; jeho vypočtený výsledek se **zahodí**.
Dodané částky zůstanou beze změny (vyžadovány dvě desetinné pozice). Zdrojová
úhrada potřebuje částku, datum, způsob a poznámku; stav „uhrazeno“ bez historie
se nikdy nepřevádí na vymyšlenou platbu. Platby jsou atomické s fakturou,
navazují na P08, lze je později opravovat/stornovat běžným auditem.

## MIG-P09-03 — oprávnění a report

Prepare/table/apply: owner/editor aktivního profilu; viewer čte historii/report.
Každé API/MCP volání znovu ověřuje relaci i členství. Instalační admin členství
neobchází. Web POST má ochranu Origin a X-Faktura-Request. Žádné odesílání.
Report JSON obsahuje identitu dávky/zdroje/agendy, všechny původní klíče,
stavy/důvody, odkazy na cíle a součty podle měn (zdroj, nově importované,
vynechané/duplicitní, importované úhrady), včetně seznamu omezení. Počty se
určí z řádků; neplatná částka není přičtena do součtu. JSON nevyhodnocuje Excel
vzorce a není exportován jako potenciálně spustitelné CSV.

UI je CS/EN, používá aktivní profil, sdílené Select/Field/Checkbox/Badge, routing
přes trvalý layout. Neodeslané mapování je lokální; připravená dávka přežije reload.
Před potvrzením text upozorní na zálohu DB a souborů. Automatický rollback
záměrně chybí: po úpravách/vazbách nelze bezpečně plošně smazat cíle.

## Kontrakty a migrace

- `GET/POST /api/companies/{company}/imports`: historie / prepare.
- `GET/POST /api/companies/{company}/imports/{id}`: report / apply `{skip:[index]}`.
- `POST .../imports/table`: omezené base64 CSV/XLSX → pole hlaviček/řádků.
- MCP `imports_list`, `import_get`, `import_prepare`, `import_apply`: stejné příkazy.
  MCP pracuje s kanonickými záznamy; souborová konverze je web/REST pomůcka.
- Autorita `contracts/import.openapi.json`; `pnpm generate:api` regeneruje
  klienta i MCP schemas. Serverové validační kódy se lokalizují na klientu.
- Migrace 010 je aditivní: `import_batch`, `import_row`, `import_identity`.
  Kompozitní FK omezují vazby na profil; identita zdroje je append-only.
  Starší backend nesmí být použit pro import, původní doklady dál obslouží.

## Ověření a zbývající brána

`pnpm test:imports` (izolovaný PostgreSQL s CREATEDB), neignorované parser testy
`cargo test -p faktura-server --test import`, UI `tests/e2e/import.spec.ts`.
Syntetické fixtures nejsou exporty iDokladu/FakturyOnline. Obsahují záměrný
historický rozdíl 59,99 proti současnému výpočtu 60,00, aby se prokázalo zachování.

Pro uzavření P09 získat anonymizované exporty kontaktů a kompletních faktur z obou
služeb; doložit přesná záhlaví/typy, položky, původní ID agendy a chybějící pole.
Podle toho doplnit skutečné adaptéry (případně dokumentovaný XML/API), ověřit
počty, součty, úhrady a vazby proti zdroji. Pokud export obsahuje jen seznam,
neslibovat kompletní migraci faktur. Rozšíření cizích měn a ostatních dokladů
navazuje na příslušné doménové etapy.

Ověřeno 11. 10. 2026: 8 importních testů (včetně parserů, upgrade, izolace,
souběhu, pokračování, REST/MCP a CSRF), 20 regresí firem/faktur/úhrad v reálném
PostgreSQL; čtyři Playwright scénáře CS/EN × desktop/mobil s axe. XLSX fixture
je vlastní minimální OOXML soubor. Prošly typecheck, locale/conventions,
format, clippy a produkční web build. Kontrolní snímky `docs/design/import-*`.
