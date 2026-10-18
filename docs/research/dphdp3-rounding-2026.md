# DPHDP3: ověření zaokrouhlení pro další implementaci

Průzkum 18. 10. 2026. Pouze důkazy a mezery; žádná změna výpočtů ani tvrzení dokončeného exportu. Navazuje na `../specs/tax-return-domestic-proposal.md`.

## Aktuální primární zdroje

1. [Seznam tiskopisů Finanční správy pro rok 2026](https://financnisprava.gov.cz/cs/dane/danove-tiskopisy?rok=2026) při přímém stažení HTML uvádí přiznání 25 5401 vzor 26 a pokyny 25 5412 vzor 22. Není správné aktuálnost odvozovat z vyhledávačem nalezeného seznamu pro rok 2025 (vzor 25/21).
2. [5401_26.pdf](https://financnisprava.gov.cz/assets/tiskopisy/5401_26.pdf), přímo staženo 18. 10. 2026, SHA256 `d1ce6df787360729e9c50efa85c3d4329114e9e02e380fe1da66ee3470ce7484`. První strana nad tabulkou požaduje údaje přiznání v celých korunách. Nestanoví konkrétní metodu převodu haléřových součtů.
3. [5412_22.pdf](https://financnisprava.gov.cz/assets/tiskopisy/5412_22.pdf), 235 846 bytů, přímo staženo 18. 10. 2026, SHA256 `3e264e7008a20e86283b7199bf5b0d25e42d981a45306933f35143c1913dfc11`. Text extrahován pomocí pdftotext -layout. Aktuální pokyny uvádějí součtové/rozdílové řádky a dovolují záporné opravy, ale nebyla v nich nalezena explicitní obecná metoda zaokrouhlení dílčích řádků. To je zjištění z přečteného dokumentu, nikoli důkaz neexistence jiného pokynu.
4. [Daňový řád 280/2009 Sb., aktuální informativní znění e-Sbírky od 1. 1. 2026](https://e-sbirka.gov.cz/sb/2009/280?odkazId=37020125), §146: odst.1 stanoví celé koruny nahoru pro daň; odst.3 upravuje přesnost výpočtů sazeb, koeficientů, ukazatelů a měnového přepočtu na dvě platná desetinná místa a zakazuje postupné zaokrouhlování ve více stupních. Odst.2 již obsahuje i samostatné pravidlo pro zálohu na daňový odpočet, takže starší kopie zákona bez této věty není spolehlivým dokladem aktuálního znění. Pravidlo pro výslednou daň samo neurčuje konkrétní zaokrouhlení každého základu a dílčí DPH v XML.
5. Oficiální DPHDP3 XSD 03.01.04, 11. 8. 2026, [dphdp3_epo2.xsd](https://adisspr.mfcr.cz/adis/jepo/schema/dphdp3_epo2.xsd), SHA256 `08e7b168b0675c896298217b7049889d14789353220a973665a0bf33064aa0cf`. Podrobnosti v `epo-schema-inventory.json`. Atributy navrhovaného tuzemského rozsahu mají fractionDigits=0 a totalDigits=14. Schéma určuje lexikální rozsah, nikoli právní rounding algoritmus.

Aktuální PDF byly skutečně přečteny, nebylo provedeno podání ani nahrání XML do EPO. Dočasné stažené soubory a textové extrakce jsou v tmp/p19-integration/epo; nejsou účetní fixture ani pozorování referenční aplikace.

## Co je doložené pro výsledné řádky

Pokyny vzor22 přímo popisují:

- ř.46 jako součet ř.40–45 ve sloupcích plný a krácený nárok;
- ř.62 jako daňové součty ř.1–13 minus ř.61 a příslušná další daň podle §108;
- ř.63 jako plný nárok ř.46 + odpočet ř.52 + změna ř.53 + úprava ř.60;
- ř.64 jako kladný rozdíl ř.62−63, ř.65 jako opačný kladný rozdíl;
- ř.66 jako rozdíl ř.62−63 pouze u dodatečného přiznání;
- pro vícenásobné odrážky jednoho řádku součet jejich částek.

Tyto vzorce lze použít nad doloženými hodnotami řádků. Nevysvětlují, zda se pro součtový řádek mají znovu vzít původní nezaokrouhlené dokladové částky, ani samy nerozhodují o rozdílu součtu samostatně zaokrouhlených řádků vůči zaokrouhlení jejich celkového původního součtu. Implementace musí zachovat jednoznačně doloženou návaznost, nikoli smazat rozdíl dodatečným přepočtem jedné kolonky.

## Nepřenášet pravidla z jiného problému nebo historického období

[Informace FS z 14. 10. 2019 o výpočtu DPH a zaokrouhlování úplaty](https://financnisprava.gov.cz/cs/financni-sprava/novinky/novinky-2019/vypocet-dph-a-zaokrouhlovani-od-1-10-2019) řeší doklad a úplatu podle §37, včetně hotovostního zaokrouhlení. Není specifikací celých korun v přiznání. Uváděné historické sazby 21/15/10 nelze přenést na rok2026.

[Stará HTML nápověda pokyny_dp1](https://adisspr.mfcr.cz/adis/jepo/dph/pokyny_dp1.htm) uvádí celé koruny pro základ/daň/plnění, ale neověřuje současnou formu 5401_26 ani znaménkovou rounding metodu. Staré zákony o správě daní a poplatků, historická znění §37 a účetní blogy nepoužíváme jako autoritativní aktuální pravidlo. Pravidla kontrolního hlášení dovolující haléře nejsou pravidlem DPHDP3.

## Přesná zbývající mezera

Z dosud nalezených aktuálních primárních zdrojů nelze poctivě uzavřít následující očekávané výsledky pro automatický generátor:

1. Směr a tie-breaking při převodu každého agregovaného základu a každé agregované dílčí daně na celé CZK (nejen výsledné daňové povinnosti).
2. Jednoznačný výsledek záporných hodnot −100,49 / −100,50 / −100,51: nestačí bez výkladu zvolit ceiling, away-from-zero či matematické zaokrouhlení knihovny.
3. Doložené pořadí agregace, zaokrouhlení základních řádků a odvození ř.46/62/63/64/65 v případě, kdy jednotlivé daňové řádky obsahují haléře.
4. Způsob zobrazení/serializace přesné nuly a obou výsledných nulových kolonek při rovnosti ř.62 a63; XSD optional není věcným pravidlem.
5. Referenční příklad s opravou a záporným řádkem v aktuální verzi přiznání; samotná podpora záporných čísel v XSD není důkaz správného zaokrouhlení.

Další použitelný důkaz musí být aktuální oficiální metodický pokyn či technické pravidlo EPO, nebo explicitně označené reprodukovatelné pozorování aktuálního formuláře s uvedenými vstupy/verzí. Pozorování samo zůstane oddělené od právní interpretace. Odpověď podpory by musela konkrétně řešit tyto případy; nebyla vyžádána ani žádná zpráva odeslána.

Do té doby návrh bezpečně omezuje první export na přesně celočíselné agregované základy i dílčí daně a blokuje všechny necelé případy. To není náhradní právní algoritmus, pouze omezení podporovaného rozsahu. Negativní/opravné případy navržená první dodávka nepodporuje. Žádná rounding funkce ani test s vymyšlenými právními expected hodnotami nebyly přidány.
