# P18-05a — Import ceníku Money S3

## Podklad a rozsah

E-058 dokládá nativní XML Money S3, nikoli shodu se všemi verzemi Money nebo
živou synchronizaci iDokladu. Podporujeme UTF-8 export MoneyData/SeznamZasoba
do ceníku aktivního profilu. Money S4/S5 zůstávají P18-05b.

## Mapování

| Zdroj v Zasoba             | Cíl                    |
| -------------------------- | ---------------------- |
| Sklad/GUID + KmKarta/GUID  | stabilní zdrojový klíč |
| KmKarta/Popis              | název                  |
| KmKarta/Katalog            | kód                    |
| KmKarta/BarCode            | čárový kód             |
| KmKarta/MJ                 | jednotka               |
| PC bez Hladina: Cena1/Cena | základní prodejní cena |
| PC/SDPH                    | cena s DPH / bez DPH   |
| PC/Mena/Kod                | měna                   |
| konfigurace/SDPH_Prod      | sazba DPH              |

Prázdný kód měny vyžaduje výslovné potvrzení domácí CZK. Ceník nyní podporuje
CZK a sazby 0/12/21; nepodporované hodnoty jsou neplatné řádky.
Výchozí množství je 1; stav zásob se nepřenáší. Další cenové hladiny, množstevní
ceny, obrázky a nákupní ceny se nepřenášejí. Zděděný výpočet základní ceny
a nenulová základní sleva jsou odmítnuty, nikoli odhadnuty.

## Společný průchod

REST POST /api/companies/{company}/imports/money a MCP import_money používají
stejný parser a příkaz přípravy importu. Náhled nic nezapisuje do ceníku.
Po potvrzení používá import_apply existující transakční dávky po 25 řádcích.
Oprávnění a aktivní profil se ověřují při přípravě i zápisu.

Identitu tvoří profil, zdroj money_s3, uživatelem určená stabilní agenda a
dvojice GUID skladu/karty. Stejný obsah je duplicita; změněný obsah konflikt.
Shodný neprázdný kód, barcode nebo název existující položky (i archivované)
je konflikt. Import nepřepisuje existující položky. Uživatel může konfliktní
či neplatné řádky přeskočit. Opakování a souběh nevytvářejí duplicity.
Importované položky mají audit. Nevznikají skladové karty ani pohyby.

XML je omezeno na 2 MB, 500 položek, 60 000 uzlů a hloubku 32.
DOCTYPE, nejednoznačné základní ceny a jiné deklarované kódování jsou odmítnuty.
Raw XML se neukládá; ukládají se normalizované řádky, stav a identita.
Migrace 050 rozšiřuje import_batch/import_identity o ceníkové položky.

## Ověření

Testy import.rs zahrnují náhled, opakování, souběh, konflikty, role, izolaci
profilů a REST/MCP shodu. money_catalog testuje parser a odmítnutí nejasných
vstupů. E2E import.spec.ts pokrývá CS/EN na desktopu a mobilu.
Syntetická fixture je explicitně označena. Volitelný test
published_vendor_fixture používá přesnou staženou ukázku výrobce přes
MONEY_VENDOR_FIXTURE; soubor se neukládá do repozitáře.

## P18-05c — Obecný CSV/XLSX import ceníku

Vlastní mezicesta při nedoloženém nativním formátu S4/S5 (E-059).
V Importu dat zvolit Jiný zdroj → Ceník. Používá existující parseImportTable
na backendu, ruční mapování a společné import_prepare/import_apply
s kind=catalog a source=generic. Nevyžaduje novou DB migraci.

Povinná mapovaná pole: stabilní zdrojové ID, název, jednotková cena, měna,
sazba DPH. Volitelné: kód, barcode, jednotka. Množství je 1.
Cena s/bez DPH se vybírá výslovně pro celý soubor, bez výchozí domněnky.
Desetinný oddělovač je nastavitelný (tečka/čárka); formát čísel se nehádá.
UI nepoužívá JS Number pro ceny. Odmítá smíšené oddělovače, skupiny tisíců,
měnové značky, procenta, exponenty a chybějící ceny. Neplatný číselný vstup
se do náhledu předává jako neplatná hodnota, nikoli nula. Původní tabulka
zůstává při mapování dostupná. Identifikátory se nečíslují a nulové prefixy
z textových buněk zůstávají zachované.

Kontrakt importního CatalogData přijímá surové řetězce měny a sazby:
náhled musí umět vrátit neplatný řádek. Produkční ceníkový kontrakt zůstává
striktní. Backend ověřuje CZK, sazby 0/12/21, přesnost a limity; zápis,
konflikty, oprávnění, audit a deduplikace jsou společné s P18-05a.
XLSX vzorce odmítá existující parser. Souborový vzor je
apps/web/public/catalog-import.csv; nejde o export Money.

Testy: catalog-import-mapping.spec.ts (přesné desetinné řetězce a neplatné
vstupy), import.spec.ts (CS/EN desktop/mobile zápis), serverový import.rs
(generic_catalog_invalid_rows_and_exact_values, neplatné řádky, přeskočení,
izolace profilu, uložené ceny a opakování).

## P18-05b — Uživatelské mapování a rozšíření místo neověřeného adaptéru

Rozhodnutí uživatele 18. 10. 2026: nedoložené konkrétní S4/S5 exporty může
provozovatel namapovat sám; implementace nečeká na jeho účet ani soubory.
Existující CSV/XLSX mapper doplňuje spustitelný lokální konfigurovatelný
XML adaptér `scripts/adapters/catalog_xml.py` a
[návod kontraktu rozšíření](../INTEGRATION_ADAPTERS.md). Výstup je existující
source=generic import_prepare s náhledem a potvrzením, nikoli přímý zápis.

Přesná pole, cenový režim, měnu a identity určuje uživatelská mapa. Duplicity,
chybějící/nejednoznačné ceny a neznámá měna se neodhadují. Syntetické fixtures
v tests/fixtures/adapters nejsou nativní export Money. Test Python3 plus
Rust fixture-contract test ověřují přesné ceny, ID s nulovými prefixy,
bezpečnost XML a použití autoritativního DTO/doménové validace. Nativní
shoda univerzálního formátu S4/S5 se nadále netvrdí (E-059); přijatý rozsah
je doložený adaptér nebo vlastní mapování/extension point.
