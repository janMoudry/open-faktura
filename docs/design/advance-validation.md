# ADVANCE-01 / P13-01 — ověření 12. 10. 2026

Implementovaný rozsah a odchylky: [specifikace](../specs/advance-invoices.md).
Veřejná dokumentace iDokladu je evidovaná jako E-032. Tento krok neobsahoval
experiment v přihlášené referenční agendě; technické testy nepotvrzují přesnou
numerickou ani vizuální shodu s iDokladem. Testovací údaje jsou syntetické.

## Automatické kontroly

- `DATABASE_URL=… cargo test --locked -p faktura-server --test advance --test sales --test delivery --test share -- --ignored --test-threads=1`: 15 scénářů prošlo (3 advance, 4 sales, 4 delivery, 4 share).
- Zálohy: souběžné vystavení a opakování příkazu, oddělená číselná řada, uložené PDF, částečná úhrada a přeplatek, storno a retry, konflikt revize, blokace úprav zaplaceného dokladu, cizí profil, role, REST/MCP a upgrade z migrace 017 bez ztráty nabídky. Platby záloh nevstupují do běžných úhrad faktur.
- `pnpm exec playwright test tests/e2e/sales.spec.ts --workers=1`: všechny 4 scénáře prošly i po finálních opravách, CS/EN × desktop/mobil; vytvoření, vystavení, stažení PDF, částečná úhrada, storno a seznam. Součástí scénářů je regrese šablon, nabídek, převodu, e-mailu a veřejného odkazu.
- `pnpm check` prošel: lokalizace, TypeScript, formátování, konvence, rustfmt a workspace Clippy. Po posledních opravách popisků/adresy znovu locales, typecheck, rustfmt, build serveru a Clippy serveru.
- `pnpm build:web` prošel; přetrvává upozornění na velký sdílený chunk.
- `pnpm generate:api`: všechny generované typy a MCP schémata mají shodný obsah před a po regeneraci.

Integrační a UI testy používají izolované PostgreSQL databáze. E-mailové regrese
používají loopback SMTP capture, nikoli skutečné příjemce. První běh nové PDF
regrese vyžadoval zvýšení limitu testovacího HTTP readeru kvůli vloženému fontu;
finální integrační běh prošel. Limity produkčního endpointu se tím neměnily.

## Vizuální a lokální ověření

Screenshoty `docs/design/advance-{editor,detail,list}-{cs,en}-{desktop,mobile}.png`
a PDF `output/advance-{cs,en}-desktop.pdf` jsou lokální ignorované artefakty.
Zkontrolován český detail na desktopu, mobilní seznam a render PDF přes Poppler.
Opravené přebytečné čárky v prázdné adrese a popisky převzaté z nabídek/šablon.
PDF má samostatné číslo ZF a označení, že nejde o daňový doklad.
Fyzický tisk ani nativní desktopová instalace nebyly ověřovány.

Vývojový server byl aktualizovaný, migrace 018 aplikovaná a `/api/ready`
vrací `ready`. Do uživatelské agendy testy nevkládají doklady.

## Zbývající rozsah

Daňové doklady k přijaté platbě (P13-02), vyúčtování (P13-03), e-mail/přílohy
záloh, vlastní formát číselné řady a pokročilé filtry nejsou součástí této dodávky.
Vyúčtování je v UI neaktivní a konverze zálohy na běžnou fakturu je na serveru
odmítnutá; nabídky zachovávají vlastní funkční převod do návrhu faktury.
