# P19-01d — Bilance, TOP 3 a prodlení

Podklad E-061: veřejná nápověda iDokladu. Rozsah CZK, bez tvrzení numerické
shody nebo podpory měnových přepočtů. Společný read-only příkaz dashboard_report
používá REST GET /api/companies/{company}/reports/dashboard a MCP.

## Výběr a konzistence

Datum today pochází z časového pásma profilu. Parametr year (1900 až současný
rok, výchozí současný) vybírá rok bilance. UI nabízí letošní a loňský rok.
all_years přepíná pouze prodlení mezi vybraným rokem a celým obdobím.
Žebříčky jsou vždy za celé období do dneška. Všechny dotazy sdílejí jeden
repeatable-read snapshot PostgreSQL a autorizaci člena profilu, včetně viewer.
Není potřeba nová migrace: čteme existující doklady, úhrady a jejich opravy.

## Bilance

Zdroj a rozhodná data jsou shodné s period-report.md: vystavené faktury,
vystavené dobropisy, nestornované prodejky a přijaté faktury/účtenky.
Zálohy ani DDPP se samostatně nepřičítají. Vyúčtování používá původní plnění
včetně uložených odpočtů. Budoucí doklady za dnešní datum nevstupují.
Vracíme oddělený gross/net, settled a outstanding.

Gross/net jsou uložené částky plnění. Vyrovnání a zůstatek jsou vždy s DPH;
UI tuto skutečnost označuje i při volbě hlavní hodnoty bez DPH.
Zůstatek běžné faktury bere invoice_settlement (platby bez storen, zápočty).
Nákup bere max(total-paid,0), účtenky a prodejky jsou plně vyrovnané.
Nevyrovnaný dobropis je záporný zůstatek: jeho záporná částka plus aktivní
zápočty a vratky bez stornovaných událostí. Settled = gross − outstanding,
není to peněžní tok. Přeplatek běžné faktury nezvyšuje vyrovnanou hodnotu.
Záporné saldo představuje převahu nevyrovnaných vratek. Toto zacházení
s částečným dobropisem je naše explicitní pravidlo, ne pozorovaný výpočet
iDokladu. Související originální faktury a dobropisy mohou spadat do jiných roků.

## Prodlení

Samostatně vydané běžné faktury a přijaté faktury s kladným zůstatkem.
Bez záloh, dobropisů a bez vzájemného snížení různých dokladů.
Dnešní splatnost není prodlení. Vlastní intervaly:
do splatnosti včetně dneška, 1–30, 31–60, 61–90, více než 90 dní.
Každý řádek obsahuje součet a počet dokladů nad celým výběrem, nikoli stránkou.
Prázdné intervaly jsou nulové. Vybraný rok používá stejné rozhodné datum
jako bilance; all_years zahrnuje veškerou dostupnou historii od roku 1900.

## Největší odběratelé a dlužníci

Běžné faktury a nearchivované zálohové faktury do dneška, včetně DPH.
Odběratelé podle součtu fakturovaných částek, dlužníci podle aktuálního
kladného zůstatku po splatnosti. Zálohy používají advance_paid.
Nejde o obrat, protože žebříček může obsahovat zálohu i její vyúčtování.
Vracíme TOP 3, shody řadíme stabilně podle identity. Seskupení podle
uloženého contact_id, jméno z posledního dokladu; bez vazby na kontakt
zůstane každý doklad samostatný (neslučujeme osoby jen podle jména).
Zobrazené jméno je historický snapshot, nemusí být dnešní název adresáře.

## Ověření

period_report.rs: přesné nezávisle očekávané částky, storno úhrady, částečný
zápočet dobropisu a jeho nevyrovnaná část, nákup a účtenka, záloha v TOP 3,
hraniční dny 0/1/30/31/60/61/90/91, izolace profilů, viewer, chybný rok,
REST/MCP shoda. Dashboard E2E používá izolovanou DB a CS/EN desktop/mobile.

Ověření 17. 10. 2026: `pnpm test:period-report` prošlo (6 testů),
`playwright test tests/e2e/dashboard.spec.ts --workers=1` prošlo (4 scénáře:
CS/EN, desktop/mobil, včetně přístupnosti a oddělení profilů).
`pnpm check` a `pnpm build:web` prošly. Referenční screenshoty našeho UI jsou
v `docs/design/dashboard-{cs,en}-{desktop,mobile}.png`. Lokální API bylo
restartováno s novým sestavením; `/api/ready` potvrzuje připravenost.
