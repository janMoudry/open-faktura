# TAX-RECEIPT-01 / P13-02 — ověření 12. 10. 2026

[Specifikace a hranice](../specs/advance-tax-receipts.md), reference E-033.
Oficiální nápověda je podklad pro pracovní postup, nikoli důkaz numerické nebo
vizuální shody. Nebyla provedena nová relace v přihlášené agendě iDokladu.
Všechny testovací údaje a částky jsou syntetické.

## Backend a databáze

- `cargo test --locked -p faktura-domain tax_receipt`: prošel číselný scénář dvou sazeb (12/21 %), poloviční platby, haléřového zbytku a odmítnutí nulové/příliš přesné/přeplatkové částky. Očekávání jsou nezávislé explicitní částky.
- `DATABASE_URL=… cargo test --locked -p faktura-server --test tax_receipt --test advance --test sales --test invoice -- --ignored --test-threads=1`: prošla regrese záloh (3), nabídek (4) a faktur (7) i první čtyři nové integrační scénáře.
- Finální `DATABASE_URL=… pnpm test:tax-receipts` (odpovídající přímý cargo příkaz): všech 5 scénářů prošlo, včetně přidaného případu více úhrad a limitu celkové částky. Celkem 19 různých integračních scénářů v tomto kroku.
- Nové scénáře ověřují souběžné stejné vystavení, idempotenci a změněný příkaz, oddělené číslování, náhled bez zápisu, snapshot/PDF, role a cizí profil, souběh se stornem, neplátce, neplatné datum/revizi/jazyk, REST/MCP a upgrade z 018 s existující zaplacenou zálohou.
- `pnpm check`: CS/EN katalogy, TypeScript, formátování, konvence/tokeny, rustfmt a workspace Clippy prošly. Následné úpravy jsou popisek, nezalamování peněz a ekvivalentní zjednodušení výpočtu zbytku; znovu ověřeny locales/typecheck, doménový test a build.
- `pnpm build:web` a `cargo build --locked -p faktura-server` prošly. Web dál upozorňuje na velikost sdíleného chunku.
- Regenerace `pnpm generate:api` nezměnila žádný generovaný soubor. `git diff --check` bez chyb.

První kompilace testů odhalila nesprávně použitý testovací MCP helper a chybějící
dev závislost v doménové fixture (nahrazená typovaným literálem). První integrační
pokus neplátce měl neplatnou syntetickou sazbu 21 %; fixture opravená na 0 %.
Finální běhy prošly bez těchto chyb.

## UI a PDF

`pnpm exec playwright test tests/e2e/sales.spec.ts --workers=1`: všechny 4 scénáře
CS/EN × desktop/mobil prošly. Vystavení z úhrady, náhled 1 000 Kč → základ 826,45 Kč
+ DPH 173,55 Kč, popis, detail, PDF download, odkaz na zdroj, chybějící storno
chráněné úhrady, seznam a prázdný cizí profil. Automatická a11y kontrola seznamu
bez nálezů. Stávající nabídky, e-mail, veřejné odkazy a převod zůstaly v regresi.

Lokální ignorované screenshoty `docs/design/tax-{preview,detail,list}-{cs,en}-{desktop,mobile}.png`.
Vizuálně prohlédnut český detail na desktopu a náhled na mobilu; odstraněné
zalamování peněžních částek a neaktuální text o nedostupnosti daňových dokladů.
Po těchto drobných opravách znovu prošly oba mobilní scénáře (`--project=mobile`).
PDF `output/tax-receipt-{cs,en}-desktop.pdf` vyrenderované přes Poppler a vizuálně
zkontrolované v obou jazycích: číslo DDPP, vazba na zálohu, datum DPH, rozpis,
upozornění „již uhrazeno“, bez nové splatnosti. Fyzický tisk a Tauri nebyly ověřovány.

Testy používají jednorázové DB; e-mailová regrese pouze loopback SMTP capture.
Lokální vývojové API bylo restartované s migrací 019 a `/api/ready` vrací `ready`.
Žádná skutečná úhrada ani zákaznický doklad nebyly testy změněné.

E-mail/přílohy tohoto typu dokladu, automatizace, opravy a vyúčtování záloh
nejsou tímto ověřením pokryté ani deklarované jako implementované.
