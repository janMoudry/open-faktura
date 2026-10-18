# EXPORT-01 — ověření 12. 10. 2026

- `cargo test -p faktura-infrastructure export --lib`: 1 test prošel. CSV escaping/formula neutralizace; nezávislé otevření XLSX přes calamine, původní text, úvodní nuly, žádné formule.
- `DATABASE_URL=… pnpm test:exports`: 2 izolované PostgreSQL scénáře prošly. Filtr hledání/stavu, všechny stránky, prázdný profil, cizí aktér, viewer, neplatné sloupce/řazení, limit 10 000 a REST/MCP download.
- `pnpm exec playwright test tests/e2e/contacts.spec.ts tests/e2e/invoices.spec.ts --workers=4`: všech 8 scénářů prošlo (CS/EN × desktop/mobil × kontakty/faktury), včetně stažení CSV a XLSX.
- `pnpm check`: lokalizace, TypeScript, formát, hranice UI/tokenů, rustfmt a workspace Clippy prošly. Po finální úpravě textu a rozložení znovu prošly locales/typecheck a web build.
- Produkční web build prošel; přetrvává upozornění na velký sdílený chunk.
- Regenerované exportní OpenAPI typy a MCP schéma mají shodný obsah; odkazy z README, checklistu a specifikace existují.

První E2E běh měl dva neúspěšné průchody mimo export během průběžných úprav UI (bootstrap potvrzení a otevření filtru). Finální běh nad stabilním UI prošel bez retry všech osmi scénářů.

Lokální screenshoty `docs/design/export-{contacts,invoices}-{cs,en}-{desktop,mobile}.png` nejsou commitované. Vizuálně zkontrolován český desktopový dialog faktur a mobilní dialog kontaktů; výběr ve dvou sloupcích, dlouhý obsah lze posouvat. Nativní Microsoft Excel ani fyzický desktopový download nebyly tímto krokem ověřeny.

Lokální API aktualizováno; readiness vrací ready. Testy používají izolované databáze, do skutečné agendy nepřidávají záznamy. Export nepotřebuje novou migraci.
