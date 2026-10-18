# Referenční scénáře

R-01 až R-06 a R-08 až R-10 jsou **připravené, nespouštěné**. R-07 byl 11. 10. 2026
částečně proveden: 300 Kč → úhrada 100 Kč → doplatek 200 Kč, neplátce CZK;
viz E-021 a screenshoty. Přeplatek, odpojení, nulový a opravný doklad neověřeny. Očekávané částky doplnit z reference, nevypočítat vlastním budoucím kalkulátorem. Konkrétní daňové nastavení a sazby zachytit z testovací agendy; zde se neurčují právní pravidla.

Před každým během zachytit legislativu, režim DPH, měnu, nastavení desetinných míst, roli, verzi rozhraní a datum. Používat syntetické kontakty v testovací agendě. U výpočtů uložit vstup a odpověď přepočtu; pokud se ověřuje vystavení, také detail uloženého dokladu a PDF. Anonymizované artefakty patří do `docs/research/fixtures/<scenario>/<run>/` až po skutečném běhu.

| ID | Vstup a postup | Co zachytit |
| --- | --- | --- |
| R-01 | Jedna a pak tři položky, množství 3 a 0.125, cena 0.01 a 19.995; režim ceny s DPH i bez DPH; více povolených sazeb | Základ/DPH/celkem po položkách i sazbách; rozdíl náhledu, uložení a PDF |
| R-02 | Stejná faktura bez slevy, s položkovou slevou 10 %, s dokladovou 5 %, s oběma; hraniční hodnoty zvlášť | Pořadí aplikace, základ slevy, zaokrouhlení a zamítnuté hodnoty |
| R-03 | Cizí měna, kurz 25.123 s množstvím kurzu 1 a 100, více položek | Směr přepočtu, částky v obou měnách, přesnost a uložený kurz |
| R-04 | Uložit nový doklad, změnit, kopírovat, smazat a obnovit; zopakovat s úhradou a exportem | Stavy, přidělení čísla, dovolené operace, chybové odpovědi a vazby |
| R-05 | V testovací agendě dva souběžné pokusy o stejné číslo; opakovat identický požadavek | Zda vzniknou duplicity, konflikt, rezervace nebo mezera v řadě; bez tvrzení, že reference podporuje idempotency key |
| R-06 | Vystavit doklad a změnit adresu kontaktu, banku firmy i cenu ceníkové položky | Rozdíly uloženého detailu a nově generovaného PDF; zda existuje explicitní aktualizace údajů |
| R-07 | Kladná faktura: část úhrady, doplatek, přeplatek, odpojení; zvlášť nulový a opravný doklad | Stav úhrady, zbývající částka, datum úhrady, vazba na finanční pohyb a případné chyby |
| R-08 | Dvě testovací firmy, majitel a omezený uživatel; číst a měnit doklad i přílohu přes UI/API | Dostupné role, reakce na cizí ID, rozsah přístupového tokenu |
| R-09 | Dvě zálohy s částečnou úhradou, daňový doklad k platbě, konečná faktura; poté pokus o změnu zdroje | Povolené kombinace, odpočty, vazby, sazby, rozdíly SDK a webu |
| R-10 | Pravidelný vzor: 31. leden, měsíční interval a konec měsíce, přestupný rok, ukončení datem/počtem; zastavení a obnovení, chyba vystavení a e-mailu | Skutečné termíny, doplnění zmeškaných výskytů, role, číslování a oddělení vystavení/odeslání. Zatím pouze veřejná nápověda E-036; živý experiment neproveden. |

Náš akceptační výsledek může být přísnější než reference (např. ochrana proti duplicitnímu požadavku). Takový rozdíl má status vlastního návrhu, nikoli důkazu funkční shody.

## R-11 — upomínky a poděkování

Veřejný podklad E-037; živé odesílání v referenční agendě neprovedeno.
V testovací agendě zbývá referenčně ověřit přesný okamžik první/opakované
upomínky, poděkování při částečné úhradě/stornu, změny kontaktu a odvolané
oprávnění. Naše reprodukovatelné scénáře, vstupy a odchylky jsou ve
[specifikaci](../specs/invoice-communication.md) a
[validačním záznamu](../design/communication-validation.md). Žádné experimenty
proti skutečným odběratelům.

## P15-01 — otevřená živá shoda (E-039)

V testovací agendě ověřit přijatý doklad od neplátce/plátce, přesnou zdrojovou
DPH odlišnou od kalkulátoru, interní a externí číslo, částečnou platbu a storno.
Zaznamenat nastavení firmy, vstup a výstup, nikoli pouze screenshot formuláře.
Převzetí automatického zaplacení podle metody, přeplatků, účetních exportů,
číselných řad a daňových odpočtů není součástí ověřené shody P15-01.

## P15-02 — účtenky a přehled (E-040)

Neověřené živé referenční scénáře: účtenka bez dodavatele kartou/hotově,
oprava částky před a po bankovním párování, odstranění účtenky a dopad
do přehledu za různá období. Nápověda dokládá základ, nikoli naše atomické
platební události a periodizaci. Technické syntetické scénáře jsou v
received.rs a purchase-receipts.spec.ts.

## P16-01 / E-041 — banka

Ověřit na testovací agendě reference: založení a archivaci CZK účtu, import
reálného anonymizovaného exportu, shodné VS při rozdílných částkách, ruční výběr
částečně/již zaplaceného dokladu a odpojení dřívější úhrady. Zaznamenat počáteční
stav, částky, vazby, revize a saldo po každém kroku. Dosavadní podklad je veřejná
dokumentace; vlastní syntetické CSV testuje Open Fakturu, nepotvrzuje bankovní
adaptér ani numerickou/UI shodu iDokladu.

## P16-03 — k ověření proti živé testovací agendě

Dvě platby stejného VS/částky na různých stránkách; více dokladů se stejným VS;
částečná úhrada, předem ručně zaznamenaná platba a změna mezi náhledem a potvrzením.
Zaznamenat konkrétní vstupy, účty a výsledky iDokladu. Dosud pouze veřejné zdroje
E-043 a syntetické technické testy Open Faktury, nikoli potvrzená shoda dialogu.

## Pokladna — doplnění P16-04

Budoucí živé srovnání: dvě testovací CZK pokladny s počátečním stavem,
příjem/výdaj, částečná hotovostní úhrada faktury, již uhrazená účtenka,
odpojení a storno, změna počátečního data po prvním dokladu, tisk CS/EN.
Výsledek zatím nepozorován; aktuální důkaz E-044 je pouze veřejná nápověda.

## P18-05b — Money S4/S5 nativní ceník (čeká na podklady)

Podklad E-059. V testovací agendě zaznamenat verzi Money ERP/S4/S5,
domácí měnu, cenové hladiny, režim DPH a exportní konfiguraci včetně XSLT.
Vyexportovat anonymní dvě karty, základní a vedlejší ceny, nulovou cenu
a kartu bez ceny. Opakovat beze změny a po změně jedné ceny.
Uložit anonymizované XML a popis výběru objektů/polí; XSD, pokud je dostupné.
Porovnat identity, význam částek a vazby na hladiny s hodnotami v aplikaci.
Bez těchto výstupů nelze označit nativní adaptér za ověřený.
