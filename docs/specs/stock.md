# P18 — Sklad

Podklad E-025/E-053. Stav: implementace, 15. 10. 2026.
Přesnost, transakce a pravidla inventury níže jsou vlastní rozhodnutí.

Jedna skladová karta odpovídá ceníkové položce v aktivním profilu. Sledování
je opt-in. Zapnutí zapíše počáteční pohyb se snapshotem názvu a jednotky,
datem a množstvím. Existující faktury se zpětně nepřepisují. Množství je
Decimal s nejvýše čtyřmi desetinnými místy a absolutním limitem 1 000 000
na operaci; saldo má limit 1 000 000 000. Záporná zásoba je viditelná.

Ruční příjem/výdej vyžaduje datum a důvod. Historické pohyby se nemažou;
oprava vytvoří opačný pohyb a odkaz na původní. Opakování UUID vrací tentýž
výsledek. Firemní oprávnění a složené FK platí i pro ceník a vazby dokladů.

Inventura zapíše fyzicky zjištěný stav a přesný rozdíl proti současnému
saldu, s očekávanou revizí karty. Používá dnešní datum; souběžná změna
vyžaduje nový náhled. Počáteční datum omezuje starší pohyby. Název a jednotka
v historii zůstávají snapshoty. Vypnutí sledování dosud není implementované.

Napojení prodeje/nákupu musí používat explicitní catalog_id, nikdy název
položky. Vystavení, oprava a skladový dopad jsou jedna transakce. Historické
doklady bez vazby nedostanou odhadnutý skladový pohyb. Návrh nevyskladňuje.
Dobropis vrací jen skutečně opravené množství. Příjem dokladu není sám o sobě
potvrzením fyzického naskladnění.

Akceptace: přesné desetinné stavy; replay/souběh/role/cizí profil; nulové
a záporné množství; reverz bez ztráty historie; inventurní konflikt;
období nezávislé na stránkování; REST/MCP a CS/EN UI; návazné doklady.

## P18-03a — statistika vybrané karty (17. 10. 2026)

Vlastní návrh nad evidencí E-053, nikoli potvrzená numerická shoda iDokladu.
Odpověď historie obsahuje statistiku pouze při výběru jedné existující karty;
bez výběru nebo před jejím založením je null. Nesčítáme různé položky/jednotky.
Počáteční saldo je součet před `from` (bez dolní meze nula), příjmy a výdeje
jsou kladné a absolutní záporné ruční a dokladové pohyby včetně obou mezních dnů.
Počáteční zápis, inventury a reverzy v období jsou samostatné podepsané úpravy.
Konečné saldo = počáteční saldo + příjmy − výdeje + úpravy.
Bez horní meze zahrnujeme celou historii. Součty, počet i stránka pocházejí
z jednoho REPEATABLE READ snapshotu a statistika není omezená stránkováním.

Ověření P18-03a: `pnpm test:stock` zahrnuje přesné součty s inventurou a
reverzem, období mimo počáteční zápis, 27 pohybů přes dvě stránky, izolaci
profilů a totožnou REST/MCP odpověď. UI regrese v `tests/e2e/catalog.spec.ts`
kontroluje konečné saldo a výdeje v CS/EN na desktopu i mobilu.
Neobsahuje ocenění ani souhrnnou statistiku více karet; navazuje P18-02.

## P18-02 — vazby dokladů (17. 10. 2026)

E-053 dokládá pohyby z dokladů. Následující přesná pravidla jsou vlastní
návrh, nikoli ověřená numerická shoda iDokladu. Migrace 044 přidává profilové
FK na fakturu, nákup, dobropis a prodejku a identitu zdrojového řádku.
Historické doklady se nedoplňují odhadem ani podle názvu.

- Výběr ceníku ukládá volitelné `catalog_id` do řádku. Návrhy, šablony,
  nabídky a zálohy zásobu nemění. Vystavení konečné faktury sníží zásobu
  sledované karty o přesné množství, bez přepočtu cen nebo jednotek.
- Nákupní řádek může mít explicitní `stock` (karta, množství, jednotka).
  Uživatel tím potvrzuje fyzický příjem. Uložení/oprava/storno provádí
  rozdíl proti již zaevidovanému množství ve stejné transakci, k dnešku.
- Prodejka při uložení/změně/stornu obdobně zapisuje výdej nebo rozdíl.
- Dobropis má volitelné `return_quantity`. Bez něj jde pouze o finanční
  opravu. Součet vratek nesmí překročit původní skutečný skladový výdej
  řádku. Staré faktury bez skladové vazby nemají automatickou vratku.
- Změna karty/množství nákupu či prodejky vytvoří dohledatelné rozdílové
  pohyby. Přímý reverz zdrojového pohybu mimo jeho doklad je zakázaný.
- Množství > 0, nejvýše čtyři desetinná místa, stejná jednotka karty;
  záporná zásoba je dovolená. Archivovanou kartu nelze použít pro nový
  příjem/výdej, ale storno původního dokladu se musí dát dokončit.
- Vše se účtuje v transakci dokladu; replay vrací původní výsledek. Pohyby
  z dokladů vstupují do příjmů/výdejů statistiky, nikoli inventurních úprav.

Před upgradem zastavit staré API, zálohovat DB a spustit nový proces.

Ověření P18-02: `pnpm test:stock` (včetně upgrade 043), `pnpm test:retail`,
regrese invoice/received/credit_note/sales; CS/EN desktop/mobile scénáře
`catalog.spec.ts` a `received.spec.ts` ověřují uloženou vazbu a skutečný
příjem z formuláře. Původní limity úprav zaplacených a zaúčtovaných dokladů
zůstávají zachované. Referenční numerická shoda dosud není potvrzená.

## P18-03b — souhrnné stavy a explicitní ocenění

Podklad E-056, 17. 10. 2026. Stock report vrací skladové karty včetně
archivovaných, existující k `as_of` (výchozí dnešek v Europe/Prague, budoucí
stav odmítá). Množství je součet všech pohybů do data včetně; názvy a kódy
jsou současné. Hledání je doslovný podřetězec názvu/kódu. Stránka má 25 řádků,
součty jsou za celý filtr v jednom REPEATABLE READ snapshotu. Nejvýše 10 000
karet ve filtru, větší výsledek vyžaduje užší hledání. Množství různých
jednotek se nikdy nesčítá do jednoho čísla.

Ocenění je vlastní explicitní metoda: množství × ručně zadaná skladová cena
CZK/jednotku (0 až 1 000 000 000, nejvýše 4 desetinná místa), zaokrouhlení
každé položky na 2 místa, MidpointAwayFromZero. Součet je součtem těchto
hodnot. Nejde o prodejní potenciál, automatické FIFO ani průměrné pořizovací
ceny a nevytváří se účetní zápis. Uživatel volí vhodnou skladovou cenu;
backend ji neodvozuje z DPH, prodejní ceny ani pouhého nákupního dokladu.

Cena má datum platnosti, důvod, autora a rostoucí revizi v migraci 045.
Zápisy jsou append-only, očekávaná revize řeší souběh, UUID replay. Volí se
nejnovější datum platnosti do as_of a na stejný den nejvyšší revize. Zpětná
oprava může změnit historický výsledek přehledu; nejde o uzavřenou účetní
sestavu. Historie poskytuje posledních 100 změn. Prázdná cena explicitně
obnoví neznámé ocenění. Zero cena je vědomě zadaná nula; nulová zásoba má
nulovou hodnotu i bez ceny. Záporná zásoba má neznámou hodnotu. Report vrací
známý mezisoučet a počet neoceněných/záporných karet; nesmí předstírat úplný
součet, pokud některé hodnoty chybí. Viewer pouze čte.

Před migrací 045 zálohovat DB a zastavit staré API. Staré zásoby zůstanou
bez ceny; žádné hodnoty se nedoplňují odhadem.

## P18-03c — pořizovací vstup a průběžný průměr (vlastní pravidlo)

Pořizovací cena za jednotku v CZK je volitelný explicitní údaj fyzického
příjmu na nákupním dokladu, max. 1 miliarda, 4 desetinná místa. Uživatel zadává
cenu včetně jím rozvržených vedlejších nákladů a neodpočitatelné DPH; aplikace
ji neodvozuje z prodejního ceníku ani automaticky z daňového základu. Prázdná
cena znamená neznámou, nula bezplatný příjem. Migrace staré příjmy neoceňuje.

Automatické ocenění je oddělené od ručního P18-03b. Příjmy přičítají přesnou
hodnotu množství × uložená pořizovací cena, výdej odebírá poměr aktuální hodnoty
a množství. Mezivýsledky se zaokrouhlují na 12 desetinných míst, půlky od nuly;
zobrazená hodnota karty na 2 a průměr na 4. Report řadí pohyby podle data a ID.
Přepočet zpětně datovaných pohybů není uzavřený účetní deník.

Neoceněný příjem, kladné inventurní rozdíly, vratky bez doložené původní ceny,
opravované nákupní příjmy a záporná zásoba činí automatickou hodnotu neznámou.
Nulový zůstatek hodnotu vynuluje. Další plně oceněný příjem může zahájit nový
průměr. Nulový otevírací pohyb není chybějící cena. Oprava ceny bez změny
množství přidá nulový auditní pohyb označený jako neocenitelná oprava; nesmí
ponechat staré ocenění zdánlivě platné. Storno příjmu zpracuje běžný množstevní
protipohyb. Historické snapshoty se nepřepisují.

Tento díl nedokončuje ocenění vratek a oprav již spotřebovaných příjmů;
nezobrazuje odhad jako účetní hodnotu. Také chybí samostatné rozvržení
vedlejších nákladů a pořizovací ceny ručních/počátečních příjmů. Numerická shoda
s iDokladem není doložena (E-056). Akceptace: dva nákupy s různou cenou,
výdej s váženým průměrem, neznámá cena, nula, záporný sklad, oprava pouze ceny,
profilová izolace, REST/MCP, CS/EN formulář a report.

Automatický report zpracuje nejvýše 100 000 pohybů vybraných karet; při
překročení vrátí validační chybu místo částečného součtu. Filtr názvu/kódu
a stránkování mají stejná pravidla jako ruční report.

## P18-03d — vratky, cenové opravy a vedlejší náklady

Tato část nahrazuje konzervativní zneplatnění opravených nákupních příjmů
a vratek z P18-03c. Staré snapshoty se zachovávají, přepočet je odvozený.

Vlastní pravidlo navazuje na E-056, nejde o doložený algoritmus iDokladu.
Report k datu použije poslední známou cenu každého nákupního řádku platnou
podle pohybů do tohoto data a přepočte jeho příjmy i navazující výdeje.
Oprava ceny tedy přerozdělí rozdíl do zásoby a čistého výdeje; report obě
částky ukáže proti ceně prvního příjmu. Jde o přepočitatelnou skladovou
analytiku, ne zápis účetní opravy do hlavní knihy. Auditní pohyby se nemění.
Storno příjmu zachovává předchozí cenu pro výpočet odchodu zásoby.

Dobropis vrací cenu původního výdeje. Prodejka s více dílčími výdeji vrací
váženou cenu dosud nevrácené části téhož řádku. Ruční storno použije cenu
svého zdrojového pohybu. Zpětně datovaná vratka před zdrojovým výdejem ani
vratka neoceněného výdeje nevytváří odhadovanou cenu. Záporné množství nebo
hodnota jsou neznámé; nulové množství vynuluje skladovou hodnotu.

`stock_incidental_cost` na nákupním dokladu je volitelná celková částka CZK,
max. miliarda, 2 desetinná místa; výchozí nula. Rozvrhuje se pouze mezi
explicitní skladové řádky poměrem množství × základní pořizovací cena.
Zaokrouhlení dolů na haléře, zbytek metodou největších zbytků, shoda podle
pořadí řádku. Součet přídělů přesně odpovídá zadané částce. Nenulové náklady
vyžadují známé ceny všech skladových řádků a kladný součet vah. Částka není
přičítána k závazku dodavateli; vstupuje pouze do skladového ocenění a nesmí
být už zahrnuta v základní pořizovací ceně. Příděl na řádek a jeho celkové
množství se uloží do snapshotu pohybu. Náklady za jednotku se počítají na
12 míst; výstup na 2 místa. Neřešíme automatické dohledání dopravní faktury.

Akceptace: nákup 10×20, výdej 4, oprava ceny na30 → sklad180, čistý výdej120,
dopad do skladu60 a do výdeje40; vratka2 → sklad240, čistý výdej60.
Rozvržení 0,05 na tři stejné váhy → 0,02/0,02/0,01. Storno, částečná
vratka, změna samotných nákladů, opakování, profilové role, stará data,
REST/MCP a UI CS/EN musí zachovat stejné výsledky. Počáteční a ruční příjmy
bez pořizovací ceny nadále zůstávají otevřenou částí P18.

Při fyzickém stornu již spotřebovaného dražšího příjmu může vzniknout
záporné množství nebo záporná odvozená hodnota. Tyto stavy se zobrazí jako
neznámé; automaticky se nevytváří vyrovnávací účetní zápis. Nulová zásoba
má nulovou skladovou hodnotu, případný nevysvětlený rozdíl zneplatní cenu
čistého výdeje. Není dovoleno tento rozdíl skrýt jako známý nulový náklad.

Zobrazené rozdělení cenové opravy zaokrouhluje nejprve celkový rozdíl na
haléře, potom skladovou část; část výdeje je přesný haléřový zbytek. Např.
0,005 + 0,005 → 0,01 do zásoby a 0,00 do výdeje, nikoli celkem 0,02.
Stejné pravidlo platí pro záporné opravy. Jednotlivé absolutní analytické
hodnoty se zobrazují samostatně zaokrouhlené; nejsou účetními zápisy.

## P18-03e — pořizovací cena ručních příjmů

Návrhové rozhodnutí Open Faktury, nikoli ověřená numerická shoda s iDokladem:
počáteční kladný stav, ruční příjem a inventurní přebytek mohou nést volitelnou
jednotkovou pořizovací cenu v měně profilu, bez DPH, 0 až 1 miliarda, nejvýše
4 desetinná místa. Prázdná cena znamená neznámou, nula známý bezúplatný příjem.
U inventury se cena vztahuje pouze na kladný rozdíl, nikoli celý zjištěný stav.
Cena u výdeje, nulového rozdílu nebo storna se odmítne. Storno přebytku/příjmu
odpočítá původní hodnotu a nemění hodnotu výdejů. Staré pohyby se nedoceňují.
Akceptace: 10 × 20, výdej 4, příjem 5 × 40, přebytek 2 × 30: sklad 380,
výdeje 80. Storno přebytku a příjmu vrátí sklad na 120 a výdeje zůstanou 80.

Ověření P18-03e (17. 10. 2026): `pnpm test:stock` 10 testů, regrese
`retail` 5 testů, `cargo test -p faktura-domain stock_cost` 8 testů;
`catalog.spec.ts` CS/EN desktop/mobile včetně pořizovací ceny a automatické
hodnoty skladu. `pnpm check` a `pnpm build:web` prošly. Numerická shoda
s referenčním iDokladem zůstává neověřená.
