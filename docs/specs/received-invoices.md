# P15-01 — evidence přijatých faktur

13. 10. 2026. Reference E-039; nejde o potvrzení numerické ani vizuální shody.

## Rozsah a vlastní pravidla

Evidence běžných tuzemských závazků v CZK: dodavatel uložený jako snapshot,
volitelná vazba na kontakt stejného profilu, číslo dodavatele, VS, bankovní účet,
vystavení/přijetí/DUZP/splatnost, popis a poznámka. Při prvním uložení vznikne
interní číslo PF + rok přijetí + šestimístné pořadí v profilu. Číslo dodavatele
je samostatné; jeho opakování není důkaz duplicity a není unikátním klíčem.

Částky se přepisují z originálu: nejvýše 100 řádků s popisem, sazbou, základem
a přesnou DPH. P15-01 sazbu pouze eviduje, DPH nepřepočítává. Desetinná
aritmetika, nezáporné částky s nejvýše dvěma desetinnými místy, nejvýše
999999999999.99 za celý doklad. Zaokrouhlení zvlášť v rozsahu -1 až 1 Kč,
celkový závazek musí být kladný. Náhled i uložení používají stejný Rust výpočet.
Daňový rozpis není výpočet nároku na odpočet. Reverse charge, cizí měny,
quantity/unit-price editor, ceník a daňové výstupy patří do dalších kroků.

Stav evidence active/cancelled je oddělený od úhrady unpaid/partial/paid.
Ruční platba je kladná, nejvýše zbývající závazek; přeplatky nyní odmítáme.
Hotovost ani bankovní metoda samy platbu nevytvářejí. Platba je evidence již
provedeného výdaje, nikdy bankovní příkaz. Storno platby vyžaduje důvod,
původní událost zůstává a zůstatek se obnoví. Zrušení dokladu vyžaduje důvod
a nulové aktivní platby. Zrušený doklad zůstává v historii, nelze ho upravovat.
Obsah dokladu s aktivními platbami nelze měnit; nejprve jejich storno.
Nákupní platby nikdy nevstupují do invoice_payment ani do příjmů P08.

Přílohy/originál používají existující souborovou vrstvu, autorizaci a limity
P10-03. Stažení je originál nahraného souboru, nevytváříme náhradní fakturu.
Připojení originálu je doporučené, nikoli podmínka uložení. Archivovaný doklad
umožňuje čtení, nikoli upload/mazání příloh. Samostatná dodavatelská role
adresáře, účtenky, seznam všech výdajových plateb, tisk vlastní rekapitulace,
e-mail účetnímu a bankovní párování nejsou součástí P15-01.

## Konzistence a API

Migrace 025: received_invoice, received_series, received_command, received_audit.
Částky NUMERIC(14,2); obsah JSONB, platby append-only v auditovaném JSONB.
Složené FK pro profil a kontakt, číslo unikátní v profilu. Každý zápis ověří
editor/owner, zamkne profil a revizi; UUID je unikátní v profilu a vrací uložený
výsledek stejného příkazu. Konfliktní UUID nebo revize vrací 409. Reader čte,
instalační správce neobchází členství. Audit ukládá aktéra, čas a snapshot.

REST/MCP sdílejí received příkazy create/command/get/list/preview. Seznam má
25 řádků, textové hledání a filtry all/unpaid/paid/overdue/cancelled; overdue
se počítá z kalendářního dne Europe/Prague. Řazení: přijetí a ID sestupně.
CS/EN UI používá stejný seznam/detail/editor a komponenty jako ostatní agendy.

## Ověření

Izolovaný PostgreSQL: souběh/UUID/revize, číslování, přesné součty, částečná
platba a storno, zrušení, role/cizí profil/kontakt/příloha, REST/MCP a upgrade
024. Playwright: CS/EN desktop/mobile, vytvoření, editace, originál a úhrada.

Zrušený doklad má zbývající závazek nula; původní částky zůstávají uložené.
Výsledky kontrol: [ověření P15-01](../design/received-validation.md).
