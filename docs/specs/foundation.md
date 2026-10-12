# Infrastrukturní kostra

Vlastní technické požadavky Open Faktury, bez tvrzení o shodě s iDokladem. Datum: 10. 10. 2026.

| ID | Požadavek | Ověření |
| --- | --- | --- |
| BASE-001 | Web a desktop používají stejné React UI a Tailwind tokeny | Web build, Tauri build ze stejného dist |
| BASE-002 | Rust server se připojí k PostgreSQL a aplikuje migrace | Lokální Compose, úspěšná úvodní migrace a ready endpoint |
| BASE-003 | Výpadek databáze nesmí být vykázán jako připravenost | Rust router test vrací 503 při nedostupném PostgreSQL |
| BASE-004 | Uživatel vidí spojení a může je znovu ověřit | Playwright proti skutečnému API; simulace výpadku a obnova |
| BASE-005 | Úvodní stránka funguje na mobilním rozměru | Playwright mobilní viewport bez horizontálního přetékání a vizuální kontrola |
| BASE-006 | Monorepo má připnuté závislosti a opakovatelné kontroly | pnpm frozen lockfile, Cargo.lock, rust-toolchain, check/test příkazy |

Scope: pouze health API, start aplikace a vývojové prostředí. Neobsahuje skutečná firemní data ani jejich autorizaci. Doménové crate zatím vyjadřuje pouze stav dostupnosti; účetní typy vzniknou podle referenčních specifikací. Worker se založí až s první skutečnou úlohou.

Podrobné spuštění a omezení: [Lokální vývoj](../DEVELOPMENT.md).
