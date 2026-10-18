# P16-07/P16-08 — Prodejky a souhrnné zaúčtování

Stav 15. 10. 2026: implementovaný tuzemský CZK tok prodejek a dávkového
zaúčtování (migrace 039/040). Skladové a daňové návaznosti P18/P19 zbývají.
Podklady E-047 a E-050, zejména https://www.idoklad.cz/podpora/prodejky.
Číselné příklady a pravidla níže jsou návrh Open Faktury, nikoli potvrzená
numerická shoda reference.

## Hranice modelu

Prodejka je samostatný číslovaný doklad o prodeji a jeho plné úhradě.
Nezapisuje do invoice_payment ani automaticky do bankovních pohybů.
Způsoby úhrady hotovost/karta/stravenka lze kombinovat; jejich součet se musí
přesně rovnat splatné částce. Odběratel je nepovinný. Uloží se snapshot
vystavitele, případného odběratele, položek, terminálu a výpočtů.

Vystavení transakčně přidělí číslo, uloží přesné částky jednotlivých metod,
přidá audit a případné skladové pohyby P18. Opakování stejného příkazu nesmí
přidělit další číslo ani další pohyb. Čas auditu je oddělený od data prodeje.
Převod změny do stornovacích/opravných záznamů musí být dohledatelný, bez
přepisování zdrojové historie. Fyzický tisk se neposuzuje jen podle PDF bajtů.

## Zaúčtování

Samostatný potvrzovaný příkaz s náhledem období a vybraných prodejek.
Náhled obsahuje počty, skupiny podle terminálu/odběratele, platební metody,
daňové souhrny a fingerprint revizí. Zápis znovu ověří oprávnění a revize,
rezervuje všechny zdroje a je atomický. Zdroj lze použít pouze v jedné
aktivní dávce. Výsledkem jsou pokladní doklady pouze za hotovost a interní
doklady pro kartu/stravenky. Interní doklad není nová peněžní úhrada.

Přenos používá uložené částky a rozpisy. Rozdělení zaokrouhlovacích zbytků
mezi metody potřebuje explicitní deterministické pravidlo a ručně vypočtené
regresní příklady; dokud chybí, nesmí se nahrazovat přibližnými float výpočty.
Storno dávky zachová historii, zruší její aktivní výsledné evidence a uvolní
zdroje. Pokladní saldo i daňové výstupy se započtou pouze jednou.

## Akceptace před uzavřením

- Backend/DB/REST/MCP a CS/EN seznam, editor, detail, filtry, čísla, PDF a audit.
- Přesná kombinace úhrad, nedoplatek/přeplatek odmítnutý před zápisem.
- Archivovaný terminál odmítnut u nového dokladu; starý snapshot zachovaný.
- Souběh číslování, replay, profilové role a zabránění dvojímu zaúčtování.
- Daňový rozpis a zaokrouhlení několika sazeb i kombinací plateb.
- Náhled dávky se změněným zdrojem odmítnutý bez částečného zápisu.
- Oprava a storno včetně pokladního salda, P18 skladu a P19 výstupů.
- Daňové podmínky ověřené proti aktuálním oficiálním zdrojům.

## Výpočet P16-07 — vlastní rozhodnutí 15. 10. 2026

CZK používá stávající of-czk-v1 kalkulátor. Úhrady jsou nezáporné částky
s nejvýše dvěma desetinnými místy; součet přesně odpovídá payable, payable > 0.
Pořadí rozdělení je hotovost, karta, stravenka. Každý základ a daň v uloženém
souhrnu se rozdělí kumulativním poměrem skutečných úhrad k payable, zaokrouhleným
na haléř half-away-from-zero; poslední metoda dostane přesný zbytek. Hrubá
částka je součtem rozděleného základu a daně. Rozdíl úhrady proti hrubému
součtu je uložené zaokrouhlení. Zaúčtování pouze sčítá tyto snapshoty.
Toto pravidlo není ověřený algoritmus iDokladu.

První podporovaný režim je tuzemský CZK prodej, standardní DPH nebo neplátce.
Pro plátce zavádíme produktový limit 10 000 Kč podle referenční nápovědy;
vyšší prodej se řeší fakturou. Nejde o tvrzení úplné podpory všech právních
režimů. Cizí měny, přenesená daň a zahraniční plnění vyžadují vlastní návaznost.


## Ověření implementace

Čtyři izolované PostgreSQL scénáře v `tests/retail.rs` ověřují přesné úhrady,
replay, role, snapshot terminálu, číslování, PDF, storno, souběžné zaúčtování,
zastaralý náhled a shodu REST/MCP. Regrese všech 16 pokladních scénářů prošla.
CS/EN desktop/mobile `cash.spec.ts` ověřuje vytvoření prodejky, PDF,
náhled, zaúčtování a zrušení dávky. PDF pokladního dokladu obsahuje převzatý
souhrn DPH. Interní rozpis karet/poukázek je uložený v dávce; tisk celé dávky včetně interních skupin doplňuje P16-08b níže.

## P16-08b — PDF zaúčtování (18. 10. 2026)

REST `/retail/postings/{id}/pdf?language=cs|en` a MCP `retail_posting_pdf`
vrací společnou sestavu všech skupin v dávce, včetně interních čísel
karet/stravenek, terminálu, odběratele, zdrojových ID a uložených rozpisů.
Neprovádí nový výpočet ani novou úhradu. UI nabízí PDF/tisk v historii dávek.
Jde o vlastní tiskové rozložení nad E-050, nikoli doloženou vizuální shodu.

Migrace 056 přidává snapshot vystavitele při zaúčtování a cache podle
profilu, dávky, revize a jazyka. Staré dávky mají issuer=NULL a výslovné
upozornění, že historické údaje nejsou uložené. Dnešní údaje se nedoplňují.
Storno vytváří PDF nové revize s označením a důvodem; stará cache zůstává.
Přístup ověřuje členství i při cache hitu. Renderer nemá síťové I/O.

Test retail ověřuje CS/EN PDF, cache, izolaci, storno, REST/MCP a upgrade055;
UI cash.spec.ts ověřuje dostupné PDF. Systémový tisk Tauri není tímto testem
ověřen. Úplné daňové režimy prodejek zůstávají součástí P19.
