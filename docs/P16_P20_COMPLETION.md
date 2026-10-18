# Aktuální stav P16–P20

Aktualizace: 18. 10. 2026. Aktivní rozsah zahrnuje všechny zbývající části
P16–P20, backend, PostgreSQL, REST/MCP a češtinu i angličtinu. Tento soubor
popisuje **současný stav**, nikoli historický stav jednotlivých dodávek.
Historii zachovává [implementační plán](IMPLEMENTATION_PLAN.md). Přesný rozsah
je ve specifikacích a [referenčním soupisu](research/FEATURES_REFERENCE.md).

## P16 — banky, pokladna a prodejky

Základ banky není rozpracovaný znovu. Implementované jsou účty a počáteční
salda, importy, ruční/dělené/dávkové párování, připojení existujících úhrad,
zálohy a vratky, různé měny účtů, zahraniční IBAN a uložené kurzové rozdíly.
Vlastní napojení podporuje Fio a instalačně schválené brány, test spojení,
ruční i plánovanou synchronizaci. Automatické párování má samostatný opt-in;
zálohy/vratky vyžadují další výslovnou volbu.

Pokladna má ruční i automatické doklady, samostatná daňová plnění, PDF a částku
slovy. Prodejky mají kombinované platby, terminálové snapshoty, souhrnné
zaúčtování, storno, skladové návaznosti a PDF souhrnné dávky.

| Návaznost                         | Aktuální stav                                                                                                             |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| Párování různých měn záloh/vratek | P16-13j implementované; migrace 058, nové i existující úhrady, ochrana DDPP; bankovní regrese 71/71                       |
| Příjem bankovních notifikací      | Implementovaná autentizovaná vlastní brána nad stejnými importními příkazy; nejde o univerzální parser textů e-mailů bank |
| Tiskové předvolby                 | Navazují na P20; původní uložená PDF se nemění                                                                            |
| Členění DPH                       | Vyžaduje klasifikaci a verzovaná pravidla P19, samotný součet DPH ji nenahrazuje                                          |
| Terminálový hardware              | Konfigurace a identifikace existují, externí zařízení potřebují konkrétní protokol                                        |
| Skutečné bankovní exporty         | Syntetické testy fungování parserů nenahrazují ověření anonymizovaného výpisu konkrétní banky/verze                       |

Podrobnosti: [akceptace P16](P16_COMPLETION.md), [bankovní měny](specs/bank-currencies.md),
[párování různých měn](specs/bank-foreign-matching.md), [automatika](specs/bank-auto-matching.md).

## P17 — příjem dokladů

Upload, originály, deduplikace, archivace/obnova, historie, vazba na existující
doklad a atomický převod do přijaté faktury jsou implementované. Implementované adaptéry rozšiřují ISDOC o vložené soubory v omezeném PDF,
QR Fakturu v PNG/JPEG, autentizovaný příjem z vlastní e-mailové brány a
volitelné vytěžení instalačně nakonfigurovaným poskytovatelem.

Výstup rozpoznání je vždy návrh k uživatelské kontrole. Samotná QR Platba
není faktura; chybějící údaje se nevymýšlejí. Gateway může používat vlastní sběr pošty. Nativní IMAP klient navíc umí TLS přihlášení instalačně vybranou schránkou, ruční načtení a samostatně zapínané plánování; podporuje heslo/aplikační heslo, nikoli OAuth. Běžné PDF bez vloženého ISDOC vyžaduje
ruční zadání nebo nastaveného poskytovatele. Další ISDOC režimy se uzavírají
po jednotlivých kontraktech, ne označením celého formátu za podporovaný.

Nahrávání přetažením, náhled originálu a explicitní vytvoření/porovnání dodavatele jsou implementované; nejnovější browser regrese je zaznamenaná v [validačním protokolu](VALIDATION_2026_10_18.md). Vlastní oznámení příjmu jsou implementovaná s opt-in kategoriemi a kurzorem přečtení pro uživatele/profil. Odpojení originálu je implementované s potvrzením, historií a zachováním účetního dokladu i jeho přílohy. Předání účetnímu je explicitní SMTP odeslání jednoho původního souboru s potvrzením, snapshotem, historií a ochranou proti opakování; bez veřejného odkazu.

Podrobnosti: [schránka dokladů](specs/inbox.md), [IMAP](specs/inbox-mailbox.md).

## P18 — sklad a ceník

- Implementované: skladové karty, ruční i dokladové pohyby, inventury,
  historie, stavy k datu, souhrnné ocenění a pořizovací ceny.
- Implementované: ocenění vratek, cenové opravy mezi zásobu a výdej,
  vedlejší náklady, ocenění počátečních stavů a inventurních přebytků.
- Implementované: prodeje po položkách, zdrojové doklady, filtry a CSV/XLSX.
- Implementované: Money S3 základní ceník XML a obecné mapování CSV/XLSX.
- Money S4/S5: podle rozhodnutí uživatele 18. 10. 2026 se neověřené varianty
  řeší obecným mapováním nebo vlastním adaptérem provozovatele. Nativní podporu
  bez doloženého kontraktu neslibujeme; nejde již o požadavek na dodání uživatelova výpisu.

Metoda ocenění je vlastní doložené rozhodnutí Open Faktury; numerická shoda
s iDokladem není potvrzená. Záporná či neznámá zásoba není nulová hodnota.
Cizoměnové skladové příjmy přijímají výslovnou pořizovací cenu a vedlejší náklady v CZK, nezávisle na měně závazku; neznámá cena se neodvozuje od prodejní ceny ani neoznačuje za nulu.
Podrobnosti: [sklad](specs/stock.md), [prodeje položek](specs/item-sales.md),
[import ceníku](specs/catalog-import.md).

## P19 — přehledy, integrace a daně

Měsíční/kvartální/roční přehledy, kumulace, meziroční srovnání, bilance,
největší partneři/dlužníci, pásma prodlení, evidence DPH a orientační obrat
jsou implementované. Přehledy používají uložené kurzy do CZK, zahrnují
samostatná pokladní plnění a nezapočítávají úhrady či prodejky podruhé.
Uložené bankovní kurzové rozdíly mají samostatný přehled.

| Návaznost                         | Aktuální stav                                                                                                                                                                                                                             |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| ARES                              | Implementovaný v kontaktech i fakturačním profilu                                                                                                                                                                                         |
| Registr DIČ                       | Implementované oficiální české RWS a EU VIES: typ subjektu, spolehlivost, zveřejněné účty a odlišení výpadku od nenalezení                                                                                                                |
| ISDOC export                      | Implementovaný ISDOC 6.0.2 běžných vystavených faktur v podporovaných měnách; ověření oficiálním XSD, vyúčtování záloh zatím vyloučené                                                                                                    |
| Evidence DPH CSV/XLSX             | Implementovaný export filtrovaných zdrojů ze společného databázového snapshotu                                                                                                                                                            |
| Kurzy ČNB                         | Návrh kurzu v editoru a explicitní volba aktuálního kurzu pro pravidelné faktury; výpadek ČNB nevystaví doklad s vymyšleným kurzem                                                                                                        |
| Varianty widgetů                  | Implementované preference vlastního účtu: pořadí, viditelnost a kompaktní/podrobná varianta                                                                                                                                               |
| Různé kurzy záloh/DDPP/vyúčtování | Neplátce bez DDPP má samostatné původní kurzy záloh a kurz konečného plnění; přesné native odpočty zůstávají. Různé ekonomické kurzy DDPP u plátce zůstávají blokované do ověření daňového algoritmu                                      |
| Účetní exporty dalších systémů    | Doložené formáty jako nativní adaptéry; ostatní může podle rozhodnutí uživatele 18. 10. 2026 doplnit provozovatel přes dokumentované rozhraní                                                                                             |
| Údaje daňového subjektu           | Explicitní profil s verzí, auditem a oficiálním číselníkem finančních úřadů/pracovišť; neověřuje místní příslušnost ani nepodává přiznání                                                                                                 |
| Daňové režimy a formuláře         | Explicitní klasifikace podkladů obratu a uplatňované vstupní DPH se snapshotem zdroje implementovaná; vlastní přiznání/KH/SH stále otevřené; nutná klasifikace plnění, pravidla nároku na odpočet a oficiální schémata konkrétního období |

Evidence DPH není daňové přiznání a orientační obrat není právní posouzení
zákonného obratu. Technické regresní testy tato pravidla nenahrazují.

## P20 — úplnost produktu

| Oblast                      | Aktuální stav                                                                                                                                                                                                                                                              |
| --------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Správa profilů              | Implementovaná správa členů podle e-mailu, role, převod vlastnictví, odchod a obnova smazaného profilu; ochrana posledního vlastníka                                                                                                                                       |
| Přenosný export profilu     | Implementovaný ZIP s explicitním manifestem tabulek/sloupců, přílohami a redakcí tajemství; není databázovou zálohou                                                                                                                                                       |
| Štítky                      | Implementovaný profilový číselník, vazby a serverové filtrování faktur, prodejních/nákupních dokladů a kontaktů                                                                                                                                                            |
| Tiskové výstupy             | Implementované hromadné fakturační PDF, RTF faktury, dodací list s volbou cen (profilová předvolba nebo jednorázově) a potvrzení úhrady                                                                                                                                    |
| QR Platba                   | QR Platba, QR Faktura a Platba+F pro podporované nové faktury; explicitní výběr a nezávisle dekódované CS/EN testy, původní PDF beze změny                                                                                                                                 |
| Vzhled aplikace             | Světlý/tmavý/systémový režim je uložený na účtu                                                                                                                                                                                                                            |
| Nápověda                    | Uživatelské postupy CS/EN, stav serveru/databáze a odkazy na instalaci, vydání a hlášení chyb                                                                                                                                                                              |
| Vratný koš                  | Nevystavené nepropojené návrhy a nepoužité nabídky/šablony; obnova, revize/audit. Vydané, navázané, sdílené, odeslané a exportem uzamčené doklady jsou chráněné                                                                                                            |
| Exportní zámky              | Neměnné JSON snapshoty, SHA-256, historie, ochrana obsahu a vlastnické odemčení s důvodem; není nativní účetní export                                                                                                                                                      |
| Brand dokumentů             | Logo, podpis/razítko a CS/EN patičky; snapshoty nových faktur, nabídek, záloh, pokladních dokladů, DDPP a dobropisů; historická PDF beze změny                                                                                                                             |
| Desktopové instalátory      | Matice do draft Releases, lokálně ověřený macOS debug DMG a SHA-256; ručně potvrzený signed updater je build-time opt-in, výchozí vypnutý. Kryptografické/loopback fixture testy, nikoli skutečný OS upgrade; platformní podpisy/notarizace a instalace dalších OS zbývají |
| Anonymizace adresáře        | Pouze vlastník po potvrzení; záznam a adresářová historie, konečný archivovaný stav; finanční snapshoty, původní PDF a ostatní kopie zůstávají                                                                                                                             |
| Instalovatelný web          | Manifest, ikony a online režim; při nedostupné navigaci obecná CS/EN obrazovka, bez offline ukládání účetních dat                                                                                                                                                          |
| Tiskový text před položkami | Firemní CS/EN předvolba přes uložený document_style; nové faktury/nabídky/zálohy/DDPP/dobropisy, bez pokladny. DB 4/4 a product-admin UI 12/12; [rozsah](specs/document-style.md). Samostatný vizuální PDF důkaz víceřádkového textu nebyl pořízen.                        |
| Stále otevřené              | Koš dalších typů dokladů, rozšíření oznámení, širší tiskové preference/výstupy, úplný audit přístupnosti, pokročilé migrace, skutečné OS upgrade/release ověření a osobní desktopový provoz                                                                                |

## Pravidla uzavření

Položku označujeme za dokončenou až po specifikaci, implementaci a ověření.
Shoda s iDokladem vyžaduje konkrétní referenční důkaz. Není dovoleno zmenšit
rozsah přejmenováním funkce, označit vlastní gateway za nativní bankovní
integraci ani skrýt chybějící části za souhrnné „P16–P20 hotovo“.
