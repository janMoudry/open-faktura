# P14-03 — validační záznam

13. 10. 2026. [Specifikace](../specs/invoice-sharing.md), reference E-038.

## Backend a kontrakty

- `pnpm check`: CS/EN, TypeScript, formát, tokeny, Rust fmt a Clippy prošly.
- `pnpm test`: 21 úspěšných testů; 102 integračních/live scénářů se standardně přeskakuje.
- Explicitní PostgreSQL regrese: 31 úspěšných testů (`invoice_share` 5,
  `invoice_delivery` 5, `share` 4, `attachment` 3, `communication` 7, `recurring` 7).
- Nové testy: souběh/replay, konflikt UUID, draft, role/profil, expirace,
  odvolání, smazaná firma, původní PDF, změna stavu úhrady bez zveřejnění
  interní poznámky, deduplikace/limit zobrazení, REST/MCP, same-origin,
  link e-mail bez PDF přílohy, zastavení zrušeného odkazu před SMTP,
  upgrade z 023 při zachování PDF a starého replay e-mailu.
- Regenerování kontraktů je deterministické; nový kontrakt a DB scénáře jsou v CI.
- Build webu i serveru prošel; existující společný webový chunk nad 500 kB
  zůstává optimalizační rezervou.

## Rozhraní

Playwright `tests/e2e/invoice-sharing.spec.ts`: CS/EN, desktop a mobil,
izolované DB a loopback SMTP. Vytvoření odkazu v detailu faktury, otevření
bez přihlášení, stažení PDF, obnovení, historie návštěvy, odeslání odkazu,
odvolání a nedostupnost v anonymním okně. Axe WCAG 2 A/AA veřejné stránky
bez nálezů, bez horizontálního přetečení. Mobilní anonymní kontext má
stejný viewport a dotykový režim jako přihlášená aplikace.

Lokální syntetické snímky `invoice-{public,share}-{cs,en}-{desktop,mobile}.png`
se necommitují. Ověřena hierarchie, zalamování a sdílené monochromatické
komponenty. Headless screenshot nedokládá systémový PDF viewer ani tisk;
stažení PDF je ověřené, byty vůči privátnímu PDF porovnává DB test.
Prohlížeč bez vestavěného PDF vieweru nedostává prázdný iframe, dostupné
zůstává stažení a otevření PDF. První opakování mobilního testu během Rust
kompilace překročilo 5s limit přípravy PDF; čekání je nyní omezené na 20s.

## Meze shody

Referenci dokládá veřejná nápověda, ne nové živé odeslání v iDokladu.
Expirace, tokeny, limity sledování a konkrétní monochromatická stránka jsou
naše rozhodnutí. Zobrazení není důkaz přečtení ani identity příjemce.
Automatické e-maily dál přikládají PDF. Testy neposílají skutečným zákazníkům.
