# ADVANCE-01 / P13-01 — zálohové faktury

Reference E-032: veřejná nápověda iDokladu. Vlastní první implementace tuzemských
CZK záloh; přesné soubory, numerická shoda a živý průchod iDokladu v této etapě
nejsou ověřené. Sdílený editor a seznam navazují na dřívější pozorování prodeje.

## Rozsah a hranice

- Prodej → Zálohové faktury: hledání, aktivní/archiv, filtry všechny/uhrazené/
  neuhrazené/po splatnosti, stránkování, detail a editor v CS/EN.
- Vystavit uloží samostatný proforma doklad, vlastní řadu `ZF{rok}{pořadí:04}`,
  kopii dodavatele/odběratele, položky, součty a PDF. Nejde o běžnou fakturu.
  Prázdný variabilní symbol se doplní číslicemi čísla zálohy.
- Uložení je explicitní vystavení; neukládáme neúplné návrhy záloh. Doklad
  vyžaduje popis, dodavatele, zákazníka, položky, datumy a kladnou částku.
- PDF CS/EN uvádí „Zálohová faktura / Proforma invoice“ a „Není daňovým dokladem /
  Not a tax invoice“. Neobsahuje datum zdanitelného plnění. Stažení lze otevřít
  a vytisknout v PDF prohlížeči. Hodnoty DPH jsou kalkulační, ne daňová evidence.
- Ruční plné/částečné platby, přeplatek a storno s povinným důvodem. Datum platby
  od vystavení do dneška podle Europe/Prague; bankovní převod/hotovost. Záznam
  platby nic neodesílá a nevytváří daňový doklad.
- P13-02: [daňový doklad k přijaté platbě](advance-tax-receipts.md) je dostupný z řádku úhrady. P13-03 vyúčtování zatím není dostupné.
  UI to uvádí; akce Vyúčtovat je disabled. Konverze zálohy přes sales_convert je
  odmítnuta, aby se nevytvořila běžná faktura bez odpočtu.
- E-mail, veřejné odkazy a vlastní přílohy záloh nejsou součástí této dodávky.
  Dosavadní quote/invoice adaptéry zálohu odmítnou. Koš/hromadné akce, nastavitelné
  řady a pokročilé intervalové filtry nejsou v první verzi záloh implementované.

## Vlastní pravidla

Číslování je oddělené od CN i běžných faktur, podle roku vystavení v profilu.
Číslo je přidělené transakčně pod profilovým zámkem; request_id váže aktéra,
druh dokladu a původní data. Retry stejného příkazu vrací původní doklad,
změněný vstup konflikt. Výpočty používají stávající Decimal `of-czk-v1`.

Do přijetí platby lze údaje upravit s revizí a auditní kopií, vygeneruje se nová
aktuální verze PDF. Při kladné evidované úhradě jsou úpravy a archivace blokované
(vlastní přísnější pravidlo). Po stornu všech plateb lze znovu upravit. Platbu s vydaným daňovým dokladem (P13-02) stornovat nelze. Archivace
je vratná, nemaže platby ani audit. Tato pravidla vyžadují rozšíření při P13-02/03.

Každá platba má neměnné údaje a idempotentní UUID, souběžné požadavky se
serializují a revize chrání před přepsáním. Stejné storno stejného aktéra a
stejného důvodu lze opakovat; jiné opakování je konflikt. Storno není refundace.
Součet plateb je aktualizovaný atomicky s historií a sales_audit. Úhrady nemění
PDF. Zálohy a jejich platby nevstupují do invoice_draft/invoice_payment ani
současného přehledu běžných faktur. Přeplatky lze vidět jako samostatný stav.

## Data, oprávnění a kontrakty

Migrace 018 přidává `sales_document.kind=advance`, uložené `advance_totals`,
`advance_pdf` a `advance_paid numeric(20,2)`, samostatnou advance_sequence a
advance_payment. Firemní FK váže každou platbu do stejného profilu a dokladu.
Historie obsahuje aktéra, čas, původní příkaz a metadata storna. Dosavadní
nabídky/šablony se nemění. Při nasazení zastavit staré procesy, aplikovat migraci
novým serverem a teprve potom zpřístupnit nový klient; staré klienty neznají druh advance.

Existující sales CRUD přes společnou aplikační vrstvu; nový kontrakt
`contracts/advance.openapi.json` pro PDF a úhrady. MCP advance_pdf,
advance_payments, advance_register a advance_reverse používají stejné operace.
Viewer čte PDF/detail/historii, owner/editor zapisují; správce instalace nesmí
obejít členství. Všechny cizí profily a ne-zálohové ID jsou odmítnuté.

## Ověření

`pnpm test:advances`: izolovaná DB, souběh/retry, oddělená řada, PDF, platby,
storno, role, REST/MCP a upgrade z 017. Regrese test:sales/delivery/shares.
`tests/e2e/sales.spec.ts`: český/anglický editor, vystavení, PDF, částečná úhrada,
storno a seznam na desktopu/mobilu. Výsledky ve
[validačním záznamu](../design/advance-validation.md).

P19-01i rozšiřuje tento základ o cizí měny s uloženým kurzem zálohy; bankovní úhrady jsou v měně dokladu, DDPP kurz přebírá. Omezení a testovací scénáře: [cizoměnové vyúčtování](advance-settlement.md#p19-01i--cizí-měny-s-jednotným-uloženým-kurzem).
