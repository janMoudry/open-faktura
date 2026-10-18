# P16-12 — Bankovní a pokladní vazby záloh a dobropisů

Stav: implementováno, 15. 10. 2026. Navazuje na specifikace advance-invoices,
advance-tax-receipts, advance-settlement, credit-notes, banking a cash-register.
Nejde o nově pozorované chování iDokladu; níže jsou naše pravidla společné
platební vrstvy. Referenční podklady jednotlivých dokladů zůstávají v jejich
specifikacích. Numerická ani úplná interakční shoda zde není potvrzená.

## Pravidla

Příchozí pohyb lze spojit se zálohou, odchozí s vrácením dobropisu.
Částka je kladná v platební operaci a znaménková v evidenci banky/pokladny.
Banka podporuje shodnou měnu účtu a dokladu (P16-13f); pokladna zůstává CZK.
Převody rozdílných měn záloh a vratek zatím nejsou podporované. V bankovním rozdělení lze spojit
více typů dokladů; identitu tvoří typ i ID, protože ID různých tabulek se překrývají.

Vytvoření úhrady používá společné operace advance/credit, nikdy invoice_payment.
Připojení existující aktivní platby ji nesmí vytvořit znovu. Metoda musí odpovídat
bance nebo pokladně. Platba smí mít právě jednu aktivní vazbu napříč evidencemi.
Změna a autorizace probíhají pod profilem, s kontrolou revize a v jedné transakci.

Odpojení ruší pouze platbu vytvořenou párováním. Ruční zrušení spárované platby
mimo evidenci se odmítne. DDPP a rezervace vyúčtování mají přednost: pokud blokují
storno, odpojení se celé vrátí zpět, včetně vazeb ostatních částí pohybu.
Historie si ponechá typ, číslo a částku původního přiřazení.

Automatické návrhy podle VS zatím zůstávají omezené na původní podporované typy;
přidání kandidátů ručního párování samo nezapíná automatické zápisy.

## Akceptace

- Příjem zálohy a vrácení dobropisu v bance i pokladně, včetně zůstatků.
- Existující platba bez zdvojení; odpojení ji zachová.
- Vytvořená platba se při odpojení stornuje; DDPP/rezervace vrátí konflikt.
- Revizní konflikt, opakování příkazu, oddělení profilů a oprávnění.
- Smíšené rozdělení banky a přesný nerozdělený zbytek.
- REST/MCP, CS/EN, odkazy na správný typ dokladu a historie.
- Upgrade z migrace 036, regrese advance/credit/bank/cash.

Migrace 037 rozšiřuje profilové cizí klíče; před upgradem zastavit starý server.

## Ověření implementace

`tests/bank/treasury.rs` ověřuje příjem zálohy, vratku dobropisu, profilovou
izolaci, replay, zákaz storna mimo evidenci, existující pokladní úhradu bez
zdvojení a atomický rollback smíšeného rozdělení s DDPP. Regrese: bank, cash,
advance, credit_note, tax_receipt a advance_settlement. CS/EN bank.spec.ts
kontroluje párování zálohy a odkaz na správný detail na desktopu i mobilu.
Automatika pokladních dokladů z úhrad záloh/dobropisů zůstává samostatnou mezerou;
tato dodávka ji nezapíná.
