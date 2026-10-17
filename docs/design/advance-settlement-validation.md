# ADVANCE-SETTLEMENT-01 / P13-03 — ověření 12. 10. 2026

[Specifikace](../specs/advance-settlement.md), reference E-034. Doložený je postup
z veřejné nápovědy, nikoli nová relace v přihlášené agendě iDokladu. Numerická ani
přesná vizuální shoda není potvrzená. Testovací data jsou syntetická.

## Doména a databáze

- Doménový test `advance_settlement` ověřuje přesný odpočet tří uložených
  haléřových DDPP, rozdíl oproti novému přepočtu daně, nulový doplatek a odmítnutí
  záporného doplatku. Očekávané částky jsou explicitní.
- `DATABASE_URL=… pnpm test:advance-settlements`: 6 scénářů prošlo. Více záloh,
  přesné snapshoty DDPP, souběžná rezervace a replay, blokace nových plateb/storna/DDPP,
  uvolnění návrhu, nezměnitelnost po vystavení, cizí profil a viewer, REST/MCP/CSRF,
  upgrade z 019 s původní fakturou a zaplacenou zálohou, neplátce a zaokrouhlení.
  Úhrada doplatku 121 Kč dává běžný příjem 121 Kč, původní záloha se nezapočte podruhé.
- Regrese: advance (3), invoice (7), invoice_delivery (5), payment (6), sales (4),
  tax_receipt (5) — celkem 30 různých integračních scénářů prošlo.
- Upgrade test invoice_delivery po rozšíření view vyžaduje vyprázdnění připravených
  SQL dotazů. Produkční postup zastaví původní proces a nový server začíná s novým
  poolem. Staré PDF zůstává stejné.

## Statické kontroly a build

`pnpm check` prošel: oba katalogy, TypeScript, formát, tokenové konvence,
rustfmt a Clippy celého workspace. `pnpm test` prošel (16 testů; 78 integračních
scénářů je ve výchozím běhu ignorovaných, relevantní DB scénáře jsou spuštěné
výše samostatně). `pnpm build:web` i server build prošly; sdílený webový chunk
nadále vyvolává upozornění na velikost. Regenerace API nezměnila žádný generovaný
soubor. `git diff --check` bez chyb.

## UI a PDF

`pnpm exec playwright test tests/e2e/sales.spec.ts --workers=1`: finální samostatný běh prošel ve všech 4 scénářích (CS/EN × desktop/mobil). Sales E2E ověřuje: zaplacení zbytku zálohy, výběr,
převod do návrhu, zobrazení uloženého DDPP, změnu ceny plnění, vystavení,
doplatek 121 Kč, PDF a návrat na vyúčtování ze zálohy. Součástí zůstává regrese
nabídek, doručení přes loopback SMTP, sdílení a daňových dokladů.

Screenshoty jsou lokální ignorované `docs/design/settlement-{editor,detail}-{cs,en}-{desktop,mobile}.png`;
PDF `output/settlement-{cs,en}-{desktop,mobile}.pdf`. Vizuální kontrola přes Poppler
odhalila chybějící znak Unicode minus ve vloženém fontu. Renderer používá podporovaný
znak `-`, opravené PDF má čitelné záporné odpočty. UI používá společné monochromatické
komponenty, peněžní hodnoty se nezalamují. Screenshoty se pořizují po přepočtu a po
návratu scrollu na začátek, aby nevznikaly artefakty fixní navigace.

První souběžný běh s kompilací/testy překročil výchozí 5sekundová čekání na PDF
náhled, doručení a synchronní vystavení DDPP. Odpovídající čekání jsou bounded 20 sekund,
E2E běží s jedním workerem. Nešlo o změnu doménových očekávání. Při doplnění testu
úhrady byla opravená fixture data platby před datem finální faktury a očekávaný
HTTP kód existující CSRF ochrany (401).

Fyzický tisk ani Tauri tento krok neověřoval. Testy používají jednorázové DB;
žádné zprávy skutečným zákazníkům. Lokální API běží s migrací 020 a vrací ready.
Dobropisy, částečné vyúčtování, vrácení přeplatků a historické sazby zůstávají mimo
rozsah. Podrobné rozdíly proti referenci uvádí specifikace.
