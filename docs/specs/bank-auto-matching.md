# P16-14 — průběžné automatické párování

Návrh Open Faktury, 14. 10. 2026, navazuje na doložené VS/částky E-043
a konzervativní pravidla [hromadného párování](bank-batch-matching.md).
Nejde o potvrzenou úplnou shodu serverových pravidel iDokladu.

Automatika je oddělená od stahování bankovních dat. Owner/editor ji musí
zapnout pro konkrétní účet; funguje i pro importované a ručně evidované pohyby,
bez uložených bankovních přístupů. Nové i migrované účty ji mají vypnutou.
Aktér aktivace je uložen; worker opětovně ověřuje jeho práva. Ztráta oprávnění,
archivace účtu či smazání profilu automatiku vypne, návrat práv ji sám nezapne.

Worker zpracuje nejvýše 25 pohybů účtu v jedné transakci. Sdílený profilový
zámek chrání před souběhem manuálních i automatických plateb. Používá stejné
vyhodnocení a platební operace jako hromadné potvrzení: jediný kandidát,
normalizovaný VS, přesná částka a kontrola konkurenčních pohybů mimo dávku.
Nejednoznačné, částečné či přebytečné úhrady, chybějící VS a příliš mnoho
kandidátů se vynechají. Existující úhrady pouze připojí, nevytváří je znovu.

Pohyby s historií odpojeného párování se automaticky nevracejí na doklad.
Jejich nové párování vyžaduje ruční akci. Odpojení živého napojení zastaví
stahování; automatické párování je samostatný přepínač a zůstává dle své volby.

Kurzor postupně projde všechny stránky, vynechané pohyby tak neblokují další.
Další plná dávka může běžet po 30 s; po konci průchodu další kontrola za 5 minut.
Nové nastavení plánuje první kontrolu za 30 s. Neřešené pohyby se znovu posoudí
při dalším průchodu (například po vystavení odpovídající faktury).
Neúspěšná dávka se celá vrátí; bezpečný kód chyby a odklad o 5 minut se uloží
odděleně. Kurzor se při chybě neposune. Změna konfigurace zneplatní starého
kandidáta workeru revizí. Žádné síťové I/O není součástí párovací transakce.

`bank_connection_execute action=matching` nastavuje enabled s revizí a UUID.
Opakované UUID vrátí konflikt, UI může obnovit stav. `bank_connection_status`
vrací zapnutí, příští termín, chybu a souhrn poslední dokončené dávky.
Audit bank_matching_run uloží aktéra, revizi konfigurace a ID/status každého
pohybu; bank_match a společný platební audit zachovají návaznosti.
P14 může následně odeslat poděkování, jen pokud je již samostatně povolené.

Migrace 035 zachovává pohyby, platby, přístupy i plán synchronizace a nikomu
nezapíná párování. Testovat pouze v izolovaných DB. Scénáře: přesná shoda,
existující úhrada, nejednoznačnost mimo dávku, stránkování, odpojení bez
opětovného připojení, souběh, odebrání práv, rollback a upgrade034.

## Ověření 14. 10. 2026

Izolovaný PostgreSQL: `pnpm test:bank` (26 scénářů) a doplněný test
`matching_rest_mcp_configuration_and_status` (1 scénář). Ověřeno přesné
párování, připojení existující úhrady, souběh, odpojení, stránkování,
konkurenční pohyby, odebrání práv, transakční rollback a upgrade 034.
`pnpm test:e2e tests/e2e/bank.spec.ts --workers=1`: 4 scénáře Chromium
CS/EN desktop/mobile včetně skutečného workeru v izolované DB, přepnutí
a vypnutí automatiky a přístupnosti. `pnpm check`, webový i serverový build
prošly. Syntetické scénáře nepotvrzují numerickou ani úplnou UX shodu iDokladu.

## P16-13e (18. 10. 2026)

Worker a dávka podporují běžné faktury v měně účtu, včetně non-CZK.
Konkurenční pohyb má stejnou měnu; čísla v různých měnách nejsou srovnatelná.
Nemění opt-in, oprávnění ani zacházení s existujícími úhradami a odpojením.
[Podrobnosti a omezení](bank-currencies.md#p16-13e--dávka-a-automatika-ve-shodné-měně).

## P16-14b — volitelné zálohy a vratky (18. 10. 2026)

Vlastní konzervativní návrh, nikoli potvrzený algoritmus iDokladu.
`action=matching` přijímá `include_treasury` (výchozí false); stav vrací
`matching_treasury`. Migrace 057 zachová běžné párování a nové rozšíření
všem vypne. Revize a kontrola práv platí i pro změnu rozsahu.

Při explicitním zapnutí worker zahrne zálohy a vratky dobropisů ve stejné
měně účtu. Záloha používá vlastní VS, vratka VS původní faktury. Prázdný VS
a nejednoznačné kombinace běžných faktur, záloh či dobropisů se vynechají.
Vyžaduje přesnou částku; existující převodní úhrada má přednost před novou
platbou téhož dokladu. Kandidáti jiných dokladů se tím neodstraňují.
Společný treasury příkaz zachovává rezervace, DDPP a profilová oprávnění.
Odpojená vazba se automaticky neobnovuje; původní připojená úhrada se nestornuje.

Ruční návrhy a hromadný náhled nadále používají svůj dosavadní rozsah;
rozšíření je nastavení workeru. Různé měny záloh a vratek zůstávají otevřené.
Testy native_treasury pokrývají opt-in, existující úhradu, vratku, odpojení,
nejednoznačnost a upgrade056. REST/MCP konfiguraci pokrývá automation.rs,
CS/EN ovládání bank.spec.ts.
