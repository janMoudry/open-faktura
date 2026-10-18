# Vlastní integrační adaptér

Rozhodnutí uživatele 18. 10. 2026: podporovat doložené nativní adaptéry;
u ostatních zdrojů umožnit vlastní mapování a rozšíření provozovatelem.
Není nutné čekat na konkrétní uživatelův Money S4/S5 export. Konfigurovatelný
adaptér je otevřený bod rozšíření, nikoli prohlášení o univerzální kompatibilitě.

## Hotové vstupy a výstupy

- CSV/XLSX: v Importu dat zvolit Jiný zdroj → Ceník, namapovat přesné sloupce,
  zvolit cenu s/bez DPH a desetinný oddělovač. Náhled a potvrzení používají
  společné příkazy a deduplikaci. Vzor `apps/web/public/catalog-import.csv`.
- Nativní Money S3: doložený omezený UTF-8 XML adaptér podle
  [specifikace](specs/catalog-import.md). Nepřejmenovávat jej na S4/S5.
- Konfigurovatelné XML (včetně vlastního exportu Money ERP): spustit níže
  uvedený lokální adaptér s mapou konkrétních exportovaných polí.
- Elektronické vystavené faktury: [ISDOC 6.0.2](specs/isdoc-export.md),
  REST GET `/api/companies/{company}/invoices/{id}/isdoc`, MCP invoice_isdoc.
  Přijetí konkrétní verzí účetního systému ověřuje autor jeho adaptéru.

## Spustitelný příklad XML ceníku

```sh
python3 scripts/adapters/catalog_xml.py \
  --input tests/fixtures/adapters/synthetic-user-catalog.xml \
  --mapping tests/fixtures/adapters/synthetic-user-catalog.mapping.json \
  --agenda user-defined-test \
  --request-id 00000000-0000-4000-8000-000000000001 \
  --output /tmp/catalog-prepare.json
```

Příklad je **syntetický uživatelský formát**, nikoli export Money S4/S5.
Výsledný JSON lze poslat jako tělo REST POST
`/api/companies/{company}/imports` nebo obsah předat MCP import_prepare
s company_id vybraného profilu. To vytváří pouze náhled, nikoli položky.
Výstup uchovávejte soukromě; skript sám nevytváří relaci ani nevolá síť.
Po kontrole stavů řádků potvrdit existující import_apply/POST
`/imports/{batch}` se seznamem indexů skip. Backend znovu ověří profil,
roli, pole a konflikty; opakování nikdy neřešit vypnutím jeho kontrol.

Pro skutečný zdroj zkopírovat mapu a nahradit přesné cesty a explicitní
konstanty. record_path je relativní cesta od kořenového elementu k záznamům;
fields obsahuje přímé cesty k textovým listům nebo `@atribut` či
`VnorenyElement/@atribut`. Nesmí být nejednoznačné. Cesty nejsou XPath
program: žádné `//`, predikáty, XSLT, externí dokumenty ani automatické
odhadování prvního nalezeného cenového pole. Namespacované exporty nejprve
transformovat vlastním verzovaným lokálním adaptérem, nikoli odstraňovat
namespace odhadem v produkčním serveru.

Povinné mapování: key, name, unit_price, currency, vat_percent. Volitelné
code, barcode, unit; množství je 1. price_mode=net/gross a decimal_separator
musí být uvedené. Chybějící cena není nula, další cenová hladina není
základní cena. CZK může být explicitní konstanta jen tehdy, pokud význam
konkrétního exportu ověřil autor mapy. Ceník podporuje CZK a sazby 0/12/21;
nepodporovaná měna se nepřepočítává bez zvoleného pravidla.

Skript zpracuje nejvýše 2 MiB, 500 záznamů, 60 000 XML uzlů, hloubku 32.
Povoluje UTF-8, zakazuje DTD/entity. Peníze jsou desetinné řetězce, žádný
float; ceny nepřijímají exponenty ani oddělovače tisíců. Nulové prefixy ID
se zachovávají. Výstupní soubor vytváří výlučně, nepřepisuje existující.
Backendová validace je autoritativní; pomocný skript ji nenahrazuje.

## Veřejný kontrakt rozšíření

Autoritativní vstup je Prepare/Record v `contracts/import.openapi.json`,
generované MCP schéma a klient. Příklad normalizované dávky je
`tests/fixtures/adapters/synthetic-catalog-prepare.json`; Rust test
user_configured_adapter_fixture_matches_shared_prepare_contract ověřuje
stejný DTO a doménovou validaci. Vlastní adaptér může být libovolný lokální
program, který vytvoří tento JSON. Není nutné přidávat plugin s přístupem
k databázi ani spouštět cizí kód v API serveru.

source=generic je záměrné. agenda je stabilní identita zdrojové agendy;
key je stabilní identita položky. Nečíslovat řádky podle současného pořadí,
nepoužívat dnešní název jako identitu a neměnit agendu při opakování.
Stejný obsah je duplicita, změněný konflikt. Jiný nativní source se přidává
až s verzovaným mapováním a doloženým zdrojovým kontraktem.

U účetního výstupu je vstupem vlastní uložený snapshot a příslušný read API,
nikoli přepočet ze současného ceníku nebo přímý SELECT databáze. Adaptér
musí explicitně odmítnout nepodporovaný typ (DDPP, dobropis, odpočty záloh,
reverse charge) a zachovat desetinné částky, měnu, kurz a vazby. Neznámý
režim se nesmí tiše převést na běžnou domácí fakturu.

## Ověření vlastní mapy

Přidat anonymizovaný nebo syntetický vstup s jasným označením původu,
verzi aplikace a nastavení exportu, mapu a očekávaný normalizovaný JSON.
Ověřit dvě položky, základní i jinou cenu, nulovou a chybějící cenu,
textové ID s nulovými prefixy, duplicitu, změnu obsahu a oddělení profilů.
Pro účetní výstup navíc ověřit XSD verze cíle a skutečné přijetí v testovací
agendě; syntetická fixture ani validní XML samy tento poslední krok nepotvrzují.

```sh
python3 -m unittest discover -s scripts/adapters -p 'test_*.py'
cargo test -p faktura-server --test import user_configured_adapter_fixture
```

Money ERP veřejná dokumentace verze 1.17.1 popisuje konfigurovatelný výběr
objektů/polí a XSLT, nikoli jeden univerzální ceníkový tvar:
https://help.moneyerp.com/versions/1.17.1/am_export_do_xml.htm.
Důkaz E-059 proto zůstává přesně vymezený. Provozovatel může použít vlastní
CSV/XLSX mapu nebo tento extension point bez čekání na nativní adaptér.
