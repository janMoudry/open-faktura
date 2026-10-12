# Editor faktury — podklad pro P06

11. 10. 2026, přímý průzkum veřejného nepřihlášeného editoru
https://www.fakturaonline.cz/faktura v prohlížeči. Viz E-014.

## Pozorované UX FakturaOnline

Jeden formulář shora dolů: základní údaje dokladu (typ, číslo, data, splatnost,
platba), rozbalitelné obecné nastavení, zákazník a dodavatel, položky a součty.
U obou stran je IČO, jméno/adresa, další kontakty jsou sbalené. U platby převodem
jsou bankovní pole. Položka nabízí počet, jednotku, popis, DPH, cenu a vypočtený
součet; přepínač zadávání cen s/bez DPH platí pro formulář. Slevy na položku a
celý doklad jsou volitelné akce; sleva na doklad otevírá dialog s výší a typem.
Obecná sekce schovává jazyk dokladu, měnu, zaokrouhlení, QR a vzhled. Výstupní
akce je dole. Přidání položky a rozbalení voleb bylo ověřeno v UI.

Pozorování se nevztahuje na přihlášený adresář, uložení ani PDF. Žádný doklad
nebyl vystaven. Náhled šablony není důkaz algoritmu výpočtů.

## Převod do Open Faktury

Zachovat náš monochromatický design systém, trvalý sidebar a appbar s aktivním
profilem. Převzít jednoduchost průchodu, ne cizí značku nebo všechny volby naráz.

1. Nadpis Nová faktura a stav Návrh. Dodavatel je kompaktní souhrn aktivního
   profilu s akcí Upravit údaje na dokladu; jeho změna nemění profil.
2. Odběratel je první hlavní vstup: hledání v adresáři aktuálního profilu,
   IČO/ARES nebo ruční údaje. Výběr zkopíruje snapshot. Ručně zadaného zákazníka
   lze uložit do adresáře explicitně, nevytvářet záznam při každém psaní.
3. Vedle sebe datum vystavení, splatnost a způsob úhrady. Předvyplnit z profilu
   a kontaktu; u neplátce skrýt daňové ovladače. Návrhu nepřidělovat konečné číslo.
4. Jedna první položka: popis, množství (1), jednotka, cena; u plátce sazba.
   Sleva a doplňkový text až na vyžádání. Přidání další položky přesune focus
   do jejího popisu. Na mobilu řádky skládat jako karty s vlastními labely.
5. Rekapitulace z backendu průběžně, při načítání označit starý výsledek;
   stará odpověď nesmí přepsat novější vstup. Chyba přepočtu nesmí vypadat jako
   nulová faktura. Částky podle jazyka, transport jako desetinné řetězce.
6. Méně časté údaje pod Další možnosti: objednávka, texty, jazyk dokladu.
   Nastavení účtu a globální branding do editoru nepatří.
7. V P06 hlavní akce Uložit návrh, žádné předstírané vystavení/PDF. P07 přidá
   samostatné Vystavit s validací a číslováním. Překlady CS/EN od začátku.

Přepnutí profilu nebo opuštění rozepsané faktury vyžaduje stávající ochranu
změn. Seznam návrhů má filtry a serverové stránkování. Prázdný stav musí vést
přímo k první faktuře, nikoli nutit znovu do plné konfigurace firmy.

## Korekce po přihlášeném průchodu 11. 10. 2026

E-020/E-021 a [audit](reference-audit-2026-10-11.md) doplňují původní veřejný
průzkum. V aktuálním editoru jsou data/platba před stranami dokladu podle
FakturyOnline; zákazník začíná hledáním, ruční pole se otevírají explicitně.
Výběr kontaktu zobrazí souhrn s možností editace snapshotu. Toto pořadí není
kopie iDokladu (ten začíná odběratelem/popisem, data a platbu má vedle sebe).
Samostatný návrh/vystavení a zákaz změn vydaného dokladu zůstávají vlastní
rozhodnutí, nikoli shoda s pozorovaným iDokladem.
