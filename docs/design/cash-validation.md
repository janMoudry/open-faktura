# P16-04 — ověření pokladní evidence

14. 10. 2026, místní macOS, PostgreSQL z vývojového Compose. Testy používají
izolované databáze a syntetická data; nepřevádějí peníze a neodesílají zákazníkům.

- `DATABASE_URL=… cargo test --locked -p faktura-server --test cash --test bank --test payment --test received --test communication -- --ignored --test-threads=1`:
  **42/42** (cash 7, bank 13, payment 6, received 9, communication 7).
- Pokladní testy zahrnují přesné saldo přes více stránek, období, PP/VP,
  opakování a souběh, rollback číslování při chybě, neměnnost počátku, role,
  profilovou izolaci, storno, nové i existující hotovostní úhrady, účtenky,
  připojení jedné platby nejvýše jednou, REST/MCP a upgrade 028 → 029.
- `pnpm exec playwright test tests/e2e/cash.spec.ts tests/e2e/routing.spec.ts --workers=2`:
  **8/8**, čtyři pokladní průchody CS/EN na desktopu/mobilu a čtyři regrese
  trvalého layoutu. Vytvoření pokladny/příjmu, párování, odpojení, storno,
  saldo, jiný profil, Axe WCAG2 A/AA a horizontální přetečení stránky.
- `pnpm check`, `pnpm generate:api`, následné `pnpm typecheck` a
  `pnpm format:check` prošly. Server i web build prošly. Zůstává známé
  upozornění Vite na sdílený chunk přes 500 kB.
- Lokální API restartováno po zastavení předchozího procesu, migrace 029;
  `/api/ready` vrací ready.

Vizuálně zkontrolován [CS seznam desktop](cash-register-cs-desktop.png)
a [EN detail mobil](cash-detail-en-mobile.png). Další syntetické snímky:
`cash-{register,detail}-{cs,en}-{desktop,mobile}.png`.

Technické ověření nedokládá úplnou shodu s iDokladem. E-044 je veřejná nápověda,
nikoli experiment v aplikaci. Automatické generování při úhradách mimo pokladnu,
tisk/PDF, prodejky a daňové položky zůstávají mimo tento díl.
[Specifikace a zbývající rozdíly](../specs/cash-register.md).


## P16-05 — automatika a PDF, 14. 10. 2026

- `DATABASE_URL=… CASH_PDF_TEST_DIR=/tmp/open-faktura-cash-pdf pnpm test:cash`: **12/12**.
  Navíc volba hlavní pokladny, opakování bez zpětného doplnění, oprava/storno
  platby, změna účtenky cash → card, vypnutí automatiky, zákaz archivace hlavní
  pokladny, rollback platby při neplatném datu, ruční párování bez duplicity,
  uložený vystavitel PDF, revizní/jazyková cache, REST/MCP shoda PDF a upgrade
  029 → 030. Transportní test používá limit 10 MB pro PDF s vloženými fonty.
- Regrese `--test bank --test payment --test received --test communication`: **35/35**.
- `pnpm exec playwright test tests/e2e/cash.spec.ts tests/e2e/routing.spec.ts --workers=2`: **8/8**.
  CS/EN desktop/mobil: zapnutí automatiky, zdrojová cash úhrada, automatický
  doklad, stažení PDF, přepnutí jazyka, náhled, storno zdroje, profilová izolace,
  Axe a trvalý layout. Test neprovádí fyzický tisk.
- `pnpm generate:api`, `pnpm check` (včetně Clippy), server build a web build prošly.
  Vite nadále upozorňuje na sdílený chunk přes 500 kB.
- Vizuálně zkontrolovány renderované CS/EN PDF (Poppler, česká diakritika,
  původní vystavitel po změně profilu, částka a podpisová místa) a
  [CS desktop](cash-automatic-cs-desktop.png), [EN mobil](cash-automatic-en-mobile.png).
  Snímky obsahují pouze syntetická data.
- Staré lokální API zastaveno, nový proces aplikoval migraci 030; readiness ověřena.

Veřejná reference E-045 není potvrzení úplné vizuální nebo numerické shody.
Zůstává otevřené: částka slovy, konfigurace sestav, samostatná DPH, prodejky,
automatické zálohy/dobropisy a cizí měny. Automatika je výslovně opt-in a
staré platby se nepřevádějí. U starých 029 dokladů je vystavitel stav při upgradu.

## P16-06 — položka s DPH, 14. 10. 2026

- `cargo test -p faktura-domain cash`: 1 test s nezávislými syntetickými příklady
  net/gross, sazeb, znamének, haléřové hranice a odmítnutých vstupů.
- `DATABASE_URL=… cargo test --locked -p faktura-server --test cash --test bank -- --ignored --test-threads=1`:
  **28/28** (15 cash, 13 bank). Nově výpočet/náhled, profilové plátcovství,
  data, souběžné opakování, rollback číslování po chybě zápisu rozpisu,
  izolace, viewer, zákaz párování, saldo/storno, snapshot po změně profilu,
  PDF a upgrade 030 → 031 včetně zachování předchozí PDF cache. REST/MCP
  náhled vrací shodné hodnoty. Po doplnění filtru tax znovu 3/3 tax_tests.
- Cash + routing E2E: **8/8** CS/EN desktop/mobil; po úpravě označení S DPH
  znovu 4/4 cash průchody. Výběr net/gross, serverový náhled 100 + 21 = 121,
  uložení, saldo, detail s rozpisem bez párování, Axe a šířka mobilu.
- `pnpm generate:api`, následné generování cash kontraktu, `pnpm check`
  včetně Clippy a server/web build prošly. Vite hlásí známý chunk > 500 kB.
- Vizuální kontrola CS příjmového a EN výdajového PDF přes Poppler: rozpis,
  datum, sazba, součty, podpisy, bez překryvů. Syntetické soubory jsou
  v /tmp/open-faktura-cash-tax. Zkontrolován [CS desktop](cash-tax-cs-desktop.png)
  a [EN mobil](cash-tax-en-mobile.png), poslední CS snímek také po změně badge.
- Starý API proces zastaven, nový aplikoval migraci 031; `/api/ready` = ready.

Testy ověřují naše implementační pravidlo, nikoli numerickou shodu iDokladu.
Členění DPH, výkazy, prodejky a souhrnné zaúčtování zůstávají otevřené dle
specifikace. Běžná úhrada ani automatický pokladní doklad nemají vlastní tax.
