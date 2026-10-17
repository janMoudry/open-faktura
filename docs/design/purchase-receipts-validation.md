# Ověření P15-02 — účtenky a výdajové přehledy

13. 10. 2026. Specifikace [purchase-receipts](../specs/purchase-receipts.md),
veřejná reference E-040. Živé vizuální porovnání s iDokladem není potvrzené;
přesné zdrojové částky, atomické platební události a metriky jsou náš návrh.

## Databáze a kontrakty

`DATABASE_URL=… pnpm test:received`: 9 scénářů prošlo na izolovaném PostgreSQL.
Zahrnují původní faktury a účtenky, UUID/souběh/revize, oddělené číslování,
opravu a zrušení bez dvojího výdaje, přesné součty přes 26 dokladů a dvě
stránky, hranice období, historické nezaplacené závazky, filtry/metody/storna,
role/cizí profil a REST/MCP shodu včetně vytvoření účtenky a obou přehledů.
Upgrade 024 zachová vydané PDF a přílohu; upgrade 025 zachová původní PF
řadu, starý UUID replay a originál. Regrese attachment: další 3 testy prošly.

## Uživatelský průchod

Playwright: purchase-receipts, received a routing — celkem 12 scénářů
CS/EN desktop/mobile. Nové testy vytvářejí izolovaný profil a DB:
účtenka bez dodavatele, originál, změna částky a metody s historií,
společný seznam s částečnou úhradou faktury, filtr hotovosti, přehled,
jiný prázdný profil a zrušení účtenky s odečtem z přehledu.
Axe WCAG2 A/AA a kontrola horizontálního přetékání prošly. Regrese sidebaru
zachovává DOM uzly a rozbalené skupiny při změně stránky.

Vizuálně ověřeny desktopové seznamy a přehled i mobilní přehled.
Syntetické lokální screenshoty receipt-detail-*, expense-payments-* a
expense-overview-* jsou ignorované Gitem. Finální průchod po sjednocení
radius tokenu a resetu dat při změně profilu opět prošel ve všech 4 variantách.

Produkční web build prošel (zůstává upozornění na velikost sdíleného chunku).
Opakované generování API/MCP nemění výstupy. Zbývající rozsah, zejména
bankovní/pokladní párování, cizí měny a daňové výstupy, je v plánu.

Závěrečné kontroly: pnpm check prošel (CS/EN, TypeScript, Prettier, tokenové
hranice, rustfmt a Clippy). Celý Rust workspace: 22 prošlých testů,
111 DB testů standardně ignorovaných; relevantní DB scénáře běžely zvlášť
výše. Server build prošel. Lokální API bylo restartováno, readiness je OK
a lokální DB má migraci 026.
