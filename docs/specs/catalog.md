# P12-01 — Ceník zboží a služeb

Podklad E-024: živý seznam a nový editor iDokladu, neplátce, 11. 10. 2026.
Referenčně pozorovaná pole: název, katalogové číslo, EAN, prodávat po, jednotka,
cena, měna; sklad je samostatná volba s počátečním pohybem. Uložení a numerická
pravidla reference dosud neověřena. Následující validační a transakční pravidla
jsou vlastní návrh Open Faktury, nikoli tvrzení o vnitřní implementaci iDokladu.

## Rozsah a pravidla

- CAT-001: seznam, hledání názvu/kódu/EAN, detail, založení, úprava, archiv/obnova
  pod aktivním profilem; vlastník/editor zapisují, viewer čte. Nečlen nečte.
- CAT-002: opakované založení se stejným UUID a normalizovaným obsahem vrátí
  stejnou položku. Jiný obsah/autor je konflikt. Úprava vyžaduje aktuální revizi.
- CAT-003: audit v téže transakci. Existující položky se při archivaci nemažou.
- CAT-004: výběr do faktury vytvoří nezávislou kopii názvu, množství, jednotky,
  ceny a sazby. Změna ceníku nepřepíše rozpracovaný ani vystavený doklad.
- CZK, množství > 0 do 1 000 000, cena >= 0 do 1 000 000 000; čtyři desetinná
  místa, JSON řetězce. Název 1–200 znaků, kód/EAN do 80, jednotka do 20.
  EAN není v této etapě ověřován jako GTIN; slouží jako textový vyhledávací údaj.
- Sazby 0/12/21 a režim net/gross jsou kompatibilní se současným kalkulátorem.
  Převod jednotkové ceny provádí Rust Decimal, čtyři místa, midpoint away from
  zero. Neplátce přebírá zadanou cenu bez přepočtu a sazbu 0. Převedená cena
  nad limitem se odmítne. Jiné měny a historické sazby navazují v P12-04.
- Ceníkový výběr neprovádí žádný pohyb zásob. Sledování skladu není zatím aktivní;
  P18 zavede vazbu řádku dokladu na skladovou položku a samostatný audit pohybů.

## Model a migrace

Migrace 011 pouze přidává catalog/catalog_audit. API vrací bigint ID jako řetězec,
revizi, archived a data. Všechny čtecí/zápisové operace autorizují členství,
transakce drží sdílený zámek firmy/členství proti souběžnému odebrání přístupu.
Starší aplikace nová data nečte; existující tabulky ani doklady se nepřepisují.

## Akceptace

Izolovaná PostgreSQL: souběžný create bez duplicit, změněný klíč odmítnut,
revizní konflikt, archiv/obnova, audit, cross-profile a viewer/outsider.
REST ověří session/Origin; MCP stejné příkazy. UI CS/EN desktop/mobile:
založit → reload → upravit → vložit do faktury; cena a množství zůstávají
textové, změna ceníku nezmění uložený návrh. Nikdy nezaměnit chybějící server
za prázdný ceník.

## Ověření 11. 10. 2026

- `DATABASE_URL=… pnpm test:catalog`: 4 úspěšné PostgreSQL scénáře, včetně
  upgrade 010→011, souběhu a REST/MCP převzetí položky.
- `cargo test -p faktura-domain catalog`: 2 úspěšné scénáře desetinné přesnosti,
  převodů, záporných/nečíselných vstupů a limitů.
- `pnpm exec playwright test tests/e2e/catalog.spec.ts tests/e2e/invoices.spec.ts --workers=1`:
  8 úspěšných scénářů CS/EN desktop/mobile. Izolované databáze a syntetická data.
  Ceník: validace, reload, konflikt se zachováním vstupu, hledání, profilová
  izolace, archiv/obnova, viewer a uložený návrh nezávislý na pozdější ceně.
- Snímky `docs/design/catalog-{editor,list,picker}-{cs,en}-{desktop,mobile}.png`.
  Axe kontrola detailu bez serious/critical nálezů; nejde o úplný audit přístupnosti.
- Lokální server aktualizován, readiness OK, ceník otevřen i v existujícím
  uživatelském profilu bez zápisu testovacích dat.
