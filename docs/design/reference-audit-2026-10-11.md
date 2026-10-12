# Audit skutečného průchodu referencemi — 11. 10. 2026

Rozsah: odběratel → návrh → vystavení → úhrada, seznam a detail dokladu.
Tento audit nahrazuje dojem shody založený pouze na screenshotu veřejného návodu.
Technická implementace P01–P09 sama o sobě neznamená referenčně ověřené chování.

## FakturaOnline — E-020, přihlášené rozhraní

Prostředí: Chrome, webová aplikace, české UI, profil neplátce DPH. Build není v UI
uveden. Prohlédnut skutečný účet pouze pro čtení; žádný kontakt, doklad ani úhrada
nebyly vytvořeny, změněny nebo odeslány. Osobní údaje ani doklady se do repozitáře
nekopírují. Otevření prázdného formuláře a rozbalení polí nejsou důkaz uložení.

| Scénář | Kroky a pozorování | Rozdíl Open Faktury | Oprava / stav |
| --- | --- | --- | --- |
| UX-01 | `/faktura`: po ustálení načtení základní údaje, obecné nastavení sbalené, dodavatel jako souhrn, odběratel jako hledání, položky, rekapitulace a výstupní akce | Editor ukazoval všechna pole zákazníka a začínal dodavatelem | Data/platba přesunuta na začátek; ruční pole zákazníka na vyžádání |
| UX-02 | Klik „Zadám údaje ručně“ odhalí jméno, IČO, DIČ, adresu a kontakty | Ruční pole soutěžila s vyhledáním od prvního zobrazení | Hledání zůstává primární; výběr uloženého kontaktu přejde na souhrn, úprava snapshotu zůstává dostupná |
| UX-03 | `/vystavene-faktury`: sloupce číslo, odběratel, splatnost, typ, částka, stav, akce | Platební badge byly uvnitř názvu a obtížně se porovnávaly mezi řádky | Samostatný stav úhrady; draft badge vedle názvu zachován na výslovné přání uživatele |
| UX-04 | „Vybrat filtry“ otevře nabídku období, odběratel, typ dokladu, typ platby, cena; nad řádky stavové součty a hromadné akce | Máme jen hledání, odběratele a stav | Otevřená mezera; bez globálních agregací nepředstírat součty aktuálně načtené stránky jako celkové |
| UX-05 | Klik čísla otevře detail s náhledem dokumentu, stavem, upravit/duplikovat/odeslat/tisknout/smazat a historií | Náš detail je strukturovaná datová karta, PDF zvlášť; vystavený doklad je neměnný | Záměrná neměnnost vyžaduje vlastní specifikaci oprav; náhled a historie zůstávají rozdíl |
| UX-06 | `/kontakty`, Nový kontakt: jméno a IČO s dohledáním, adresa, kontakty, Zahodit/Uložit | Máme dohledání podle IČO, ale ne jména | Existující ARES není úplná shoda; hledání podle názvu neověřeno |
| UX-07 | Detail ukazuje přepínač zaplacení; změna nebyla provedena na skutečné faktuře | Naše dílčí úhrady mají samostatný dialog a audit | Výsledek zápisu, částečné úhrady a storna ve FaktuřeOnline neověřen |

Zdrojové URL: https://www.fakturaonline.cz/faktura,
https://www.fakturaonline.cz/vystavene-faktury,
https://www.fakturaonline.cz/kontakty. Detail otevřen přes skutečný řádek; jeho
identifikátor ani obsah zde z důvodu oddělení testovacích a skutečných dat neukládáme.

## iDoklad — průběh získání testovacího prostředí

Původní přihlášený účet přesměrovával Faktury vydané na skončené předplatné.
Uživatel požádal o nový náhodný účet určený jen pro průzkum a pozdější smazání.
Založena vlastní dočasná schránka přes https://mail.tm/; žádná smyšlená adresa
cizího příjemce. Přístupové údaje jsou pouze v ignorovaném souboru
`.secrets/research-mail.json` s oprávněním 0600, nikoli v dokumentaci/fixtures.
Registrace dovoluje „Nemám IČ“ a vyžaduje ověření e-mailu. Heslo má limit 8–32
znaků; delší bylo odmítnuto před registrací. Následné nastavení požaduje název,
ulici, PSČ, město, režim DPH a typ subjektu. Telefon není povinný.
Použita syntetická agenda bez IČO, neplátce DPH, fyzická osoba, CZ.

## Pravidlo další práce

Každá oprava musí uvést scénář, přímo pozorovaný výsledek, úmyslnou odchylku a
vlastní test. Dokumentovaná nabídka akce není ověřený výsledek zápisu. Numerické
scénáře R-01 až R-09 zůstávají neověřené, dokud se skutečně neprovedou v testu.

## iDoklad — E-021, dokončený základní průchod

Testovací registrace aktivována. Bez zadání platební karty; UI ukazuje zkušební
Prémiový tarif na 60 dní. Účet a syntetické doklady zatím ponechány pro další
průzkum a pozdější smazání podle přání uživatele.

| Scénář | Přímo ověřený výsledek | Důsledek pro Open Fakturu |
| --- | --- | --- |
| UX-08 | Plus u odběratele otevírá dialog a po uložení zvolí kontakt | Naše explicitní uložení do adresáře z editoru zachováno; samostatný dialog je zatím odchylka |
| UX-09 | Editor ukazuje číslo 20260001 před uložením, Uložit vede na seznam vydaných | Návrhy a atomické vystavení jsou vlastní životní cyklus; předběžnou serverovou rezervaci čísla nelze tvrdit |
| UX-10 | Chybějící banku lze po varování obejít akcí Přesto uložit | Ověřit proti našemu pravidlu vystavení; automaticky neměnit účetní validace podle jednoho UI případu |
| UX-11 | Seznam má 4 hlavní pohledy, samostatný stav a akce v řádku, řazení přes hlavičky; rozšířený filtr má data a cenu | Stav úhrady oddělen; přidány reálně funkční akce PDF/kopie. Přebytečné hlavní pohledy, datum/cena a řazení hlavičkou jsou otevřené rozdíly |
| UX-12 | Detail má záložky Doklad/Přílohy/Úhrady, akční lištu, přístupnou úpravu vydaného dokladu | Naše neměnnost a absence příloh jsou známé odchylky; není to shoda |
| UX-13 | Úhrada 100 z 300 Kč → Částečně uhrazeno; další dialog předvyplní 200 Kč; doplatek → Uhrazeno | Existující backend odpovídá tomuto scénáři; v dialogu doplněn dosud chybějící souhrn již uhrazeno / zbývá |

Přímé vizuální podklady: [11 screenshotů](../research/screenshots/2026-10-11/README.md).
Pořadí opravy mezer: filtry a řazení seznamu → záložky detailu a přílohy →
ověřené opravy vystaveného dokladu. Poslední bod vyžaduje doménový kontrakt,
migrace/audit a scénáře R-04/R-06, nikoli pouze změnu disabled tlačítka.

## RE-01 — druhá implementační iterace

Podklady: E-021 screenshoty 04 (seznam), 05 (filtry), 06 (detail), 11 (úhrady).

- UX-11: pět hlavních pohledů (čtyři referenční a naše návrhy), kompaktní
  vyhledávání, samostatný stav ikonou s textovým názvem, popis, hlavičkové
  řazení, počet výsledků a volba velikosti stránky. Rozbalený filtr nad
  seznamem: typ data, rozsah dat, cena, zákazník a další stavy. Filtry jsou
  serverové, společné pro REST/MCP, uchované v URL.
- UX-12: Doklad / Přílohy (disabled) / Úhrady. Obsah dokladu začíná odběratelem,
  objednávkou a popisem; následují datumy a platební údaje, rozbalitelné údaje
  obou stran, položky a součet vpravo. Úhrady mají vlastní tabulku. Zvolená
  záložka přežije reload díky URL; stav v hlavičce reaguje na úhrady.
- Zachován monochromatický brand, Manrope, Lucide a sémantické barvy.
  Menší referenční ovládací prvky nejsou kopírovány na úkor klávesnice a dotyku.
- Úmyslné odchylky: neměnnost vystavené faktury, návrhy, žádné nefunkční
  hromadné akce, žádný daňový přepínač bez doménové funkce. Přílohy, datum
  úhrady ve filtru, štítky, opakování a export zůstávají nedostupné.

Nové snímky Open Faktury: invoice-list, invoice-filters, invoice-issued a
invoice-payments v tomto adresáři, varianty cs/en × desktop/mobile.

Ověření této iterace: 7 PostgreSQL testů invoice (včetně mezí data/ceny,
profilové izolace a shodného výsledku REST/MCP); 4 Playwright scénáře
CS/EN × desktop/mobile včetně cenového intervalu, odmítnutí obráceného rozsahu,
řazení, reloadu, vystavení, PDF, opravy/storna úhrad a viewer role. Axe bez
nálezů v kontrolovaných stavech. Typecheck, lokalizace, konvence, rustfmt,
clippy -D warnings a build prošly. Build má již známé upozornění na chunk
nad 500 kB. Testovací částky jsou syntetické; nejde o další referenční
potvrzení numerických pravidel iDokladu. Lokální API /api/ready vrací ready.

## RE-01 — sjednocení editoru s detailem

Reference E-021, screenshoty 01/02 a detail 06: editor nového i uloženého
návrhu začíná odběratelem, objednávkou a popisem, pokračuje datumy a platebními
údaji ve dvou sloupcích, dalšími nastaveními, položkami a pravým souhrnem.
Stejné obsahové Tabs jako detail; přílohy jsou nedostupné, úhrady zpřístupní
vystavení. Uložený návrh má nadpis Rozpracovaná faktura / Invoice draft bez
interního ID. Návrhům se nadále nepřiděluje konečné číslo před vystavením.

InvoiceSummary je společný pro editor a detail; zobrazí pouze aktuální
serverový výpočet (při čekání či chybě nepředstírá platný součet). Položky
editoru mají na desktopu hlavičku a kompaktní řádky cena / množství / jednotka /
částka / odstranění; na mobilu vlastní labely a skupiny. Součty řádků jsou
serverové, frontend je nepočítá. Detail používá stejné pořadí sloupců.
Slevy a DPH zůstávají dostupné, nulová sleva v detailu nezabírá sloupec.
Dodavatel a jazyk dokladu jsou v Dalším nastavení; chyby dodavatele jeho
rozbalovací sekce automaticky otevřou.

Záměrné odchylky: ponechán explicitní životní cyklus Uložit návrh → Vystavit,
ARES a uložení kontaktu uvnitř editoru; žádné nefunkční přepínače daní,
šablony nebo předběžná rezervace čísla. Databázové a účetní operace se nemění.

Ověření editoru: 4 E2E scénáře CS/EN × desktop/mobile s reálným backendem
v izolované DB prošly včetně axe, konfliktu uložení, přepočtu, kopie,
vystavení, PDF a úhrad. Finální dorovnání desktopového selectu ověřuje navíc
geometrická kontrola proti popisu položky. Build, TypeScript, formátování,
kontrola lokalizace a konvencí prošly. Backend se v této iteraci neměnil.
Snímky invoice-create-* a invoice-editor-* dokumentují nový a uložený návrh;
invoice-issued-* stejný doklad po vystavení. Známé build upozornění na velikost
sdíleného chunku zůstává.

## RE-01 — adresář a detail/editor odběratele

Podklad E-022, přímé screenshoty iDokladu 12–14. Seznam Open Faktury nyní používá
stejnou hustotu řádků, záložky datasetu, kompaktní hledání a patičku jako faktury.
Sloupce odpovídají referenci: název, IČO, DIČ, kontaktní osoba, e-mail, telefon,
akce detailu. Názvy jsou odkazy; dlouhý název má title a zkrácení v tabulce,
celý text je v detailu. Stránkování a velikost 10/25/50/100 skutečně používají
backend a uchovávají hledání i archiv v URL. Řazení zůstává serverové podle názvu.

Detail je čitelný přehled; Upravit kontakt otevře formulář. Oba mají stejné dvě
obsahové záložky s klávesnicovou navigací: Základní údaje a Dodací adresy.
Identita/adresa nahoře, kontaktní informace/osoby na šedém podkladu, bankovní
údaje, předvolby a poznámka v rozbalovacích podrobnostech. Chyby platebních údajů
podrobnosti otevřou. Zrušení rozepsané úpravy žádá potvrzení; uložení vrací detail.
Odkaz Faktury odběratele otevře skutečný seznam faktur s filtrem kontaktu.

Záměrné odchylky: náš model zatím nemá dodavatele, proto Aktivní/Archivované;
archivace s obnovou místo mazání; více kontaktních osob a dodacích adres;
ARES podle IČO; žádné nefunkční hromadné akce či export. Související doklady jsou
odkaz na filtrované faktury, nikoli kopie tabulky. Monochromatické tokeny zachovány.
Databázové schéma ani serverové operace se nemění; klient využívá již existující
parametr page_size. České i anglické texty zahrnují nové akce a potvrzení.

Ověření: 4 finální E2E průchody CS/EN × desktop/mobile s reálným backendem a
izolovanou PostgreSQL prošly. Pokrývají ARES merge/retry, vytvoření, čtecí detail,
klávesnicové záložky, uložení, souběžný konflikt, zahození změn, odkaz na faktury,
archiv/obnovu, hledání, skutečné stránkování 12 kontaktů (10 + 2), reload URL,
profilovou izolaci a viewer roli. Axe bez nálezů v detailu, editoru i seznamu;
mobil nemá horizontální přetečení dokumentu (tabulka má vlastní posun).
Typecheck, kontrola CS/EN a konvencí i produkční build prošly. Zůstává známé
upozornění na sdílený chunk nad 500 kB. Snímky contacts-list-*, contacts-detail-*
a contacts-editor-* v docs/design obsahují výhradně syntetická testovací data.

## Následná vizuální kontrola — zbývající rozdíly

Porovnání uložených referencí E-022 (12–14) s contacts-list/detail/editor-cs-desktop
 a invoice-list/editor-cs-desktop, 11. 10. 2026. Jde o vizuální posouzení existujících
snímků, ne nový funkční průchod. Reference má viewport šířky 1680 px, naše snímky
1280 px; nelze z celkové výšky stránky odvozovat přesný poměr hustoty.

1. **Priorita 1 — editor je příliš vysoký.** Labely nad každým polem, samostatná
   řádka DIČ, trvalá víceřádková nápověda ARES, velká textarea pro příjemce a
   mezery u fieldsetu osoby dohromady vytvářejí dlouhý formulář. Reference má
   kompaktní řádky label/hodnota a těsnější vazbu identity a adresy. Upravit
   desktopovou mřížku, nápovědu přesunout do tooltipu, příjemce řešit kompaktním
   vícenásobným vstupem. Mobilní labely a dotykové cíle zachovat.
2. **Priorita 1 — odlišná geometrie detailu a editoru.** Detail používá řádky
   label/hodnota, editor labely nad poli; při přepnutí se údaje výrazně posouvají.
   Sjednotit osy a pořadí polí. Také faktura používá pro horní sekci omezenou
   šířku a jiné odsazení než kontakt; přijaté tokeny samy nezajišťují shodný layout.
3. **Priorita 2 — hierarchie akcí detailu.** Tři velká tlačítka vpravo dávají
   archivaci zbytečnou váhu. Reference má Upravit u názvu a vedlejší ikonové akce.
   Upravit ponechat hlavní; archivaci dát do sekundárního menu. Odkaz na faktury
   je funkční přijatá odchylka, ale vizuálně konkuruje úpravě.
4. **Priorita 2 — seznam je stále řidší než reference.** Řádky a panel hledání
   zabírají více výšky; chip hledání má vlastní ohraničený pás. Sjednotit toolbar
   s fakturami (tam jsou chipy v jednom bloku), šířku hledání a kompaktní desktopové
   ovládání. Akce oka duplikuje odkaz názvu; přímá úprava by zkrátila běžný postup.
5. **Priorita 2 — patička editoru.** Kontakty mají uložit/zrušit v oddělené kartě,
   faktura i reference uvnitř hlavního panelu. Použít společné zakončení formuláře.
6. **Priorita 3 — rozbalovací podrobnosti.** Funkčně správné, ale nativní marker
   summary působí méně dotaženě než společný ovladač s chevronem a jasným stavem.

Co odpovídá: sloupce adresáře, oddělený čtecí/editovací režim, záložky základních
údajů a adres, šedá kontaktní sekce, skryté bankovní podrobnosti, monochromatické
barvy a lokalizace. Archivace, více osob a vynechané neimplementované akce jsou
záměrné produktové odchylky, nikoli vizuální vady. Předchozí funkční testy nedokládají
vizuální shodu; označení celé vizuální části RE-01 za dokončenou bylo předčasné.

## Opravy po vizuálním auditu — kompaktní kontakty

Následující revize opravuje výše uvedené nálezy, nikoli pouze barevnost:

- Desktopový editor i čtecí detail používají shodnou šířku labelu z tokenu
  form-label, shodné pořadí adresních polí a řádky s výškou control-sm.
  Geometrický E2E test porovnává počátek hlavního panelu a osu hodnot před/po
  přepnutí na úpravu. Na mobilu zůstávají labely nad ovládacími prvky.
- Nápověda ARES je focusovatelná ikona s tooltipem, stav načtení zůstává viditelný.
  Příjemci jsou jeden vstup oddělený čárkami/středníky; server stále dostává pole
  jednotlivých adres. Přidání osoby je kompaktní akce u nadpisu.
- Upravit je vedle názvu, archivace/obnova v přístupném menu Akce. Tužka v seznamu
  otevírá editor přímo; název nadále čtecí detail. Viewer a archiv nejsou editovatelné.
- Uložit/Zrušit zakončují stejný panel i na záložce adres. Podrobnosti mají chevron
  se stavem otevření a podporou reduced motion.
- Hledání a jeho chip tvoří jeden toolbar. Desktopové řádky tabulek kontaktů
  i faktur mají kompaktní výšku podle control tokenu; mobilní velikosti zachovány.

Opětovné porovnání s E-022, snímky 12–14: shoduje se vzor label/hodnota,
oddělený detail/editor, akce Upravit u názvu, kontaktní sekce na šedém podkladu,
rozbalovací údaje a integrované zakončení editoru. Na stejném lokálním scénáři
při šířce 1280 px klesla výška plného snímku editoru z 1240 na 908 px (přibližně
27 %). Toto je srovnání našich revizí, nikoli poměr vůči jinak široké referenci.
Reference stále používá odlišné zaoblení, menší ovladače a jiné rozdělení identity
nad adresou; ponecháváme monochromatické tokeny a společnou mřížku Open Faktury.
Více osob, archiv a odkaz na filtrované faktury zůstávají záměrnými odchylkami.

Ověření této opravy: 8 E2E scénářů kontaktů a faktur v CS/EN na desktopu/mobilu
prošlo nad izolovanými databázemi. Kontaktní scénář navíc kontroluje geometrické
zarovnání detail/editor, přímou editaci z řádku a archivaci přes menu. Axe bez
nálezů v kontrolovaných stavech. Typecheck, locale/conventions, formátování a
produkční build prošly; známé upozornění na velikost sdíleného chunku zůstává.
Mobilní snímky byly prohlédnuty pro čitelnost a přetečení; mobilní referenční
snímek iDokladu nemáme, proto zde netvrdíme přímou vizuální shodu.
Finální sjednocení šířky hledání a zachování sémantické barvy chyby ARES ověřeno
navíc dvěma desktopovými CS průchody kontaktů a faktur; oba prošly.

## RE-02 — společný rám a hustota rozhraní (11. 10. 2026)

Podklady E-021 až E-023. Uživatel výslovně potvrdil zachování monochromatického
brandu. Toto je další implementační průchod, nikoli potvrzení úplné shody 1:1.
Aktuální rozsah a mezery: [matice shody](idoklad-parity.md).

- Centrální tokeny: rám 16 px, sidebar 256 px, zaoblení pracovního prostoru
  24 px; menší nadpisy, oválné akce a desktopové vstupy. Dotykové rozměry
  zůstávají větší. Stavové barvy a font Manrope/Lucide zachovány.
- Adresář a nastavení tvoří skupiny navigace; aktivní skupina se otevírá podle
  cesty, aktivní položka má tmavé pozadí. Sidebar je pevný a samostatně posuvný.
- Seznamy: vystouplé stavové záložky, kompaktní hlavičky a řádky, společné
  stránkování (první/předchozí/aktuální/další/poslední), vodorovná volba počtu.
  Hledání, oprávnění i filtry v URL zůstávají funkční.
- Profil: čtení a úprava mají stejné záložky a rám. Detail už není samostatná
  nesouvisející karta se všemi údaji schovanými pod jedním rozbalením.
- Faktura: datumy a platební pole mají na desktopu popisky vedle vstupů,
  objednávka/popisek jsou řádky, popis má kompaktní výšku. Čtecí detail používá
  stejný token šířky popisků.
- Kontakt: stabilní výška záhlaví odstranila posun obsahu 3,2 px při přepnutí
  detail/editor. Původní geometrická kontrola zůstala zachována.

Při prvním běhu Axe zachytil konflikt aktivní barvy navigace. Opraven odstraněním
soupeřící utility, pravidla kontrastu nebyla vypnuta. Následně prošlo 20 scénářů
kontaktů, faktur a sdíleného design systému (CS/EN, desktop/mobile); nastavení
profilu a účtu prošlo ve 4 scénářích. Finální zhuštění faktury ověřeno samostatně
níže. Snímky `contacts-*`, `invoice-*`, `profile-*` jsou syntetické výstupy E2E.

Rozdíl proti referenci zůstává mimo jiné v horní liště s přepínačem profilů,
rozložení některých polí a dostupných akcích. Nepotvrzené funkce nejsou nahrazeny
ukázkovými daty nebo aktivně vypadajícími nefunkčními tlačítky. Nativní Tauri
ani mobilní iDoklad nebyly tímto průchodem ověřeny.

Finální zhuštění editoru: `pnpm exec playwright test tests/e2e/invoices.spec.ts
--workers=1` — 4/4 prošly (CS/EN, desktop/mobile), včetně vystavení, PDF,
úhrad a přístupnosti. Typecheck, kontrola překladů, konvence, formátování
a produkční build prošly. Build nadále hlásí známý sdílený chunk nad 500 kB.
