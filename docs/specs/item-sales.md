# P18-04 — Přehled prodeje po položkách

Důkaz E-057: veřejná nápověda iDokladu, nikoli potvrzená numerická shoda.
Přehled pod Prodej nabízí období, typ dokladu, hledání názvu/kódu, sloupce,
CSV/XLSX a otevření zdrojů vybrané položky. Vše CS/EN a v aktivním profilu.

## Vlastní pravidla Open Faktury

- Read-only DB projekce `item_sales_line` (migrace 049) z uložených řádků
  vystavených faktur, aktivních prodejek a vystavených dobropisů. Návrhy,
  stornované prodejky, nabídky, zálohy a ručně psané řádky bez catalog_id se nezahrnují.
- Ceníkové služby bez skladové karty jsou zahrnuté. Archivace položky historii
  nesmaže. Název/kód v souhrnu je současný; zdroje ukazují původní popis.
- Datum je datum vystavení, krajní dny včetně. Bez období jsou všechny prodeje.
  Filtr typu dokladu je invoice/retail/credit_note; bez filtru všechny tři.
- Částky jsou součty uložených line totals po všech slevách. Nejde o aktuální
  ceníkové ceny ani cash-flow. Vyúčtování používá řádky plnění před odpočtem záloh;
  globální zaokrouhlení dokladu není položka a nerozděluje se.
- Dobropisy přidávají záporné uložené částky. Množství snižuje pouze explicitní
  return_quantity; čistá cenová oprava počet kusů nemění.
- Souhrn se seskupuje catalog_id/jednotka/měna: různé jednotky/měny se nesčítají.
  Peněžní součty jsou oddělené po měnách a přes všechny stránky. Množství se
  globálně nesčítá. PostgreSQL numeric, žádné výpočty přes JS Number.
- 25 řádků na stránku souhrnu i zdrojů; stabilní řazení. Limit 10 000 skupin
  platí i pro export. Při překročení explicitní chyba, žádný oříznutý výsledek.
- REST a MCP používají společný store a firemní autorizaci (včetně viewer).
  Repeatable-read transakce pro souhrn, součty a zdroje. Export stejných filtrů
  přes všechny stránky, CSV ochrana vzorců a textové XLSX ze sdíleného encoderu.
- Sloupce se ukládají v prohlížeči; export vždy obsahuje všechny věcné sloupce.

## Akceptace

Dvě prodané ceníkové položky/služby, odlišná prodejní cena a sleva: souhrn
souhlasí s uloženými řádky, nikoli novou cenou v ceníku. Volný řádek a návrh
chybí. Dobropis s vratkou snižuje množství/částku, cenová oprava jen částku.
Prodejka se započítá jednou i po zaúčtování, po stornu zmizí. Období, typ,
archivace, oddělené jednotky, prázdný výsledek, profilová izolace, stránkování,
REST/MCP a export musí mít regrese. UI CS/EN desktop/mobile.

## Ověření 17. 10. 2026

`pnpm test:stock`: 12 databázových testů včetně souhrnu 28 skupin, stránkování
zdrojů, exportu všech stran, slev, archivace, cenové opravy, množstevní vratky,
storna prodejky, izolace a REST/MCP. `pnpm check`, typecheck po závěrečné úpravě
UI a `pnpm build:web` prošly. `catalog.spec.ts` prošlo v CS/EN na desktopu/mobilu;
EN desktop byl zopakován po 503 ve starším periodickém přehledu před novým scénářem.
Screenshoty jsou syntetické, nikoli důkaz shody s přihlášeným iDokladem.
