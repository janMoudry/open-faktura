# P19-01m — Přehled uložených kurzových rozdílů

Vlastní návrh, 17. 10. 2026, navazuje na E-067 a bank-foreign-matching.md.
Nejde o doloženou shodu reportu iDokladu ani účetní deník. Bez nové migrace.

Read-only report aktivního profilu čte aktivní bank_match s uloženým
foreign_settlement. Datum období je booked_on zdrojového pohybu, včetně
obou hranic. Odpojené vazby a běžné CZK vazby vylučuje. Připojení existující
úhrady se započte stejně jako vytvoření nové: právě jedna aktivní vazba.

Součty: gains = součet kladných difference_czk; losses = absolutní součet
záporných; net = gains − losses, vše NUMERIC/CZK bez nového kurzového
přepočtu či zaokrouhlení. Prázdný report vrací nuly. Zdrojové řádky obsahují
bankovní pohyb, fakturu, datum, měnu/částku úhrady, bankovní CZK alokaci,
uloženou hodnotu dle kurzu faktury a rozdíl. Řazení datum DESC, ID vazby DESC.
Stránka 25 řádků, součty vždy za celý filtr. Změna období resetuje stránku.

Členství vyžadováno, viewer smí číst. Součty a řádky jsou ze stejné
REPEATABLE READ transakce. REST bank_foreign_report a MCP sdílejí BankStore.
Období validované jako ostatní reporty, maximálně 3653 dní; page 1–10000.
CS/EN sekce na stránce Reporty sdílí její filtr od/do; ostatní přepínače
prodejního reportu nemění význam kurzového přehledu.

Akceptace: kladné/záporné/nulové rozdíly, příjem/výdaj, původní i nové úhrady,
odpojení a opětovné připojení, přesné hranice období, stránkování bez změny
součtů, izolace profilu, REST/MCP a UI CS/EN desktop/mobile.
Zbývá cizoměnová banka/importy, zálohy/vratky a rozdílné kurzy vyúčtování.

## Ověření 17. 10. 2026

Osm izolovaných PostgreSQL scénářů bank/foreign.rs prošlo včetně reportu
nad 26 vazbami, stránkování, hranic období, izolace, odpojení a REST/MCP
shody se čtením pro viewer. Částky jsou syntetické očekávané výsledky
vlastního pravidla, nikoli pozorování iDokladu.

Bank E2E: CS/EN desktop/mobile ověřily zobrazení uložené ztráty, vyloučení
odpojené vazby a funkční odkaz na pohyb. První český mobilní běh vypršel
na screenshotu staršího importního dialogu, před reportem; opakování
téhož scénáře prošlo. Celkem všechny čtyři kombinace úspěšně ověřené.
Screenshoty exchange-difference-report-* jsou syntetická testovací agenda;
vizuálně zkontrolován desktop a mobil, upraveno zalamování identifikátorů.

Prošly pnpm check, dodatečný Clippy všech serverových cílů, typy/lokalizace,
formát i webový/serverový build. Stávající upozornění na větší webové chunky
zůstává. Lokální API restartováno a ready ověřeno; bez nové migrace.

P16-13g: report zahrnuje i různé měny z cizoměnových účtů. Bankovní alokaci
zobrazuje v settlement.bank_currency (legacy CZK), součty nadále pouze CZK
z uložených difference_czk; nic nepřepočítává.
