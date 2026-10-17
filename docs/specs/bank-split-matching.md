# P16-11 — rozdělení bankovního pohybu

Vlastní návrh Open Faktury, 14. 10. 2026. E-049: oficiální nápověda popisuje
výběr jednoho dokladu, nedokládá rozdělení ani serverová pravidla více vazeb.

Jeden CZK pohyb lze ručně rozdělit mezi vydané (příjem) nebo přijaté (výdaj)
faktury. Existující převodní úhrada, případně kartová úhrada účtenky, se připojí
celá bez vytvoření kopie. Nová platba používá společný platební příkaz a datum
pohybu; částka nesmí překročit zůstatek dokladu ani nerozdělenou část pohybu.
Částky jsou kladné desetinné řetězce, max. dvě desetinná místa, NUMERIC(14,2).
Nerozděleno = abs(pohyb) − součet aktivních vazeb; žádné zaokrouhlení ani kurz.
Přeplatek zůstává nerozdělený, nezvyšuje úhradu faktury a nic se automaticky nevrací.

`split` přijímá revizi pohybu a 1–25 allocations (druh/ID/revize dokladu,
volitelné ID existující úhrady, částka). V jedné dávce je doklad nejvýše jednou.
Vše se zapisuje atomicky; chyba vrátí celou dávku. Další dávkou lze doplnit
zbývající část pohybu. UUID a shoda vstupu chrání opakování, profilový zámek
souběh s ručním i automatickým párováním. Automatika částečné pohyby vynechá.

`split_candidates` vrací doklady pro částečnou platbu a celé existující úhrady,
částka `available` je jejich maximum. Konečné ověření probíhá při zápisu.
`unmatch` odpojí všechny aktivní vazby atomicky, povinný důvod. Stornuje pouze
úhrady vytvořené bankou; předchozí ruční úhrady a účtenky zůstanou zaplacené.
Částky vazeb zůstávají v historii. Oprava konkrétního rozdělení = odpojit a znovu
přiřadit. Původní `match` zachovává chování celé částky na jediný doklad.

Seznam rozlišuje nespárováno/částečně/spárováno. Detail ukazuje všechny vazby,
jejich částky a zbytek. Viewer čte, owner/editor zapisuje; archivovaný účet
nedovolí nové párování. CS/EN, žádné klientské účetní výpočty přes Number.
Zálohy, dobropisy, cizí měny a platební příkazy nejsou součást této změny.

Migrace 036 doplní částku historických vazeb z původní celé částky pohybu,
ruší pouze unikátnost jedné aktivní vazby na pohyb; unikátní platby zůstávají.
Obohatí uložené odpovědi idempotence podle jejich historických snapshotů,
nikoli podle dnešního párování. Před upgradem zastavit staré API.

Testy: příjem/výdaj, existující platba, desetinné zbytky, přeplatek, doplnění,
opakování/souběh, izolace profilu/role, archivace, chybná částka/revize,
rollback druhé položky, společné odpojení, upgrade035 a REST/MCP parity.

## Ověření 14. 10. 2026

Izolovaný PostgreSQL: `pnpm test:bank` 33 scénářů včetně šesti split testů,
`pnpm test:cash` 15, `pnpm test:payments` 6, `pnpm test:received` 9. Prošlo
rozlišení vytvořené/existující úhrady, příjem/výdaj, přesné zbytky, opakování,
souběh, rollback, role/izolace, REST/MCP a upgrade035 s novým připojením.
`pnpm test:e2e tests/e2e/bank.spec.ts --workers=1`: 4 scénáře CS/EN,
Chromium desktop/mobile, rozdělení 650,01 na 300 + 250,02 a zbytek 99,99,
odpojení a přístupnost. Kontrola screenshotů CS desktop a EN mobile.
`pnpm check`, webový a serverový build prošly. Po vyčerpání místa byly
neplatné běhy zopakovány po odstranění build cache a obnově lokálního Dockeru.
Syntetické testy nejsou potvrzením chování přihlášené aplikace iDoklad.
