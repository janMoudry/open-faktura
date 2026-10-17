# P19-01a — Prodej a nákup za období

Podklad E-054 (veřejná nápověda), 16. 10. 2026. Nejde o potvrzenou
numerickou shodu s iDokladem ani daňové přiznání. Přehled sleduje hodnotu
dokladů s/bez DPH, nikoli peněžní tok či zisk. Pro daňové faktury a dobropisy
používá DUZP, pro nedaňové datum vystavení; pro nákupní faktury datum přijetí
a pro účtenky/prodejky datum vystavení. Výpočet vyúčtování a technické limity
jsou vlastní rozhodnutí; kumulace a srovnání roků jsou popsány níže.

Prodej zahrnuje vystavené faktury, vystavené dobropisy se zápornou částkou
a nestornované prodejky. U vyúčtování vrací původní plnění: uložená splatná
částka + uložený odpočet záloh − zaokrouhlení vyúčtování. Zálohy, DDPP,
platby a souhrnné zaúčtování prodejek se znovu nepřičítají. Nákup zahrnuje
nestornované přijaté faktury a účtenky. Částky různých měn se nesčítají;
tato první verze zpracovává pouze CZK.

Rozsah nejvýše 367 dní, měsíční součty i celkové součty nad všemi doklady,
zdroje po 25 řádcích. Prázdné měsíce mají nulu. Jediný snapshot PostgreSQL
pro graf, součty a stránku zdrojů. Profilová práva platí i přes MCP.
Akceptace: přesné součty, záporné dobropisy, vyloučené návrhy/storna,
vyúčtování bez dvojího odpočtu, všechny stránky a izolace profilů.

Ověřeno: `crates/server/tests/period_report.rs` — syntetické uložené částky bez závislosti
na kalkulátoru, odlišná data vystavení/DUZP/přijetí, všechny stránky, profilová
izolace a REST/MCP. CS/EN desktop/mobile scénář je v `catalog.spec.ts`.
Pole zdroje `reported_on` znamená rozhodné datum pro přehled, nikoli vždy
datum vystavení. Bez nové migrace.

Volba bez DPH používá uložené základy. U vyúčtování přičte zpět přesné
uložené základy DDPP, u nákupu sečte uložené řádky. Žádný výpočet zpětným
dělením cenou/sazbou. Zaokrouhlení celkové platby není součástí čistého základu.

## P19-01b — Kumulace a meziroční srovnání

Podklad E-054 dokládá existenci těchto zobrazení, nikoli pravidla přestupných
roků. Vlastní rozhodnutí: předchozí období posune obě hranice o kalendářní
rok zpět; 29. únor se změní na 28. únor. Povolujeme nejvýše 367 dní,
aby byl možný přechod na zdroje stejně dlouhého období přes přestupný rok.
Porovnání u roku 1900 nebo s posunutým obdobím nad 367 dní se odmítne. Obě období používají jeden DB snapshot.
Kumulace běží od zadaného začátku období (nikoli od vzniku profilu), každá
měsíční hodnota je přesný nápočet v Decimal. Celkové částky se nemění.
Rozdíl je aktuální minus předchozí; záporný rozdíl není chyba. UI zobrazí
prodej/nákup odděleně pro přehledné dvě série a nabízí přechod na zdroje
předchozího období. Backend vrací přesné řetězce; JS Number slouží jen grafu.

Ověření P19-01b (16. 10. 2026): tři testy `test:period-report` včetně
REST/MCP s aktivním srovnáním, záporného rozdílu, částečného období a
přestupného dne. Čtyři CS/EN desktop/mobile scénáře `catalog.spec.ts`
včetně přístupnosti a přechodu na loňské zdroje. `pnpm check` a web build
prošly. Technické scénáře používají syntetická data, nikoli účet iDokladu.
