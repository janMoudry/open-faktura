# P16-01 — ověření banky

13. 10. 2026, macOS, PostgreSQL z dev Compose, Chromium desktop + mobilní emulace.
Podklad reference: E-041, veřejná oficiální nápověda iDokladu. Neproběhl živý
experiment bankovní agendy iDokladu; vizuální ani numerická shoda 1:1 není tvrzena.

## Implementované a ověřené průchody

- CZK účet, platný IBAN, úpravy/revize, archivace, neměnnost IBAN po pohybu.
- CSV náhled, shodné ID/obsah → duplicita, jiné částky téhož ID → konflikt,
  atomické odmítnutí souboru, souběžný import bez druhé kopie.
- Ruční příjem/výdaj, seznam a filtry, profilová izolace a viewer bez zápisu.
- Nová úhrada vydané/přijaté faktury v transakci s vazbou pohybu.
- Existující převodní platba a karetní úhrada účtenky bez druhého zaplacení.
- Odpojení se zdůvodněním: vlastní úhradu stornuje, dřívější zachová.
- Blokace samostatného storna propojené úhrady a změny propojené účtenky.
- Jednu úhradu nemohou současně získat dva pohyby; cizí doklad/profil odmítnut.
- REST/MCP shoda včetně aktualizace účtu přes MCP; ochrana původu požadavku.
- Upgrade 026 → 027 zachová existující platby a účtenky.

## Spuštěné kontroly

- `pnpm test:bank`: pět integračních scénářů; následně doplněný šestý scénář
  `one_existing_payment_cannot_be_matched_twice` spuštěn samostatně — vše prošlo.
- Izolované PostgreSQL regrese payment (6), received (9), communication (7) — prošly.
- `pnpm exec playwright test tests/e2e/bank.spec.ts tests/e2e/routing.spec.ts --workers=2`
  — osm scénářů CS/EN, desktop/mobile, včetně ručního pohybu, archivace a trvalého layoutu.
- Axe WCAG2 A/AA bez porušení na seznamu/detailu; bez horizontálního přetečení stránky.
- `pnpm check`, `pnpm test`, `pnpm build:web`, server build — prošly.
- Opakované `pnpm generate:api` zachovává bank klienta a MCP schémata beze změny.

Screenshoty obsahují jen syntetická data:
[seznam CS desktop](bank-register-cs-desktop.png),
[detail CS desktop](bank-detail-cs-desktop.png),
[seznam EN mobil](bank-register-en-mobile.png),
[detail EN mobil](bank-detail-en-mobile.png). Vizuálně zkontrolovaný seznam desktop
 a detail mobil; další varianty pokrývá běhový test.

Build hlásí známý informační limit velikosti sdíleného Vite chunku (>500 kB).
Nativní bankovní formáty, přímé připojení k bankám, automatické párování, dělení
pohybu a cizí měny nejsou součástí této dodávky. CSV fixture je vlastní formát,
nikoli reálný export banky. Testy nikdy neposílají peníze ani e-maily zákazníkům.

## P16-02 — 13. 10. 2026

Následující ověření rozšiřuje předchozí P16-01; všechny nové fixtures jsou
syntetické kontraktové příklady, nikoli exporty bank.

- `pnpm test:bank` s izolovaným PostgreSQL: **8/8**. Včetně nativního importu,
  opakování/souběhu, jiného účtu/profilu, konfliktu obsahu, mixu formátů,
  REST/MCP preview a návrhů shod, existující úhrady a nejednoznačnosti.
- `cargo test -p faktura-infrastructure bank_statement`: **4/4** skupiny pro
  camt, Fio, MONETA a RB. Ochrana XXE, hloubka XML, batch, přesnost, ID a účet.
- `pnpm exec playwright test tests/e2e/bank.spec.ts --workers=2`: **4/4**, CS/EN,
  desktop/mobile. Nativní camt náhled/import, duplicity, potvrzení doporučené
  shody, odpojení, ruční pohyb, archivace a izolace profilů.
- Axe WCAG2 A/AA bez porušení na importu, návrzích, seznamu a detailu.
- `pnpm check`: prošlo. Po úpravě délky nápovědy znovu locale/typecheck/format
  a všechny čtyři bankovní E2E scénáře — prošly.
- `pnpm test`: **27 prošlo**, 119 integračních scénářů standardně přeskočeno;
  osm bankovních bylo spuštěno samostatně proti izolovaným DB výše.
- `pnpm build:web` a server build prošly. Známé upozornění Vite na sdílený chunk.
- Lokální API restartováno, `/api/ready` vrací ready, migrace **202610130028**.

Vizuálně zkontrolováno:
[import CS mobil](bank-import-cs-mobile.png),
[návrhy CS desktop](bank-suggestions-cs-desktop.png).
Další uložené varianty: `bank-import-{cs,en}-{desktop,mobile}.png` a
`bank-suggestions-{cs,en}-{desktop,mobile}.png`. Na mobilu je dialog rolovatelný;
krátký název formátu doplňuje nápověda pro konkrétní banku.

Ověření anonymizovaných originálních výpisů zůstává otevřené. Zvláště MONETA
má záměrně omezený import s explicitním směrem. P16-02 nedodává připojení bank,
platební příkazy ani automatické potvrzování úhrad. [Specifikace](../specs/bank-statements.md).

## P16-03 — 14. 10. 2026

Uživatelem potvrzené hromadné párování zobrazené stránky, s náhledem a důvody
přeskočení. Testy používají syntetická data a izolované PostgreSQL databáze.

- `pnpm test:bank`: **13/13**. Nové scénáře ověřují nové i existující úhrady,
  výdaje, opakování požadavku, konkurující pohyby mimo stránku, částečné částky,
  zastaralý náhled, role, izolaci profilů, REST/MCP shodu, souběh a úplný rollback
  při chybě druhé platby.
- `cargo test --locked -p faktura-server --test payment --test received --test communication -- --ignored --test-threads=1`
  s testovacím `DATABASE_URL`: **22/22** (6 payment, 9 received, 7 communication).
- `pnpm exec playwright test tests/e2e/bank.spec.ts --workers=2`: **4/4**, CS/EN,
  desktop/mobile. Náhled, změna účtu mezi náhledem a potvrzením, odmítnutí
  zastaralého výsledku, obnovení a úspěšné potvrzení bez druhých úhrad.
- Axe WCAG2 A/AA bez porušení; bez horizontálního přetečení stránky.
- `pnpm check`, následné `pnpm typecheck`, `pnpm check:locales`,
  `pnpm format:check`, `pnpm build:web` a `cargo build --locked -p faktura-server`
  prošly. Build nadále upozorňuje na sdílený Vite chunk větší než 500 kB.
- Lokální API restartováno; `/api/ready` vrací ready. Nová migrace není potřeba.

Vizuálně zkontrolován kompaktní náhled
[CS desktop](bank-batch-cs-desktop.png) a [CS mobil](bank-batch-cs-mobile.png).
Uložené varianty: `bank-batch-{cs,en}-{desktop,mobile}.png`. Připravené shody
jsou první, přeskočené pohyby mají důvod; delší seznam se posouvá svisle.

Jde o vlastní konzervativní pravidla popsaná v E-043, nikoli o doloženou přesnou
kopii dialogu iDokladu. Funkce nepáruje na pozadí a nepotvrzuje částečné nebo
nejednoznačné shody. Ověření originálních výpisů jednotlivých bank zůstává
otevřené. [Specifikace](../specs/bank-batch-matching.md).
