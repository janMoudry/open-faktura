# P07 — vystavení, číslování a PDF

Implementace 11. 10. 2026, INV-004/005/008/009. Navazuje na P06 a vlastní
kalkulátor `of-czk-v1`. E-017 dokládá uživatelské funkce iDokladu; níže uvedené
zámky, neměnnost a validace jsou explicitní návrh Open Faktury, nikoli doložené
interní chování reference. P07 podporuje tuzemské CZK doklady v režimu plátce
nebo neplátce. Zahraniční a zvláštní daňové režimy zůstávají mimo tento rozsah.

## Příkaz a validace

`POST /api/companies/{company}/invoices/{id}/issue` přijímá `request_id` UUID
 a očekávanou `revision` uloženého návrhu. Vystavují owner/editor. Viewer smí
číst včetně PDF. Cizí profil, nečlen i smazaný profil vrací 404; viewer při
zápisu 403. HTTP vyžaduje relaci, stejný origin a X-Faktura-Request.
MCP `invoice_issue` volá stejný příkaz a autorizaci.

Požadovány jsou názvy a adresy obou stran (ulice, město, PSČ, CZ), IČO dodavatele,
datum vystavení a splatnost nejdříve v den vystavení; u plátce též DIČ a DUZP.
Musí existovat alespoň jedna pojmenovaná položka s kladným množstvím a kompletní
validní kalkulací. Nulová cena je přípustná. Bankovní převod vyžaduje účet nebo
validní IBAN. VS je nejvýše 10 číslic. Chyby 422 nespotřebují číslo; UI zobrazuje
lokalizovaný seznam chybějících/chybných polí. Daňový režim odpovídá historii
profilu k DUZP u plátce, datu vystavení u neplátce; bez historie se použije
nastavení profilu. Pokud historie existuje, ale nepokrývá dané datum, vystavení
se odmítne místo odhadnutí minulého režimu.

UI vyžaduje nejdříve uložení návrhu. Potvrzení vysvětluje konečné číslo a
uzamčení obsahu. Náhled čísla je nezávazný — souběžný doklad jej může spotřebovat.
Vystavení nic neodesílá ani neoznačuje fakturu za uhrazenou, a to ani u hotovosti.

## Číselná řada a souběh

Formát: prefix + čtyřmístný rok + šestimístné pořadí (např. FV2026000001).
Rok data vystavení musí odpovídat nastavenému roku řady. Žádný automatický reset
na Nový rok: uživatel vybere rok v nastavení. Rozsah pořadí 1–999999; 1000000 je
vyčerpaná řada. `invoice_number_series` má klíč profil/prefix/rok a trvalý horní
stav počítadla: snížení čísla v profilu ani návrat ke starému prefixu čísla nevrací.

Transakce nejdříve zamkne profil/členství, potom návrh a řadu. Kontrola revize,
výpočet, přidělení čísla, zvýšení počítadla, snapshot, audit dokladu i aktualizace
revize/nastavení profilu se commitují společně. DB vynucuje unikátní číslo v celém
profilu a unikátní klíč vystavení. Prázdný VS se doplní rokem a pořadím bez prefixu.
Stejný profil/klíč/aktér/ID/vstupní revize vrací původní výsledek; změna kterékoliv
části je 409. Již vystavený doklad s jiným klíčem je také konflikt, ne nové číslo.

## Neměnný detail a PDF

Existující `invoice_draft` zůstává agregátem i po vystavení: nullable `number`
a `issued_at` rozlišují stavy. Po vystavení jsou nenulové také `issue_request_id`,
`issue_actor_id`, `issue_revision` a `totals`. PUT je odmítnut. DB trigger blokuje
změnu a smazání vystavených řádků i ze starého backendu. Povolen je pouze první
zápis PDF cache; opravy dokladů patří do P13. Kopie vytváří nový návrh a vymaže VS.

Detail zobrazuje uložené součty a snapshot, nikoli nový přepočet ani živé údaje.
Seznam nabízí vše/rozpracované/vystavené; hledání zahrnuje číslo a první položku.
Číslo vystavené faktury nahrazuje název návrhu. Stav je „Vystaveno“/„Issued“;
Stavy úhrad a související filtry doplňuje [P08](invoice-payments.md).

`GET .../{id}/pdf` vrací application/pdf, inline pro prohlížení/tisk; UI nabízí
samostatné stažení s číslem v názvu. Návrh vrací 409. MCP `invoice_pdf` vrací
mime_type a base64 stejného souboru. Renderer genpdf 0.2.0 je čistý Rust, běží
v spawn_blocking mimo transakci, nejvýše dva rendery souběžně. Font Noto Sans
včetně OFL licence je vložen do binárky a PDF. Šablona v CS/EN se řídí jazykem
dokladu, nikoli aktuálním UI; obsahuje položky, slevy, daně, zaokrouhlení,
platební údaje, poznámku a číslo stránky. Dlouhé popisy se zalamují, obsah
přetéká na další stránky. PDF se po prvním vytvoření uloží do PostgreSQL;
při změně profilu, adresáře či rendereru se již uložené PDF nepřepisuje.
QR platba, podpis, logo a zákaznické šablony nejsou součástí této první šablony.

## Migrace a ověření

008 je aditivní: všechny existující řádky zůstávají návrhy, obsah se nemění.
ALTER krátce zamyká tabulky; nasazovat společně s backendem a klientem. Starý
backend může číst, ale trigger zabrání jeho pokusu o přepsání vystavené faktury.
Existující zálohy PostgreSQL zahrnují také PDF; ukládání není externí služba.

`tests/invoice.rs`: souběh různých dokladů, souběžné opakování téhož příkazu,
změněný klíč/vstup, revize, autorizace, DB neměnnost, rewind čítače, PDF po změně
profilu, REST/MCP shoda, čistá DB a upgrade P06 se syntetickým návrhem.
`tests/e2e/invoices.spec.ts`: CS/EN desktop/mobil, chybný návrh, potvrzení vystavení,
reload zamčeného detailu, skutečné stažení PDF a filtr vystavených dokladů.
PDF syntetické vzorky zahrnují českou diakritiku a 80 dlouhých položek (18 stran).
Testy dokládají implementaci našich pravidel, ne účetní shodu s iDokladem.
