# Ověření P15-01 — přijaté faktury

13. 10. 2026. Reference: E-039 a specifikace
[received-invoices](../specs/received-invoices.md). Veřejná nápověda iDokladu
dokládá rozlišení interního a dodavatelského čísla, data, originál a částečné
úhrady. Nejde o potvrzení vizuální nebo numerické shody s živým iDokladem.

## Automatické kontroly

- PostgreSQL: 5 scénářů received; souběžné číslování a UUID, přesné zdrojové
  součty, úhrada/storno/zrušení, revize, role a izolace profilu, originál,
  REST/MCP a upgrade 024 → 025 se zachováním PDF i staré přílohy.
- Regrese sdílených vrstev: 3 attachment, 5 invoice_delivery a 5 invoice_share
  scénářů. Všechny testy používají izolované DB a případně loopback SMTP.
- Playwright: 8 scénářů, received a routing v CS/EN na desktopu a mobilu.
  Vytvoření a úprava, originál, částečná úhrada a storno, zrušení; axe a
  kontrola přetékání. Navigace zachovává DOM sidebaru a rozbalené skupiny.
- TypeScript, lokalizace, Prettier, tokenové hranice, rustfmt a Clippy prošly.
  Produkční web build prošel; zůstává upozornění na velikost sdíleného chunku.
- Opakované generování API klientů a MCP schémat nemění výstupy.

## Vizuální kontrola

Lokální syntetické screenshoty received-detail-{cs,en}-{desktop,mobile}.png
jsou ignorované Gitem. Ověřen monochromatický vzhled, profilová navigace,
formulář a úhrady. Tabulky mají na mobilu vlastní horizontální posun,
přístupný z klávesnice; neroztahují celou stránku.

Editor eviduje základy a přesnou DPH podle originálu. Výpočet DPH ze sazby,
cizí měny, přeplatky, účtenky, globální seznam výdajových plateb a nákladový
přehled nejsou dodané v P15-01. Zbývající rozsah je v implementačním plánu.

Celý Rust workspace: 22 testů prošlo, 107 databázových testů je standardně
ignorovaných (vybrané běžely samostatně výše). Build serveru prošel. Lokální
API bylo restartováno, readiness je OK a databáze má migraci 025.
