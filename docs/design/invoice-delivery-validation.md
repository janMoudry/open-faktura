# P10-01 — ověření odesílání faktur

11. 10. 2026, E-029 / INVOICE-DELIVERY-01.

- `pnpm check`: překlady, TypeScript, formát, tokeny, Rust fmt a clippy prošly.
- `pnpm build:web`: prošlo; existující upozornění na velikost JS chunku.
- Integrační invoice_delivery + delivery + share: 12 scénářů prošlo v izolovaných DB.
- Invoice E2E: CS/EN × desktop/mobil, včetně retry po ztrátě odpovědi a jediné SMTP
  zprávy; quote E2E: stejné čtyři varianty pro regresi společného dialogu.
- Kontrola kontrastu čeká na dokončení vstupní animace dialogu; během fade animace
  axe naměřil přechodné barvy. Konečný stav nemá serious/critical nálezy.
- Lokální backend aktualizován, readiness OK, migrace 202610110015 aplikovaná.

Vizuálně prohlédnut český desktop/mobil a desktop historie. Screenshoty
`invoice-email-{cs,en}-{desktop,mobile}.png` a
`invoice-email-history-{cs,en}-{desktop,mobile}.png` zachycují syntetická data.
Mobilní dialog scrolluje při menší výšce; odeslání ověřeno browser scénářem.
Referenční dialog: ../research/screenshots/2026-10-11/idoklad-33-invoice-email.jpg.

Všechny testovací zprávy zachycené pouze loopback SMTP. Externí doručení ani
SMTP konkrétní instalace nebyly testovány. PDF příloha neposkytuje tracking přečtení.
