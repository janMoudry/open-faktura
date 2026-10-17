# ADVANCE-SETTLEMENT-01 / P13-03 — vyúčtování záloh

## Doložený postup a vlastní hranice

E-034: veřejná nápověda iDokladu popisuje vyúčtování plně uhrazené zálohy do
upravitelné faktury, automatické zahrnutí daňových dokladů a více záloh stejného
odběratele a měny. Podkladem není nová relace v přihlášené aplikaci; numerická
ani vizuální shoda není potvrzená.

V Open Faktuře se vyberou zálohy z detailu nebo uloženého návrhu faktury. Nový
návrh přebírá položky, odběratele a dodavatele, datum vystavení/DPH je dnešní
pražské datum, splatnost podle první zálohy. Vystavení je samostatná standardní
operace P07 v běžné řadě faktur, nic neodesílá. Finální PDF/e-mail používají
stejný uložený doklad jako ostatní faktury. Více záloh lze spojit, další plnění
přidat a vybrané zálohy z nevystaveného návrhu odebrat.

Příprava rezervuje všechny vybrané zálohy; jsou okamžitě nedostupné ostatním
návrhům. Po rezervaci nelze změnit zdrojové platby ani vystavit další DDPP.
Prázdný výběr na uloženém návrhu uvolní všechny zálohy a odstraní odpočty. Vydaná
faktura rezervace trvale drží. Opravy/dobropisy nejsou nahrazené mazáním vazeb.
Každý příkaz je idempotentní podle UUID, aktéra a celého vstupu; změna vstupu
se stejným UUID je konflikt. Revize chrání fakturu i všechny vybrané zálohy.

Vlastní hranice: CZK, nejvýše 20 záloh/500 položek, úhrada musí přesně odpovídat
částce proformy (přeplatky a částečné vyúčtování nepodporujeme). Shodný odběratel
je IČO + země, a pokud IČO není dostupné, stejné jméno a adresa; stejně se ověřuje
dodavatel. Daňový režim musí souhlasit. Data faktury nesmí předcházet žádné
zahrnuté platbě ani vystavenému DDPP. Výběr ukazuje 50 výsledků na stránku,
hledání podle čísla/popisu/odběratele. Výpočet identitu znovu ověří při uložení
faktury i jejím vystavení; preview neautorizuje budoucí zápis.

## Částky a neměnné daňové odpočty

`advances` má verzi `of-settlement-v1`. Snapshot obsahuje čísla/ID záloh a plateb,
všechny zahrnuté DDPP s jejich **uloženými** Totals, přijatou částku a identity.
Neodvozujeme DPH z dnešní sazby, kontaktu nebo poměru proformy. Celé uložené
základy/DPH jednotlivých skupin DDPP se odečtou od spočítaného plnění. Zaokrouhlovací
rozdíl z více DDPP se zachová (může vzniknout záporný haléřový základ a kladná daň).
Není zamlčen dorovnáním do nuly. Řádky plnění zůstávají oddělené od odpočtů.

Doplatek = cena konečného plnění včetně zaokrouhlení − skutečně přijaté zálohy.
Záloha bez DDPP snižuje pouze doplatek; nesnižuje základ a daň nového plnění.
Záporný doplatek je odmítnutý, neprezentujeme jej jako hotový opravný doklad.
Případný doplatek se hradí běžnou úhradou finální faktury; původní platby se
nekopírují do invoice_payment, takže přehled nepřičítá tentýž příjem podruhé.
Přehled nadále eviduje pouze úhrady běžných faktur, nikoli původní příjem záloh.

Při společném typu ceny/slevě a bez zaokrouhlení kopírujeme originální položky
včetně množství. Jinak připravíme po jedné souhrnné položce za každý původní
řádek s jeho uloženou cenou včetně DPH, bez nové slevy. Tato normalizace je vlastní
rozhodnutí; přenáší cenu, nikoli záruku shodného přepočteného základu plnění.
Součet původních zaokrouhlení je samostatná korekce plnění. Při úplném odstranění
odpočtů se tato korekce rovněž odstraní a návrh se vrátí k běžnému kalkulátoru;
před vystavením jsou nové částky znovu viditelné. Historické sazby 10/15 %,
cizí měny, vrácení přeplatku a dobropisy zůstávají mimo tuto dodávku.

## Schéma a integrace

Migrace 020: `invoice_draft.advances`, auditní snapshot a
`advance_settlement_link` s PK(company_id,advance_id) a složenými profilovými FK.
`advance_settlement_command` uchovává příkaz a výsledek (ID návrhu). Stávající
sales_document.invoice_id odkazuje na rezervovaný návrh nebo vydanou fakturu.
Příprava, vazby, revize a audit jsou atomické, zálohy zamykané v pořadí ID.
Před nasazením zastavit starý server, který nezná nové blokace.

REST/MCP: candidates, configure, preview používají stejnou aplikační vrstvu.
Čtení včetně PDF vyžaduje členství, změny owner/editor. Kontrakt
contracts/advance-settlement.openapi.json; pnpm generate:api generuje klienta
a MCP schémata. Vystavení a doručení zůstávají standardní invoice_issue/invoice_send.

Testy: `pnpm test:advance-settlements`, doména advance_settlement, regrese
advance/tax_receipt/invoice/payment/sales a CS/EN desktop/mobil v sales E2E.

Záznam provedených kontrol: [ověření P13-03](../design/advance-settlement-validation.md).
