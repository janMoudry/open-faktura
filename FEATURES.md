# Funkční soupis iDokladu a návrh Open Faktury

Datum průzkumu: **10. 10. 2026**. Referencí je český iDoklad a jeho veřejná dokumentace.

Tento dokument je základ pro funkční specifikaci plné otevřené alternativy. Popisuje veřejně doložené schopnosti produktu, oblasti k ověření a návrhy pro Open Fakturu. Nejde o dokončený audit aplikace: přesné chování obrazovek, výpočtů, oprávnění a hraničních případů vyžaduje další průchod aplikací a API.

## 1. Vydané faktury

- Vytvoření, úprava a evidence faktur.
- Výběr odběratele z adresáře nebo založení kontaktu při fakturaci.
- Odlišná fakturační a dodací adresa.
- Číslo objednávky, popis faktury a interní poznámka.
- Datum vystavení, splatnost a DUZP.
- Výchozí splatnost podle firmy nebo konkrétního odběratele.
- Více číselných řad a vlastní formát číslování.
- Kontrola duplicitních čísel a řešení souběžného vystavování více uživateli.
- Variabilní a konstantní symbol.
- Způsob úhrady a výběr bankovního účtu.
- IBAN a SWIFT/BIC.
- Fakturace v cizích měnách, načtení kurzu a ruční změna kurzu.
- Položky s množstvím, jednotkou, cenou a sazbou DPH.
- Zadávání ceny s DPH nebo bez DPH.
- Vkládání položek z ceníku a podpora čtečky čárových kódů.
- Uložení položky faktury zpět do ceníku.
- Vlastní text před položkami a za nimi.
- Volba jazyka a vzhledu dokladu.

Zdroj: [Karta faktury](https://www.idoklad.cz/podpora/karta-faktury).

## 2. Práce s doklady a jejich výstupy

- Vyhledávání podle textu a čísla.
- Kombinované filtrování.
- Přehled všech, uhrazených, neuhrazených a prošlých faktur.
- Rozlišení částečně uhrazených a přeplacených dokladů.
- Štítky a přílohy.
- Zobrazení vazeb na související doklady.
- Součty vybraných nebo vyfiltrovaných dokladů podle sazeb DPH.
- Jednotlivý a hromadný tisk.
- Náhled dokladu, PDF a RTF.
- Dodací list.
- Potvrzení úhrady a pokladní doklad.
- QR Platba, QR Faktura a společný QR Platba+F za příslušných podmínek.
- Export seznamů do XLSX a CSV s výběrem sloupců.
- Příznak exportu do účetního systému.
- Rozpoznání změny dokladu po exportu.
- Volitelné omezení úprav exportovaných dokladů.
- Koš a obnovení smazaných dokladů.

Zdroj: [Faktury vydané](https://www.idoklad.cz/podpora/faktury-vydane).

## 3. Zálohy, vyúčtování a opravné doklady

- Zálohové/proforma faktury.
- Evidence jejich úhrad.
- Přehled uhrazených, dosud nevyúčtovaných záloh.
- Vytvoření konečné faktury ze zálohy.
- Vyúčtování více záloh jednou fakturou při splnění podmínek.
- Odpočty záloh a přidání dalších fakturovaných položek.
- Přenos souvisejících daňových dokladů do vyúčtování.
- Omezení editace a mazání podle navázaných dokladů.
- Řešení vyúčtování při změně sazeb DPH.
- Dobropis vytvořený z původní faktury se zdůvodněním opravy.
- Daňový doklad k přijaté platbě.
- Vystavení daňového dokladu přímo při zaznamenání úhrady.
- Odeslání a tisk daňového dokladu.
- Upozornění na zálohy, ke kterým chybí vyúčtování nebo daňový doklad.

Zdroje: [Zálohové faktury](https://www.idoklad.cz/podpora/zalohove-faktury), [Daňové doklady k platbě](https://www.idoklad.cz/podpora/danove-doklady-k-platbe), [Dobropis z faktury](https://www.idoklad.cz/podpora/faktury-vydane).

## 4. Cenové nabídky

- Vytvoření nabídky s odběratelem, položkami a cenami.
- Použití položek z ceníku.
- Odeslání nabídky zákazníkovi.
- Štítky a stavy nabídky.
- Filtrování podle stavu a dalších parametrů.
- Převod nabídky na běžnou nebo zálohovou fakturu.
- Přenos odběratele, položek, platebních údajů a doprovodných textů.

Zdroj: [Cenové nabídky](https://www.idoklad.cz/podpora/cenove-nabidky).

## 5. Šablony a pravidelná fakturace

- Šablony běžných faktur, zálohových faktur a cenových nabídek.
- Přednastavení odběratele, položek, měny a dalších údajů.
- Šablony pro zahraniční fakturaci a přenesenou daňovou povinnost.
- Pevný variabilní symbol a pevný měnový kurz.
- Vzor pravidelné faktury vytvořený samostatně nebo z existující faktury.
- Automatické generování běžných i zálohových faktur.
- Nastavení opakování.
- Automatické doplnění čísla, data a aktuálního měnového kurzu.
- Automatické odesílání vytvořených dokladů.

Zdroje: [Šablony](https://www.idoklad.cz/podpora/sablony), [Pravidelné faktury](https://www.idoklad.cz/podpora/pravidelne-faktury), [Prodej](https://www.idoklad.cz/podpora/prodej).

## 6. Přijaté faktury a účtenky

- Evidence přijatých faktur a dodavatelů.
- Vlastní číslování přijatých dokladů.
- Položky, částky a údaje o DPH.
- Evidence úhrad a závazků.
- Přehled uhrazených, neuhrazených a prošlých dokladů.
- Vyhledávání, filtry a součty.
- Přílohy původních dokumentů.
- Tisk a odeslání přijatých dokladů e-mailem.
- Samostatná evidence účtenek z drobných nákupů.
- Zahrnutí přijatých faktur a účtenek do nákladových přehledů.

Zdroje: [Přijaté faktury](https://www.idoklad.cz/podpora/nakup-faktury-prijate), [Účtenky](https://www.idoklad.cz/podpora/nakup-uctenky).

## 7. Inbox a vytěžování dokumentů

- Vlastní příchozí e-mailová adresa pro každou firemní agendu.
- Příjem dokumentů z e-mailových příloh.
- Ruční a hromadné nahrávání souborů.
- Nahrání přetažením souboru.
- Rozpoznání a načtení údajů z ISDOC a QR kódů.
- AI vytěžování faktur a účtenek.
- Náhled originálu vedle zpracovávaných údajů.
- Kontrola a ruční oprava načtených hodnot.
- Založení nového dodavatele z dokumentu.
- Upozornění na rozdíly oproti existujícímu kontaktu.
- Zpracování jako přijatá faktura, účtenka nebo příloha existujícího dokladu.
- Evidence zpracovaných a nezpracovaných souborů.
- Připojení a odpojení dokumentu.
- Notifikace o nově přijatých dokumentech.
- Předání původních příloh účetnímu.

Podle nápovědy je Inbox od tarifu Oblíbený; AI vytěžování se účtuje zvlášť.

Zdroj: [Inbox](https://www.idoklad.cz/podpora/inbox).

## 8. Adresář a ověřování obchodních partnerů

- Odběratelé i dodavatelé.
- Firemní a osobní identifikační údaje.
- Doplnění údajů z ARES podle názvu nebo IČO.
- Kontrola spolehlivosti plátce DPH v ČR a na Slovensku.
- Kontrola platnosti DIČ partnera v EU.
- Kontaktní osoba, telefon a e-mail.
- Více e-mailových příjemců.
- Bankovní účet partnera.
- Individuální splatnost, sleva a nastavení upomínek.
- Více dodacích adres a výchozí dodací adresa.
- Přehled souvisejících vydaných a přijatých faktur.
- Vytvoření faktury přímo z kontaktu.
- Export kontaktů do XLSX a CSV.

Zdroj: [Adresář](https://www.idoklad.cz/podpora/adresar-kontakty).

## 9. Ceník a sklad

- Skladové i neskladové položky, včetně služeb.
- Název, katalogové označení, prodejní cena, množství a měrná jednotka.
- Opakované použití položek na dokladech.
- Našeptávání položek při fakturaci.
- Úprava ceny, množství a popisu na konkrétní faktuře.
- Import ceníku z podporovaných systémů Money.
- Evidence množství na skladě.
- Ruční naskladnění a vyskladnění.
- Automatické skladové pohyby z vydaných a přijatých faktur.
- Historie skladových pohybů.
- Přehled prodeje položek za zvolené období.

Zdroje: [Ceník a sklad](https://www.idoklad.cz/podpora/cenik-sklad), [Přehled funkcí](https://www.idoklad.cz/cenik).

## 10. Úhrady, banka a pokladna

- Více částečných úhrad jedné faktury.
- Datum, částka a způsob každé úhrady.
- Seznam úhrad a platby konkrétního dokladu.
- Evidence bankovních účtů, jejich měn a počátečních stavů.
- Výchozí bankovní účet; dokumentace aktuálně uvádí nejvýše pět účtů.
- Příjmové a výdajové bankovní doklady.
- Příjmové a výdajové pokladní doklady.
- Stavy bankovních účtů a pokladen.
- Párování finančních dokladů s fakturami.
- Automatické vytváření pokladního dokladu při hotovostní úhradě.
- Prodejky a jejich tisk, filtrování a statistiky.
- Automatické párování bankovních pohybů.
- Ruční párování nespárovaných pohybů.
- Rozlišení plné úhrady, částečné úhrady a přeplatku.
- Volitelné haléřové vyrovnání.
- Přehled výsledků zpracování bankovních notifikací.

Veřejně popsané automatické párování používá bankovní e-mailové notifikace. Primárně páruje podle variabilního symbolu a při více shodách zohledňuje částku. Nelze z toho odvozovat přímé napojení přes bankovní API.

Zdroje: [Úhrady](https://www.idoklad.cz/podpora/prodej-uhrady), [Banka](https://www.idoklad.cz/podpora/finance-banka), [Pokladna](https://www.idoklad.cz/podpora/finance-pokladna), [Nastavení a párování banky](https://www.idoklad.cz/podpora/nastaveni-banka).

## 11. E-maily a komunikace

- Jednotlivé a hromadné odesílání dokladů.
- Zákazník, účetní, vlastní kopie a další příjemci.
- PDF v příloze nebo odkaz na PDF v prohlížeči.
- Záznam otevření odkazu na doklad.
- Zobrazení informace o úhradě na sdíleném dokladu.
- Převzetí faktury do účtu příjemce, pokud také používá iDoklad.
- E-mailové šablony podle typu dokladu.
- Proměnné pro číslo dokladu, částku, splatnost a další údaje.
- Ruční a automatické upomínky.
- Evidence počtu a data poslední upomínky.
- Ruční a automatické poděkování za úhradu.
- Historie odeslaných e-mailů.
- Odesílání přes službu iDokladu, Gmail s OAuth nebo vlastní SMTP.

Zdroje: [E-maily a komunikace](https://www.idoklad.cz/podpora/nastaveni-e-maily-a-komunikace), [Historie otevření dokladů](https://www.idoklad.cz/podpora/rychle-tipy-nastaveni).

## 12. Přehledy a dashboard

- Fakturace za měsíc, kvartál nebo rok.
- Částky s DPH i bez DPH.
- Kumulativní zobrazení.
- Srovnání s předchozím rokem a rozdíly v čase.
- Bilance prodeje a nákupu.
- Uhrazené a neuhrazené částky.
- Pohledávky a závazky po splatnosti podle délky prodlení.
- Největší odběratelé a dlužníci.
- Přehled DPH.
- Sledování obratu a upozornění na přiblížení k limitu registrace DPH.
- Bankovní notifikace a výsledky párování.
- Výběr, pořadí a uložení vlastní varianty widgetů.
- Rychlé vytvoření dokladu, kontaktu nebo položky ceníku.

Zdroj: [Dashboard a přehledy](https://www.idoklad.cz/podpora/domovska-stranka).

## 13. Daňové funkce

- Režim plátce, neplátce a identifikované osoby.
- Nastavení zdaňovacího období.
- Členění DPH na dokladech a položkách.
- Kódy plnění pro přenesenou daňovou povinnost.
- Generování podkladů pro přiznání k DPH.
- Generování podkladů pro kontrolní hlášení.
- Generování podkladů pro souhrnné hlášení.
- Výstup pro načtení na daňový portál.
- Podklady pro přiznání k dani z příjmů fyzických osob.
- Volba zahrnutí konkrétního dokladu do podkladů pro daň z příjmů.

Generování podkladů je odlišné od samotného podání. Dokumentace popisuje následné načtení souboru na daňový portál a uvádí i případy, které automaticky nepokrývá. Přesnou podporu je potřeba rozepsat po jednotlivých scénářích. Tento soupis popisuje schopnosti produktu, nikoli pravidla pro daňové posouzení konkrétní transakce.

Zdroje: [DPH a hlášení](https://www.idoklad.cz/podpora/finance-dph), [Doklady pro finanční úřad](https://www.idoklad.cz/vlastnosti/doklady-pro-financni-urad).

## 14. Firmy, uživatelé a nastavení

- Více firem pod jedním účtem.
- Firemní identifikační a účetní údaje.
- Nastavení legislativy a typu subjektu.
- Logo, obrázek podpisu a razítka.
- Vlastní vzhled a barvy dokumentů.
- Pozvánky dalším uživatelům.
- Odebrání přístupu.
- Role majitele a uživatele.
- Převod vlastnictví firmy a opuštění firmy.
- Změna přihlašovacího e-mailu a hesla.
- Dvoufázové ověření přes autentizační aplikaci.
- Osobní údaje uživatele.
- Tmavý režim a nastavení notifikací.
- L10N-01: lokalizace rozhraní do češtiny (`cs`, výchozí) a angličtiny (`en`) od prvních produktových funkcí; přepínač i před přihlášením, zapamatování volby a formátování dat, čísel a částek podle locale. Jazyk dokladu se nastavuje nezávisle.
- Počet desetinných míst.
- Správa štítků.
- Koš.
- Anonymizace údajů kontaktu.
- Zrušení firemní agendy.

Zdroje: [Firma a uživatelé](https://www.idoklad.cz/podpora/nastaveni-firma), [Uživatelský účet](https://www.idoklad.cz/podpora/nastaveni-uzivatel), [Nastavení aplikace](https://www.idoklad.cz/podpora/nastaveni-aplikace-idoklad).

## 15. Import, export, integrace a API

- Import vydaných faktur a kontaktů z Pohody a SuperFaktury.
- Při importu párování kontaktů podle IČO a založení chybějících kontaktů.
- Export pro Pohodu.
- Propojení s Money S3, Money S4/S5 a Variem.
- Zpětná aktualizace úhrad nebo obousměrná komunikace podle konkrétní integrace.
- Katalog externích doplňků.
- Napojení e-shopů a dalších aplikací.
- REST API a vývojářský portál.
- Generování Client ID a Client Secret.
- Přehled připojených aplikací.
- Evidence spotřeby API.
- Oficiální .NET SDK.

Integrace mají různý rozsah; napojení na účetnictví potřebuje samostatnou specifikaci pro každý systém.

Zdroje: [Účetní systémy](https://www.idoklad.cz/podpora/komunikace-s-ucetnimi-systemy), [Import a nastavení API](https://www.idoklad.cz/podpora/nastaveni-aplikace-idoklad), [API a doplňky](https://www.idoklad.cz/api-a-doplnky-k-idokladu), [Oficiální SDK](https://github.com/Solitea/IdokladSdk), [API dokumentace](https://api.idoklad.cz/Help/v3/cs/).

## 16. Mobilní aplikace a služby kolem produktu

- Aplikace pro Android a iOS.
- Vystavení faktury, zálohy, nabídky a dobropisu.
- Odesílání dokladů.
- Načtení přijaté faktury přes QR.
- Přehled vydaných a přijatých faktur.
- Evidence úhrad.
- Vyhledávání a úprava kontaktů.
- Synchronizace s webovou aplikací.
- Nápověda, zákaznická podpora a vzdělávací miniakademie.
- Automatické zálohování provozované služby.

Zdroje: [Mobilní aplikace](https://www.idoklad.cz/aplikace-idoklad), [Nápověda](https://www.idoklad.cz/podpora), [Ceník a služby](https://www.idoklad.cz/cenik).

## 17. Tarify jako kontrola rozsahu

Pro Open Fakturu je cílem sjednocení funkčního rozsahu všech tarifů.

| Tarif iDokladu | Hlavní omezení a rozšíření |
| --- | --- |
| Zdarma | 5 kontaktů, 1 uživatel, základní fakturace, přehledy a mobil |
| Základní | Neomezený adresář, 2 uživatelé, přílohy, štítky, ceník, sklad a účetní integrace |
| Oblíbený | 5 uživatelů, banka a automatizace, pokladní funkce, daňové výstupy a 7 500 API požadavků měsíčně |
| Prémiový | 9 uživatelů, 75 000 API požadavků měsíčně a přednostní podpora |

Zdroj: [Aktuální ceník](https://www.idoklad.cz/cenik). Přesné přiřazení každé funkce je potřeba ověřit v aplikaci: textový výstup ceníku nezachovává všechny značky dostupnosti. Limity referenčního produktu nejsou navrženými limity Open Faktury.

## 18. Co ověřit při reverse-engineeringu

| Oblast | Co potřebujeme zjistit |
| --- | --- |
| Výpočty | Zaokrouhlování položek a součtů, slevy, kurzové přepočty a historické sazby |
| Životní cyklus dokladů | Povolené úpravy, mazání, dobropisy, zálohy a jejich vazby |
| Automatizace | Intervaly, ukončení opakování, opakované pokusy a chování při chybě |
| Úhrady | Zrušení úhrady, přeplatky, vratky, zápočty a nejednoznačné párování |
| Daně | Podporované situace, roční verze formulářů a výjimky |
| Oprávnění | Přesná práva uživatele, majitele a připojených aplikací |
| API | Úplný katalog operací, polí, filtrů a rozdíly proti webu |
| Integrace | Formáty, směry přenosu, deduplikace a řešení konfliktů |
| Mobil | Rozdíly proti webu a případná práce bez připojení |
| Historické funkce | Aktuální stav ProfiÚčtenky a starých zmínek o EET |

[Ceník](https://www.idoklad.cz/cenik) stále obsahuje některé historické formulace. Každou takovou položku je potřeba před zařazením do implementace ověřit.

Další průzkum bude vycházet z pozorování aplikace, dokumentovaného API a exportů. Výsledkem bude vlastní implementace a vlastní uživatelské rozhraní.

## 19. Návrh kompletního open-source produktu

Následující body jsou návrhem pro Open Fakturu, nikoli tvrzením o funkcích iDokladu.

- Otevřený frontend, backend, API, MCP, migrace databáze i všechny vlastní konektory.
- **SETUP-001 až SETUP-012: Instalace přes Docker Compose a průvodce prvním spuštěním.**
- Oddělená konfigurace instalace, firmy a uživatele; jednorázové chráněné založení správce bez výchozího účtu a veřejné registrace.
- Obnovitelný průvodce založením firmy: identita, účetní profil, bankovní údaje a číselná řada; následně prázdná agenda bez demo dat.
- SMTP, import a externí integrace jsou volitelné; obnova přístupu je možná i bez SMTP.
- Desktopový klient umožní připojení k vlastnímu serveru; aktuální instalátor sám neinstaluje backend ani PostgreSQL.
- **PERSONAL-01 (plán):** osobní desktop „Používat na tomto počítači“ se spravovaným lokálním Rust/PostgreSQL runtime a bootstrapem bez ručního kopírování klíče; nejprve ověřit balení, přístup přes OS profil a životní cyklus dat.
- [Specifikace instalace a onboardingu](docs/specs/installation-onboarding.md), [dodávky P00–P20 a podmínky dokončení](docs/IMPLEMENTATION_PLAN.md).
- Dokumentovaný provoz na vlastním serveru.
- Vlastní SMTP a příjem dokumentů do inboxu.
- Kompletní export databáze, dokumentů a příloh.
- Zálohování, obnova a bezpečné aktualizace.
- **MIG-01: Jednoduchá migrace z iDokladu, Faktury online (fakturaonline.cz) a dalších služeb.** Průvodce přenosem kontaktů a dokladů, později úhrad, vazeb a příloh podle dostupnosti zdrojových dat.
- Předpřipravené importní adaptéry a obecný CSV/XLSX import s mapováním sloupců; podporu konkrétních exportů a API nejprve ověřit.
- Náhled před zápisem, řešení duplicit a konfliktů, pokračování po přerušení a závěrečná kontrola počtů, částek podle měn a vazeb. Migrace sama nespouští odesílání ani automatizace.
- [Rozsah, zařazení a akceptační scénáře migrace MIG-01](docs/IMPLEMENTATION_PLAN.md#mig-01--jednoduchý-přechod-z-jiné-fakturační-služby).
- Nastavitelná oprávnění a audit změn.
- Dokumentované API a události pro integrace.
- Vytěžování dokumentů s volitelným poskytovatelem, případně lokálním modelem.
- Základní fakturace fungující bez placené služby provozovatele projektu.
- Případný spravovaný hosting stejné otevřené aplikace.

Rozsah může vznikat postupně, ale všechny moduly budou součástí otevřené verze. Backend a doménové jádro Open Faktury budou v Rustu, databází bude PostgreSQL (rozhodnutí 10. 10. 2026). Frontend používá React a Tailwind s atomic designem; projekt bude monorepo s tradiční strukturou pages/components/utils a desktopovými instalátory v GitHub Releases. Desktopový runtime je Tauri (potvrzeno 10. 10. 2026). Konkrétní doplňkové knihovny a licence zatím nejsou uzavřené.

## 20. MCP

### Současný stav u iDokladu

Existuje komunitní [iDoklad MCP server](https://github.com/veceravojtech/idoklad-mcp). Jeho README při průzkumu uvádělo 12 nástrojů pro faktury, kontakty, ceník a obecné volání API. Repozitář dokumentuje i režim pouze pro čtení. Funkčnost serveru nebyla v tomto průzkumu spuštěním ověřena.

Oficiální MCP od iDokladu nebylo ve zkoumaných zdrojích dohledáno. To není důkaz jeho neexistence.

### Návrh MCP pro Open Fakturu

MCP bude používat stejnou aplikační logiku jako web a API. Každá operace bude mít stejné validace, výpočty a oprávnění.

| Oblast | Navržené schopnosti |
| --- | --- |
| Hledání | Najít doklady, kontakty, položky, úhrady a přílohy |
| Fakturace | Připravit návrh, upravit jej, vystavit doklad a získat PDF |
| Nabídky a zálohy | Vytvořit nabídku, převést na fakturu a vyúčtovat zálohy |
| Nákup | Nahrát dokument, vytěžit údaje a založit přijatý doklad |
| Platby | Zaznamenat úhradu, navrhnout párování a zobrazit nesrovnalosti |
| Komunikace | Připravit a odeslat fakturu, upomínku nebo poděkování |
| Automatizace | Spravovat šablony a pravidelnou fakturaci |
| Sklad | Číst zásoby a zaznamenávat pohyby |
| Přehledy | Pohledávky, závazky, tržby, náklady a podklady pro účetní |
| Administrace | Pracovat v konkrétní firmě podle udělených oprávnění |

Příklad cílového použití:

> Připrav fakturu firmě X za říjnový hosting podle zářijové faktury, se splatností 14 dní.

MCP vrátí návrh a náhled. Vystavení a odeslání budou samostatné operace. Součástí návrhu je režim pouze pro čtení a ochrana proti duplicitnímu vystavení při opakování požadavku.

## 21. Převod soupisu do implementační specifikace

Ke každé funkci bude potřeba doplnit:

- Jednoznačný identifikátor a stav ověření.
- Vstupy a výstupy.
- Stavy a povolené přechody.
- Výpočty, validace a oprávnění.
- Vazby na další moduly.
- Hraniční případy a chybové stavy.
- Ukázkový scénář a ověřovací test.
- Referenční zdroj nebo záznam pozorování.

Tím vznikne měřitelná definice toho, kdy Open Faktura pokrývá funkční rozsah referenčního produktu.

### Implementace P03 (10. 10. 2026)

Firmy, obnovitelný průvodce, základní nastavení a členství jsou implementované
v CS/EN přes Rust/PostgreSQL. Revize, audit a historie profilu DPH; read-only
MCP stdio se stejnými právy. [Rozsah a hranice](docs/COMPANIES.md).
Pozvánky/správa členů, vystavování dokladů a vzdálené MCP OAuth navazují později.
