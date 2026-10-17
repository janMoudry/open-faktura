# CREDIT-NOTE-01 / P13-04 — ověření 12. 10. 2026

[Specifikace](../specs/credit-notes.md), reference E-035. Veřejná nápověda
potvrzuje návaznost na fakturu, koncept, částečné opravy a samostatný zápočet.
Nové živé pozorování iDokladu ani přesná numerická/vizuální shoda nejsou doložené.
Všechna data automatických scénářů jsou syntetická; databáze jednorázové.

## Doména a PostgreSQL

- `cargo test --locked -p faktura-server --test credit_note`: 3 doménové testy
  prošly. Kumulativní opravy 0,01 Kč zachovají součet základu a daně; největší
  podporovaná položka nepřeteče; záporné zaokrouhlení zdroje 1,11 → 1,00 Kč
  nevytvoří zápornou kapacitu poslední položky 0,01 Kč. Peněžní alokace používá
  celočíselné haléře v i128 s kontrolou násobení, nikoli float.
- `DATABASE_URL=… pnpm test:credit-notes`: 5 integračních testů prošlo.
  Částečný dobropis 605 Kč, záporné PDF, zápočet/storno, ruční vratky/storno,
  souběžné vystavení, opakování požadavku, viewer a cizí profil, REST/MCP/CSRF,
  upgrade 020 s původním PDF a úhradou. Dobropis plně vyúčtované zálohy čerpá
  plnění 1 210 Kč s daní 210 Kč, nepoužije nulový finální daňový souhrn;
  DDPP a rezervace zůstávají zachované. Zápočet nezvyšuje cash příjem.
- Regrese `cargo test --locked -p faktura-server --test payment --test invoice
  --test tax_receipt --test advance_settlement --test invoice_delivery
  --test credit_note -- --ignored --test-threads=1`: všech 34 scénářů prošlo
  (včetně 5 nových výše). Test obnovy payment použil místní dev Compose.

`pnpm test` prošel: 19 běžných testů, 83 DB scénářů standardně ignorovaných;
relevantní databázové testy byly spuštěné samostatně výše.

## UI a PDF

`pnpm exec playwright test tests/e2e/credit-notes.spec.ts --workers=1`: 4/4
(CS/EN × desktop/mobil). Vytvoření z faktury, důvod a částečná částka, uložení,
vystavení, stažení PDF, zápočet, storno a změna nedoplatku původní faktury,
vratka, historie a přepnutí profilu. Axe kontrola seznamu bez serious/critical
nálezů. Screenshoty po návratu scrollu na začátek:
`docs/design/credit-{editor,detail,list}-{cs,en}-{desktop,mobile}.png` (ignorované).

PDF `output/credit-{cs,en}-{desktop,mobile}.pdf` vizuálně zkontrolované po renderu
Popplerem: diakritika, důvod opravy, zdrojová faktura a záporné základy/DPH/celkem
jsou čitelné. Screenshoty editoru, detailu na mobilu a seznamu desktopu zkontrolované;
opravena popiska stavu převzatá z nabídek a zarovnání částek v seznamu.
Mobilní seznam zachovává vodorovný posuv tabulky uvnitř panelu.
Fyzický tisk ani Tauri nebyly v tomto kroku ověřené.

## Kontroly a provoz

`pnpm check` prošel (CS/EN, TypeScript, Prettier, tokeny, rustfmt, workspace
Clippy). Po úpravě zaokrouhlení Clippy znovu prošel; po posledních popiscích
prošly TypeScript a katalogy. `pnpm build:web` a server build prošly. Webový
sdílený chunk stále hlásí upozornění nad 500 kB. Regenerace API nezměnila žádný
generovaný soubor. `git diff --check` bez chyb.

Lokální server byl zastaven a znovu spuštěn s migrací 021; `/api/ready` vrací
ready. Stávající doklady nebyly testovacími operacemi měněné. E-mail/přílohy
dobropisů, přímé opravy DDPP, vrubopisy a cizí měny nejsou součástí tohoto kroku.
