# P16-04 — pokladní evidence

Stav: implementováno v níže vymezeném rozsahu, 14. 10. 2026. Reference E-044.

## Rozsah

Profilové CZK pokladny s názvem, počátečním stavem a datem. Po prvním dokladu
je počáteční stav a datum neměnné; název lze upravit a pokladnu archivovat.
Příjmy a výdaje mají vlastní roční řady PP{rok}{šestimístné pořadí} a VP{rok}{šestimístné pořadí}, datum, partnera,
osobu, popis a přesnou částku. Záporné saldo je dovoleno a zobrazeno, nikoli
skrytě opraveno. Doklady před počátečním datem a v budoucnu jsou odmítnuté.

Uložený doklad je neměnný; chybu řeší storno s důvodem a nový doklad.
Storno se nepočítá do salda. Přehled počítá počáteční stav, příjmy, výdaje
a konečný stav za zvolené období přes všechny stránky.

Párování příjmu s vydanou fakturou a výdaje s přijatou fakturou vytvoří
hotovostní úhradu společným příkazem v téže transakci. Lze připojit i existující
hotovostní úhradu přesné částky, včetně účtenky, bez druhé platby. Jedna úhrada
má nejvýše jednu aktivní vazbu. Datum existující úhrady musí souhlasit s dokladem.
Odpojení stornuje pouze úhradu vytvořenou pokladnou; dřívější zachovává.
Spárovaný doklad nejprve odpojit, teprve poté stornovat. Připojenou platbu
nelze opravit mimo pokladnu. Viewer pouze čte, editor/owner zapisuje.

REST a MCP sdílejí příkazy. UUID požadavku, obsah a aktér určují opakování;
změna obsahu téhož klíče je konflikt. Číslování, úhrada, vazba a audit jsou
atomické. Revize brání přepsání souběžných změn. Firemní FK a autorizace
platí pro každou navázanou entitu. Migrace 029 je aditivní; před nasazením
zastavit staré API, které ještě nechrání hotovostní vazby.

## Hranice shody

Jde o samostatnou evidenci hotovosti, nikoli nový daňový výpočet.
Automatické generování při úhradě mimo pokladnu, zálohy/dobropisy, prodejky,
DPH samostatných pokladních plnění, cizí měny, přizpůsobení číselných řad
ani tisk/PDF nejsou součástí této etapy. Předchozí hotovostní úhrady se
nemigrují automaticky; uživatel je připojí výslovně. Neměnnost počátku,
storno namísto mazání a kontrola data jsou naše konzervativní rozhodnutí,
nikoli pozorovaná serverová pravidla iDokladu.

## Akceptace

- Více pokladen a profilů, revizní konflikt, viewer bez zápisu, cizí ID odmítnuto.
- PP/VP číslování, souběh a opakování bez duplicit, validace přesnosti/datu.
- Saldo přes všechny stránky, období, storno a počáteční stav.
- Nová i existující úhrada, částečná úhrada, odpojení, blokace samostatného storna.
- REST/MCP shoda, CS/EN, klávesnice, mobil a přístupnost.

## P16-05 — automatické doklady a PDF

Automatizace je implicitně vypnutá. Editor/owner výslovně vybere hlavní CZK
pokladnu; nastavení má vlastní revizi, audit a idempotentní příkaz. Vybranou
pokladnu nelze archivovat před změnou/vypnutím automatiky. Každá nová skutečná
hotovostní úhrada vydané/přijaté faktury a hotovostní účtenka vytvoří právě
jeden PP/VP a vazbu ve stejné transakci. Částečné úhrady mají vlastní doklady.
Změna platební metody faktury bez zápisu úhrady nic nevytváří. Párování
ručně založeného pokladního dokladu další automatický doklad nevytvoří.
Předchozí platby, zálohy a dobropisy tato etapa automaticky nepřevádí.

Při stornu/opravě zdrojové úhrady se automatický pokladní doklad stornuje
s auditovaným důvodem, jeho vazba se uzavře. Náhradní cash platba vytvoří nový
doklad do aktuálně vybrané pokladny; bank/card náhrada nikoli. Oprava účtenky
stejně nahrazuje její původní doklad. Ručně připojené doklady nadále blokují
opravu zdrojové platby až do explicitního odpojení. Automatický doklad se
opravuje pouze ze zdroje; nelze jej ručně odpojit nebo stornovat z pokladny.
Vypnutí automatiky neruší staré doklady a nezabraňuje jejich správnému stornu.
Selhání pokladního zápisu vrátí celou platbu; datum před počátkem vybrané
pokladny je explicitní chyba, nikoli přeskočená hotovost.

PDF/náhled tisku v CS/EN je dostupný všem členům profilu. Jazyk dokumentu se
volí nezávisle na UI. Výstup obsahuje číslo, datum, uložené údaje vystavitele,
partnera/osobu, popis, částku a podpisová místa. Stornovaný doklad má jasné
označení. Údaje vystavitele jsou snapshot při vzniku; u starších dokladů 029
migrace zachytí současný profil (původní historická verze nebyla uložená).
Cache PDF je po revizi/jazyku, renderer používá lokální fonty bez síťového I/O.
Náhled nabízí PDF i systémový tisk prohlížeče; fyzický tisk se netestuje.

Reference E-045 dokládá hlavní pokladnu, automatické doklady a jazyky tisku.
Opt-in, transakční hranice a opravy stornem jsou naše vlastní pravidla.
Migrace 030 je aditivní; před upgradem zastavit staré API. Prodejky, DPH
samostatných plnění, částka slovy, konfigurace vzhledu a cizí měny zůstávají otevřené.

## P16-06 — samostatná položka s DPH

Jedna samostatná položka pokladního dokladu, dle veřejné nápovědy E-046.
Dostupná pouze profilu plátce DPH. Uživatel zadá popis, datum plnění, cenu,
typ ceny net/gross a sazbu 0/12/21. `tax_manual` je oddělený příkaz od
`manual`: vstupní `data.amount` je podepsaná cena položky; výstupní částka
pohybu je vypočtená cena včetně DPH se stejným znaménkem. Náhled
`cash_tax_preview` vrací tentýž serverový výpočet bez zápisu. Vstupní cena
má max. dvě desetinná místa, absolutně nejvýše 1 miliardu CZK; nula není dovolena.

Používá se kalkulátor `of-czk-v1`, množství 1, bez slev a dodatečného
zaokrouhlení; pravidla viz invoice-calculation.md. Kladný daňový rozpis je
uložen spolu se vstupní podepsanou cenou, datem, sazbou, typem ceny a verzí
výpočtu. Směr určuje pohyb. Pozdější změna profilu jej nepřepočítá. Nejde o
přepis původního dodavatelského rozpisu; pro ten slouží přijaté faktury/účtenky.

Samostatné plnění nesmí být současně úhradou jiné faktury. Kandidáti párování
jsou prázdní a příkaz match je odmítnutý. Běžné/automatické úhrady mají tax=null,
aby nevytvářely druhé daňové plnění. Storno s důvodem zachovává snapshot a
vyřazuje pohyb ze salda. Číslování, rozpis a audit vznikají atomicky, souběh
stejného request_id nevytváří další doklad. Viewer čte a může zobrazit náhled,
neukládá. Přístup je vždy ověřen v profilu; příkaz nemůže použít cizí pokladnu.

Vlastní konzervativní pravidlo: datum plnění nesmí být později než datum
pohybu, v budoucnu ani před tax_effective_from (je-li vyplněné). Profil
historii plátcovství neobsahuje. Nejde o potvrzení těchto omezení u iDokladu.
PDF CS/EN a detail používají uložené hodnoty. Migrace 031 přidává nullable
JSONB tax, bez převodu starých dokladů; před upgradem zastavit staré API.

Akceptace: net 100/21 → 121; gross 121/21 → net 100; expense záporné saldo
při kladném rozpisu; hranice přesnosti a sazeb; storno; izolace, viewer,
změna plátcovství; idempotence/souběh; rollback po přidělení čísla; REST/MCP
náhled; PDF a CS/EN mobil/desktop. Příklady jsou syntetické, vlastní pravidla.

Neobsahuje prodejky a souhrnné zaúčtování, členění DPH, přiznání/výkazy,
přenesenou povinnost, jiné sazby/měny ani doplnění úplných identifikačních
údajů partnera. Samotný tento výstup není deklarován jako plnohodnotná
náhrada všech zákonných daňových dokladů. Následující P16-07 má samostatně
zavést prodejky a oddělit jejich platby od následného zaúčtování.

## Částka slovy — díl P16-16

Nově renderované pokladní PDF obsahuje absolutní částku slovy v CS/EN v CZK
včetně haléřů. Převod pracuje s desetinným řetězcem, nikoli float. Znaménko
pohybu určuje druh dokladu, na příjmovém/výdajovém formuláři je částka kladná.
Existující PDF cache zůstává historicky neměnná; staré cache se nehromadně
nepřegenerovávají. Konfigurace sestav a více kopií nadále zbývají.
