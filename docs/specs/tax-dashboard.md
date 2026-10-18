# P19-01e — Evidence DPH a orientační obrat

Podklad E-062, veřejná nápověda iDokladu; nikoli numerická shoda s aplikací.
Read-only REST/MCP `tax_report`, profilová autorizace, repeatable-read snapshot.
Bez nové migrace: uložené součty a data stávajících dokladů jsou zdrojem.

## Pravidla vlastní implementace

Volba roku 1900 až současnost, měsíce/kvartály. Data pouze do dneška podle
časové zóny profilu. Prázdná období mají nulu. Částky CZK v desetinné aritmetice.
DPH na výstupu: vystavené běžné faktury podle DUZP, vystavené dobropisy
podle DUZP se zápornou daní, DDPP podle data vystavení, nestornované prodejky
podle data vystavení. Používají se uložené amounts.vat; u vyúčtování již
snížené o uložené odpočty DDPP. Nevracet zpět odečtenou DPH: DDPP je samostatný
zdroj v období vystavení. Zálohové faktury ani platby se samostatně nepočítají.
Souhrnné zaúčtování prodejek nevytváří další zdroj.

DPH na vstupu: součet uložených lines.vat nestornovaných přijatých faktur a
účtenek. Rozhodné datum DUZP, pokud není vyplněné, datum přijetí u faktury
nebo vystavení u účtenky. Fallback je vlastní pravidlo. Jde o evidovanou daň,
nikoli uznaný nárok na odpočet. Rozdíl = výstup − vstup, může být záporný.
Doklady bez uložené daně nebo data pro zařazení se nezahrnují.

Obrat: orientační čistá hodnota prodejních dokladů od ledna do dneška
vybraného roku. Sdílený zdroj period-report, bez DPH, vrací původní plnění
vyúčtování (uložené základy odpočtů přičtené zpět), bez záloh/DDPP.
Nejde o zákonný obrat: zatím nerozlišujeme místo plnění, osvobození a vyloučená
plnění. Proto nezobrazujeme automatický právní závěr ani procenta zákonného
limitu. Číselné limity Finanční správy jsou podkladem pro další daňové členění,
nikoli důvodem srovnávat neúplný součet se zákonným obratem.

UI zobrazuje obrat všem profilům; DPH pouze aktuálním plátcům. API historická
data nefiltruje podle dnešního režimu. Identifikovaná osoba není podporována
modelem profilu. Cizí měny, daňové
klasifikace/nárok na odpočet a daňové formuláře zbývají. V UI je rozsah uveden.
Zdrojové doklady mají samostatnou stránkovanou tabulku (25 řádků) s volbou
DPH/obrat. Součty nejsou omezené stránkou.

Akceptace: DDPP a vyúčtování v různých obdobích bez dvojí daně; dobropis;
storna a návrhy vynechané; více stránek; DUZP vs vystavení; CS/EN; profilová
izolace a REST/MCP shoda. Syntetická data nejsou referenční pozorování.

Při ověřování přepínání filtrů se projevilo oddělené BEGIN / SET TRANSACTION
na vraceném připojení. Přehledy tax/dashboard/period nyní zahajují izolaci
jedním BEGIN ISOLATION LEVEL REPEATABLE READ a úspěšné čtení explicitně
ukončují commitem. Mobilní tabulka DPH podporuje posun klávesnicí.

Ověření 17. 10. 2026: `pnpm test:period-report` 7/7 (izolovaná PostgreSQL,
včetně REST/MCP); `playwright test tests/e2e/dashboard.spec.ts --workers=1`
4/4 (CS/EN desktop/mobil, přístupnost, změna členění a zdrojů, oddělení
profilů). `pnpm check`, `pnpm build:web` a server build prošly. Lokální API
aktualizováno, `/api/ready` vrací ready. Screenshoty: `docs/design/dashboard-*`
a `tax-dashboard-*` (výřez sekce může obsahovat překrytí pevnou navigací).
Po vizuální opravě zalamování období/čísel dokladů na mobilu znovu prošly
oba mobilní scénáře CS/EN a webový build.

## P19-01f — Samostatná pokladní plnění

Vlastní pravidlo nad existujícím snapshotem P16-06 (E-046); nejde o doloženou
numerickou shodu s iDokladem. `cash_movement.tax` obsahuje kladné uložené
amounts.net/vat, směr určuje znaménko movement.amount. Kladný pohyb vstupuje
do výstupní DPH a čistého orientačního obratu, záporný pouze do vstupní DPH.
Výdaj není záporným prodejem. Datum obou přehledů je tax.tax_on (DUZP), nikoli
booked_on. Hodnoty se nepřepočítávají z ceny, sazby ani dnešního profilu.
Zahrnují se i nulové sazby a historické doklady archivované pokladny.

Pouze CZK účty, tax != null, NOT cancelled, NOT automatic a bez aktivního
cash_match. Běžné ruční/automatické platby, převody, zálohy a souhrnné
zaúčtování prodejek bez vlastního tax snapshotu se podruhé nepřičítají.
Storno plnění odebere z obou přehledů, původní snapshot zůstává nedotčený.
API vrací kind=cash a ID pohybu, UI odkazuje na jeho detail. Součty a zdroje
používají shodný výběr v jedné transakci, platí profilová oprávnění.
Bez nové migrace; obecný period-report a finanční bilance se tímto nemění.
Omezení zákonného obratu a nároku na odpočet zůstávají.

Ověření P19-01f (17. 10. 2026): `pnpm test:period-report` 8/8,
CS/EN desktop/mobile `dashboard.spec.ts` 4/4 včetně přístupnosti a odkazů
na pokladní zdroj. Syntetické snapshoty ověřují příjem/výdaj, sazbu 0,
haléřové hodnoty, DUZP oproti booked_on, archivaci, změnu plátcovství,
storno a oddělení profilu; REST/MCP shoda obsahuje pokladní plnění.
`pnpm check`, server build a web build prošly. Lokální API aktualizováno,
readiness ověřena; screenshot mobilní české verze vizuálně zkontrolován.
