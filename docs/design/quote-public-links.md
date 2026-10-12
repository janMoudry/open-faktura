# Veřejné nabídky — ověření 11. 10. 2026

Reference: E-028 v ../research/EVIDENCE.md. Jde o vlastní monochromatickou
adaptaci zdokumentovaného chování; příjemcovský náhled iDokladu nebyl živě
porovnán. Všechny snímky Open Faktury obsahují syntetická data z dočasných DB,
po testu odstraněných. Zobrazené tokeny proto nic nezpřístupňují.

## Snímky Open Faktury

- quote-public-cs-desktop.png a quote-public-en-desktop.png
- quote-public-cs-mobile.png a quote-public-en-mobile.png
- quote-shares-cs-desktop.png a quote-shares-en-desktop.png
- quote-shares-cs-mobile.png a quote-shares-en-mobile.png

Veřejné náhledy zkontrolovány na desktopu i mobilu. Správa odkazů používá
stejné sekce, pole, tlačítka, dialogy a tokeny jako detail nabídky.
Celostránkové snímky správy zachycují pevnou navigaci v pozici viewportu
po posunu; nejde o referenci jejího celostránkového umístění.

## Ověřeno

- pnpm check: CS/EN/interpolace, TypeScript, formát, tokeny, Rust fmt/Clippy.
- pnpm build:web: produkční build; existující upozornění na velikost hlavního
  sdíleného balíku zůstává.
- 12 integračních testů PostgreSQL: sales, delivery a share; všechny prošly.
- 4 E2E scénáře sales.spec.ts: CS/EN × desktop/mobil, prošly.
- Anonymní kontext bez cookies, PDF download, přístupné veřejné UI (axe bez
  serious/critical), přesně jedna návštěva i po stažení PDF, druhá po reloadu,
  historie, lokální SMTP odkaz bez přílohy, zrušený odkaz bez obsahu.
- Migrace 013→014 zachová historickou PDF přílohu a idempotentní replay.
- Lokální dev DB má migraci 014 a API /api/ready vrací ready.

Příjemce se neidentifikuje; historie je technické otevření náhledu. Tracking
má popsanou deduplikaci a limit, není potvrzením čtení. SMTP testy používají
pouze loopback capture, žádní zákazníci nebyli kontaktováni.
