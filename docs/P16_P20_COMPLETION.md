# Dokončení P16–P20

Aktivní rozsah potvrzený uživatelem 15. 10. 2026: všechny zbývající části
P16 až P20, včetně backendu, PostgreSQL, REST/MCP, češtiny a angličtiny.
Tento seznam je sledování práce, nikoli prohlášení o dokončení. Autoritativní
rozsah zůstává v IMPLEMENTATION_PLAN.md a FEATURES.md; nic se nevypouští
přesunem mezi etapami.

## P16 — Finance

Podrobná akceptace a stav: [P16_COMPLETION.md](P16_COMPLETION.md).
Dokončeno P16-12: společné platební operace záloh a dobropisů v bance/pokladně.
P16-09a má konfiguraci terminálů i jejich snapshoty na prodejkách.
CZK prodejky a dávkové zaúčtování mají backend/DB/REST/MCP/CS/EN a regrese;
skladové a základní přehledové návaznosti jsou implementované. PDF dávky
doplňuje P16-08b níže; úplné daňové režimy zůstávají otevřené.
Dále zbývají měny, notifikace,
tiskové nastavení a členění DPH. Skutečné bankovní exporty a živé ověření
adaptérů vyžadují doložené vstupy; syntetické testy je nenahrazují.

- [x] P16-13h: zahraniční IBAN bankovních účtů, normalizace a kontrola formátu/checksumu,
      zachovaná shoda účtu a měny při importu; bez migrace. Podrobnosti v
      [bank-currencies.md](specs/bank-currencies.md#p16-13h--zahraniční-iban-bankovních-účtů).
      Návaznosti: zahraniční IBAN na dokladech/QR, automatika záloh/vratek
      a jejich párování mezi různými měnami.

## P17 — Příjem dokladů

- [x] P17-01: upload, originál, deduplikace, archiv/obnova, auditní historie,
      připojení k existujícímu dokladu a atomický převod do přijaté faktury
      včetně přílohy. PostgreSQL regrese a CS/EN desktop/mobile scénáře.
- [ ] P17-02 Omezený ISDOC 6.0.1/6.0.2 je implementovaný s kontrolou součtů
      a potvrzením do závazků. Zbývají QR, PDF s vloženým ISDOC a další režimy.
- [ ] P17-03 Příjem e-mailů, přílohy, autentizace vstupu, deduplikace a diagnostika.
- [ ] P17-04 Volitelné vytěžení, vlastní nastavení poskytovatele, kontrola návrhu;
      základní tok bez AI, žádné automatické zaúčtování nedůvěryhodného výstupu.

## P18 — Sklad

- [x] P18-01 Skladové karty, počáteční stavy a ruční pohyby s auditní stopou;
      přesnost, role, replay, CS/EN a REST/MCP ověřené.
- [x] P18-02 Explicitní vazby řádků na sklad: vystavení faktury, potvrzený příjem
      nákupu, opravy/storna nákupů a prodejek, množstevní vratky dobropisů.
      Atomické pohyby, replay, limity vratek a profilová izolace; staré doklady
      se nezapojují zpětně odhadem. Záporná zásoba je povolená.
- [x] P18-03 Inventura, dohledatelné rozdíly a historie jsou implementované.
      P18-03a doplňuje statistiku vybrané karty za období (počáteční/konečné saldo,
      ruční příjmy/výdeje a oddělené úpravy), nezávislou na stránkování.
      P18-03b přidává souhrnný stav k datu, hledání, stránkování a explicitní
      časově platné ruční ocenění CZK s historií a oddělenými neznámými hodnotami.
      P18-03c přidává explicitní pořizovací ceny nákupních příjmů a konzervativní
      průběžný průměr běžných příjmů/výdejů. P18-03d doplňuje ocenění vratek,
      přepočet cenových oprav mezi zásobu a výdej a rozvržení vedlejších nákladů.
      P18-03e doplňuje pořizovací ocenění počátečních stavů, ručních příjmů a inventurních
      přebytků včetně přesného storna. Neznámé/nekonzistentní vstupy se neodhadují; shoda metody
      s iDokladem není doložená (E-056).

- [x] P18-04 Přehled prodeje po položkách: období, typ, ceníkové služby/zboží,
      uložené částky bez/s DPH, vratky, zdroje, sloupce a CSV/XLSX. REST/MCP a CS/EN.
      Vlastní pravidla dobropisů a data započtení: `specs/item-sales.md`, důkaz E-057.
- [x] P18-05a Money S3: nativní XML základních cen, náhled, konflikty, deduplikace, REST/MCP a CS/EN. Rozsah: specs/catalog-import.md (E-058).
- [ ] P18-05b Money S4/S5: chybí konkrétní anonymizovaný export a jeho konfigurace/verze; veřejná nápověda dokládá konfigurovatelné objekty a XSLT (E-059), nikoli ceníkový kontrakt.
- [x] P18-05c Obecný CSV/XLSX import ceníku: mapování, volba cen s/bez DPH a desetinného oddělovače, společný náhled a potvrzení. Není nativním adaptérem S4/S5.

## P19 — Přehledy, integrace a daně

- [ ] P19-01 Měsíční CZK přehled hodnot dokladů s dohledáním zdrojů je
      implementovaný a ověřený (PostgreSQL, REST/MCP, CS/EN desktop/mobile). Podporuje ceny s/bez DPH, kumulaci od začátku období, meziroční srovnání
      a přechod na loňské zdroje. P19-01c doplňuje kvartální a roční členění včetně víceletého rozsahu. P19-01d doplňuje bilanci, TOP 3 odběratelů/dlužníků a pásma prodlení nad aktuálními CZK zůstatky. P19-01e přidává evidenci uložené DPH a orientační obrat se zdroji. Zbývá daňová klasifikace pro zákonný obrat a nárok na odpočet, vlastní varianty widgetů a dokončení měnových návazností (kurzové rozdíly, automatické kurzy, rozdílné kurzy záloh/DDPP/vyúčtování a sklad).
- [x] P19-01f: samostatná pokladní plnění v evidenci DPH a orientačním obratu, podle uloženého rozpisu a DUZP, bez dvojího započtení úhrad.
- [x] P19-01g: kurz běžných vydaných/přijatých faktur, ruční úhrady a přepočet přehledů do CZK; omezení viz specs/report-currencies.md.
- [x] P19-01h: cizoměnové dobropisy, zápočty a ruční bankovní vratky; původní kurz, PDF a přehledy. Párování na CZK banku/pokladnu není povolené.
- [x] P19-01i: zálohy, DDPP a vyúčtování v cizí měně se shodným uloženým kurzem; různé kurzy, kurz platby a kurzové rozdíly zbývají.
- [ ] P19-02 ARES/DIČ: dokončení podle FEATURES včetně chybových stavů integrací.
- [ ] P19-03 Účetní exporty po cílových systémech, verzované kontrakty a validace.
- [ ] P19-04 Daňové režimy a formuláře po verzích podle oficiálních podkladů;
      účetní a daňová pravidla nejdřív doložit, neodvozovat z vzhledu reference.

## P20 — Úplnost produktu

- [ ] P20-01 Štítky, koš, obnova a pravidla mazání historických dokladů.
- [ ] P20-02 Zámky po exportu, revize a audit exportů.
- [ ] P20-03 Hromadný tisk, RTF, dodací listy a potvrzení úhrady.
- [ ] P20-04 QR Platba v nových PDF faktur/záloh je implementovaná; ověřeno
      dekódování CS/EN PDF. Zbývají QR Faktura, Platba+F a tiskové preference.
- [ ] P20-05 Brand dokladů, podpis/razítko, jazyk a tiskové předvolby.
- [ ] P20-06 Světlý/tmavý/systémový režim s uložením na účtu je implementovaný
      a ověřený v CS/EN desktop/mobile. Úplný audit přístupnosti a preference
      oznámení zbývají.
- [ ] P20-07 Převod vlastnictví, odchod, odstranění profilu a anonymizace historie.
- [ ] P20-08 Úplný export dat a pokročilé migrační adaptéry.
- [ ] P20-09 Nápověda, provozní dokumentace a aktualizace klientů s vazbami na P11.

Každá položka se uzavírá až po specifikaci, implementaci a ověření. Označení
„odpovídá iDokladu“ vyžaduje konkrétní referenční důkaz; technická regrese sama
shodu nepotvrzuje. Externě neověřené části mají zůstat viditelně otevřené.

P19-01j: návrh kurzu ČNB na vyžádání v editoru, REST/MCP, kontrola profilu a CS/EN. Ukládá se vybraný kurz; automatika pravidelných dokladů a kurzové rozdíly zbývají. [Rozsah a omezení](specs/exchange-rates.md).

P16-13a / P19-01k: ruční párování CZK pohybu s běžnou cizoměnovou vydanou/přijatou fakturou, oddělené částky a uložený rozdíl vůči kurzu dokladu. Bez automatického odhadu a účetního zápisu. Cizoměnové účty, existující úhrady a širší měnové návaznosti zbývají. [Specifikace](specs/bank-foreign-matching.md).

P16-13b / P19-01l: připojení existujících cizoměnových převodních úhrad běžných faktur bez duplikace; odpojení zachová původní platbu. CS/EN, REST/MCP a migrace 053. [Pravidla a zbývající rozsah](specs/bank-foreign-matching.md).

P19-01m: souhrnný přehled uložených kurzových rozdílů z aktivních bankovních vazeb, zisky/ztráty/netto, období a stránkované zdroje s odkazy. CS/EN, REST/MCP; bez účetních zápisů a nové migrace. [Specifikace](specs/exchange-difference-report.md).

P16-13c: cizoměnové bankovní účty, ruční pohyby a CSV/camt/Fio/MONETA importy,
saldo v měně účtu, kontrola měny a migrace 054. CS/EN, REST/MCP.
Párování z cizoměnových účtů a zahraniční RB ABO stále navazují.
[Specifikace](specs/bank-currencies.md).

P16-13d: ruční párování a rozdělení cizoměnových pohybů s běžnými fakturami
ve stejné měně, existující převodní úhrady bez duplikace, odpojení a CS/EN.
Bez nové migrace; REST/MCP sdílejí bankovní příkazy. Automatiku doplňuje P16-13e,
zálohy/vratky P16-13f. Různé měny a zahraniční IBAN zbývají.
[Specifikace](specs/bank-currencies.md#p16-13d--ruční-párování-ve-shodné-měně).

P16-13e (18. 10. 2026): dávkové a opt-in automatické párování běžných faktur
ve shodné měně, konkurenční pohyby se porovnávají jen v téže měně. CS/EN,
stejné REST/MCP příkazy a worker, bez migrace. Ruční non-CZK párování záloh/vratek
doplňuje P16-13f; různé měny a zahraniční IBAN zbývají. [Specifikace](specs/bank-currencies.md).

P16-13f (18. 10. 2026): ruční a dělené párování záloh a vratek dobropisů
z účtů ve shodné měně, připojení existujících převodních úhrad bez duplicity,
ochrany DDPP a rezervací. CS/EN, společné REST/MCP, bez migrace.
Zbývá párování rozdílných měn z non-CZK účtů, zahraniční IBAN a automatika
záloh/vratek. [Specifikace](specs/bank-currencies.md).

P16-13g: ruční párování různých měn z cizoměnové banky s běžnou vydanou
a přijatou fakturou, včetně připojení existující úhrady. Explicitní bankovní
kurz do CZK, uložené hodnoty obou stran a rozdíl; migrace 055, REST/MCP a CS/EN.
Zbývají rozdílné měny záloh/vratek, jejich automatika a zahraniční IBAN.
[Specifikace](specs/bank-foreign-matching.md#p16-13g--rozdílné-měny-z-cizoměnového-účtu).

### Dodávka 18. 10. 2026 — P16-13i, P16-08b, P16-14b

- [x] Zahraniční IBAN profilů a vystavených dokladů; historické PDF beze změny. QR zůstává CZ/CZK.
- [x] PDF/tisk souhrnného zaúčtování prodejek v CS/EN, snapshot vystavitele, cache po revizích, označení storna (migrace 056).
- [x] Samostatný opt-in pro automatické párování záloh a vratek ve stejné měně, existující úhrady bez duplicity (migrace 057).

Specifikace: [měny](specs/bank-currencies.md), [prodejky](specs/sales-receipts.md),
[automatika](specs/bank-auto-matching.md). Neuzavírá celé P16–P20. Otevřené
zůstávají rozdílné měny záloh/vratek, notifikace bank, příjem e-mailů a
vytěžování, S4/S5, účetní/daňové exporty a zbývající funkce P20.
