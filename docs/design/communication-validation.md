# P14-02 — validační záznam

13. 10. 2026. Specifikace: [upomínky a poděkování](../specs/invoice-communication.md),
reference E-037 (veřejná nápověda; bez nového odesílání v živé agendě iDokladu).

## Automatické kontroly

- `pnpm check`: překlady, TypeScript, Prettier, konvence tokenů, Rust fmt a workspace Clippy prošly.
- `pnpm test`: 21 testů prošlo, 97 DB scénářů se standardně přeskakuje bez explicitního integračního příkazu.
- Izolovaný PostgreSQL: 36 testů prošlo (`communication` 7, `attachment` 3,
  `delivery` 4, `invoice_delivery` 5, `payment` 6, `recurring` 7, `share` 4).
- Nové scénáře pokrývají souběžné enqueue, revize úhrad, částečnou/úplnou platbu,
  storno, pauzu, vypnutí konfigurace, odebrání práva zápisu, role, cizí profil,
  REST/MCP, minimální částku, interval, deset pokusů, nové vs. historické platby,
  CS/EN a upgrade 022 při zachování PDF, historie i plateb.
- Playwright: čtyři scénáře (CS/EN × desktop/mobile), pouze syntetická instalace
  a loopback SMTP. Ruční náhled/odeslání, historie, pauza faktury, potvrzení
  automatizace, persistence nastavení, nepovolené poděkování před úhradou,
  poděkování po úhradě. Axe WCAG 2 A/AA bez nálezů; bez horizontálního přetečení stránky.
- Build webu i serveru prošel. Web stále hlásí společný chunk větší než 500 kB;
  jde o známou optimalizační rezervu, ne selhání buildu.

## Vizuální kontrola

Lokální syntetické snímky `communication-{compose,settings,history}-{cs,en}-{desktop,mobile}.png`
jsou v tomto adresáři a podle pravidel repa se necommitují. Kontrolován desktopový
náhled, mobilní nastavení a historie. Zachované monochromatické tokeny,
sémantické barvy hlášení, sdílené dialogy/fieldy/switche; e-mail má jazyk dokladu.
Nápověda příloh faktury již nezmiňuje veřejný odkaz nabídky.

Při prvním E2E běhu bylo nastavení skryté u dosud nedokončeného profilu.
Nyní je samostatnou sekcí profilu a nezávisí na rozpracovaném formuláři identity.

## Meze potvrzené shody

Neověřovali jsme numerické ani interní schedulerové chování iDokladu živými
experimenty. Serverový worker, pauza faktury, automatické poděkování z ruční
úhrady, jeden adresát a PDF příloha jsou výslovně naše pravidla. Bankovní
párování, kontaktní pravidla, vlastní automatické šablony a veřejné odkazy
faktur zůstávají v checklistu. Testy neposlaly žádný e-mail skutečnému zákazníkovi.
